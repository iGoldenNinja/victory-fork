//! Read-only source archives for a separately mounted private package.

use std::ffi::c_void;
use std::fs::{File, OpenOptions};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf, Prefix};
use std::sync::OnceLock;

use windows_sys::Win32::Foundation::GENERIC_READ;
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_FLAG_NO_BUFFERING, FILE_FLAG_OPEN_REPARSE_POINT, FILE_FLAG_OVERLAPPED,
    FILE_FLAG_RANDOM_ACCESS, FILE_FLAG_SEQUENTIAL_SCAN, FILE_READ_ATTRIBUTES, FILE_SHARE_READ,
    OPEN_EXISTING,
};

use super::{Handle, path_slice};

pub(super) fn configured_root() -> Option<&'static Path> {
    static ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();
    ROOT.get_or_init(|| std::env::var_os("VICTORY_ARCHIVE_READ_ROOT").map(PathBuf::from))
        .as_deref()
}

// Keep this list deliberately smaller than the host's ordinary CreateFile interface.
const READ_FLAGS: u32 = FILE_ATTRIBUTE_NORMAL
    | FILE_FLAG_NO_BUFFERING
    | FILE_FLAG_OVERLAPPED
    | FILE_FLAG_RANDOM_ACCESS
    | FILE_FLAG_SEQUENTIAL_SCAN;

pub(super) fn open_request(
    root: &Path,
    path: *const u16,
    access: u32,
    share: u32,
    security: *const c_void,
    disposition: u32,
    flags: u32,
    template: Handle,
) -> Option<File> {
    if path.is_null()
        || access != GENERIC_READ
        || disposition != OPEN_EXISTING
        || share & FILE_SHARE_READ == 0
        || flags & !READ_FLAGS != 0
        || !security.is_null()
        || !template.is_null()
    {
        return None;
    }
    // SAFETY: the CreateFileW guest argument is a NUL-terminated path.
    let name = String::from_utf16(unsafe { path_slice(path) }).ok()?;
    let relative = archive_name(&name)?;
    if !super::drives().iter().any(|(letter, _)| *letter == u16::from(b'G')) {
        return None;
    }
    let file = open_regular_file(root, &relative, flags)?;
    tracing::info!(path = %name, "direct read-only source archive");
    Some(file)
}

fn archive_name(name: &str) -> Option<PathBuf> {
    let name = name.strip_prefix(r"\\?\").unwrap_or(name);
    let [drive, b':', b'\\' | b'/', ..] = name.as_bytes() else {
        return None;
    };
    if !drive.eq_ignore_ascii_case(&b'G') || name.ends_with('.') {
        return None;
    }
    let mut relative = PathBuf::new();
    for part in name[3..].split(['\\', '/']) {
        if part == "." {
            continue;
        }
        if part.is_empty()
            || part.ends_with([' ', '.'])
            || !part.bytes().all(|b| b.is_ascii_alphanumeric() || b" ._-".contains(&b))
        {
            return None;
        }
        // Windows recognizes reserved device names even with a filename extension.
        let stem = part.split('.').next()?.trim_end().to_ascii_uppercase();
        if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || matches!(stem.as_bytes(), [b'C', b'O', b'M', b'1'..=b'9'] | [b'L', b'P', b'T', b'1'..=b'9'])
        {
            return None;
        }
        relative.push(part);
    }
    let ext = relative.extension()?.to_str()?;
    (ext.eq_ignore_ascii_case("rpf") || ext.eq_ignore_ascii_case("ttpf")).then_some(relative)
}

fn open_regular_file(root: &Path, relative: &Path, flags: u32) -> Option<File> {
    // An absolute local drive root, with no device/UNC namespace or parent components.
    let mut parts = root.components();
    if !matches!(parts.next(), Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::Disk(_)))
        || parts.next() != Some(Component::RootDir)
        || !parts.all(|part| matches!(part, Component::Normal(_)))
    {
        return None;
    }
    let path = root.join(relative);
    let parents: Vec<_> = path.parent()?.ancestors().collect();
    let mut guards = Vec::with_capacity(parents.len());
    for parent in parents.into_iter().rev() {
        // Inspect each directory itself, retaining a non-delete/non-write-shared handle
        // so it cannot be substituted or changed into a junction before the file opens.
        let guard = OpenOptions::new()
            .access_mode(FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(parent)
            .ok()?;
        let meta = guard.metadata().ok()?;
        if !meta.is_dir() || meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return None;
        }
        guards.push(guard);
    }
    let file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(flags | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(&path)
        .ok()?;
    let meta = file.metadata().ok()?;
    (meta.is_file() && meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0).then_some(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::files::{CreateFileW, GetFileAttributesW, mount_at};
    use std::io::{Read, Write};
    use std::os::windows::io::FromRawHandle;
    use std::process::Command;
    use windows_sys::Win32::Foundation::GENERIC_WRITE;
    use windows_sys::Win32::Storage::FileSystem::{CREATE_ALWAYS, INVALID_FILE_ATTRIBUTES};

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain([0]).collect()
    }

    fn open(name: &str, access: u32, disposition: u32) -> File {
        let raw = CreateFileW(
            wide(name).as_ptr(), access, FILE_SHARE_READ, std::ptr::null(),
            disposition, 0, std::ptr::null_mut()
        );
        assert_ne!(raw as isize, -1, "open {name}: {}", std::io::Error::last_os_error());
        // SAFETY: successful native CreateFileW handle owned by this caller.
        unsafe { File::from_raw_handle(raw) }
    }

    fn contents(mut f: File) -> Vec<u8> {
        let mut bytes = Vec::new();
        f.read_to_end(&mut bytes).unwrap();
        bytes
    }

    #[test]
    fn guest_paths_are_limited_to_archives_on_g() {
        for bad in [
            r"G:\..\outside.rpf", r"G:\.\..\x.rpf", r"G:\dir\.\..\x.rpf", r"G:\x.rpf:alt", r"F:\x.rpf",
            r"G:x.rpf", r"G:\\x.rpf", r"G:\dir.\x.rpf", r"G:\NUL.rpf", r"\\server\G\x.rpf",
            r"G:\x.dll", r"G:\x.rpf.exe", r"G:\dir\x?.rpf", r"G:\x.rpf\.", r"G:\dir\.\",
            r"G:\.\dir\\x.rpf", r"G://./x.rpf", r"G:/dir/.//x.rpf", r"G:\.",
        ] {
            assert!(archive_name(bad).is_none(), "{bad}");
        }
        assert_eq!(archive_name(r"\\?\g:/pack/Sub 1.tTpF"), Some(PathBuf::from(r"pack\Sub 1.tTpF")));
        assert_eq!(archive_name(r"G:\.\packs\.\tables\menu.ttpf"),
            Some(PathBuf::from(r"packs\tables\menu.ttpf")));
        assert_eq!(archive_name(r"G:/././main.rpf"), Some(PathBuf::from("main.rpf")));
    }

    #[test]
    fn routes_only_source_archive_reads() {
        const PRIVATE: &str = "VICTORY_ARCHIVE_TEST_PRIVATE";
        if let Some(private) = std::env::var_os(PRIVATE) {
            let private = PathBuf::from(private);
            mount_at('G', &private);
            let enabled = configured_root().is_some();
            let expected: &[u8] = if enabled { b"source" } else { b"private" };
            let file = open(r"G:\main.RPF", GENERIC_READ, OPEN_EXISTING);
            if let Some(source) = configured_root() {
                assert_eq!(OpenOptions::new().write(true).open(source.join("main.rpf"))
                    .unwrap_err().raw_os_error(), Some(32), "source open must deny writers");
            }
            assert_eq!(contents(file), expected);
            assert_eq!(contents(open(r"\\?\g:/sub/pack.ttpf", GENERIC_READ, OPEN_EXISTING)), expected);
            assert_eq!(contents(open(r"G:\.\sub\.\pack.ttpf", GENERIC_READ, OPEN_EXISTING)), expected);
            assert_eq!(contents(open(r"G:\sub\.\..\main.rpf", GENERIC_READ, OPEN_EXISTING)), b"private");
            assert_eq!(contents(open(r"G:\sub\\pack.ttpf", GENERIC_READ, OPEN_EXISTING)), b"private");
            assert_eq!(contents(open(r"G:\module.dll", GENERIC_READ, OPEN_EXISTING)), b"private");
            assert_eq!(contents(open(r"G:\missing.rpf", GENERIC_READ, OPEN_EXISTING)), b"private");
            assert_eq!(contents(open(r"G:\link\escape.rpf", GENERIC_READ, OPEN_EXISTING)), b"private");
            assert_eq!(GetFileAttributesW(wide(r"G:\source-only.rpf").as_ptr()), INVALID_FILE_ATTRIBUTES);
            assert_eq!(GetFileAttributesW(wide(r"G:\.\source-only.rpf").as_ptr()), INVALID_FILE_ATTRIBUTES);
            if enabled {
                assert_eq!(contents(open(r"G:\source-only.rpf", GENERIC_READ, OPEN_EXISTING)), b"source");
            }
            open(r"G:\main.rpf", GENERIC_WRITE, OPEN_EXISTING).write_all(b"private").unwrap();
            open(r"G:\created.rpf", GENERIC_WRITE, CREATE_ALWAYS).write_all(b"private").unwrap();
            assert_eq!(std::fs::read(private.join("created.rpf")).unwrap(), b"private");
            return;
        }
        let fixture = tempfile::tempdir().unwrap();
        let source = fixture.path().join("source");
        let private = fixture.path().join("private");
        let outside = fixture.path().join("outside");
        for dir in [&source, &private, &outside] {
            std::fs::create_dir_all(dir.join("sub")).unwrap();
        }
        for (dir, bytes) in [(&source, b"source".as_slice()), (&private, b"private")] {
            for name in ["main.rpf", "module.dll", "sub/pack.ttpf"] {
                std::fs::write(dir.join(name), bytes).unwrap();
            }
        }
        std::fs::create_dir(private.join("link")).unwrap();
        std::fs::write(private.join("link/escape.rpf"), b"private").unwrap();
        std::fs::write(private.join("missing.rpf"), b"private").unwrap();
        std::fs::write(source.join("source-only.rpf"), b"source").unwrap();
        std::fs::write(outside.join("escape.rpf"), b"outside").unwrap();
        assert!(Command::new("cmd.exe").args(["/d", "/c", "mklink", "/J"])
            .arg(source.join("link")).arg(&outside).output().unwrap().status.success());
        for enabled in [false, true] {
            let mut command = Command::new(std::env::current_exe().unwrap());
            command.args(["--exact", "modules::files::direct_archive::tests::routes_only_source_archive_reads",
                "--nocapture"]).env(PRIVATE, &private).env_remove("VICTORY_ARCHIVE_READ_ROOT");
            if enabled {
                command.env("VICTORY_ARCHIVE_READ_ROOT", &source);
            }
            let probe = command.output().unwrap();
            assert!(probe.status.success(), "enabled={enabled}:\n{}\n{}",
                String::from_utf8_lossy(&probe.stdout), String::from_utf8_lossy(&probe.stderr));
        }
        assert_eq!(std::fs::read(source.join("main.rpf")).unwrap(), b"source");
        assert!(!source.join("created.rpf").exists());
        assert_eq!(std::fs::read(outside.join("escape.rpf")).unwrap(), b"outside");
    }
}
