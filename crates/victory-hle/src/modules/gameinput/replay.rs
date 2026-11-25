//! Diagnostics-only local input for reproducible menu and movement trials.
use super::{KeyState, PadState, fallback, lock};
use std::fs::File;
use std::io::Read;
use std::os::windows::fs::MetadataExt;
use std::path::{Component, PathBuf, Prefix};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};
use windows_sys::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_REPARSE_POINT, GetDriveTypeW};

const INTERVAL: Duration = Duration::from_millis(40);
const MAX_LINE: usize = 64;
static REPLAY: LazyLock<Option<Arc<Mutex<Holds>>>> = LazyLock::new(start);

#[derive(Default)]
struct Holds(Vec<(u8, Duration, Option<Instant>)>);

impl Holds {
    fn queue(&mut self, key: u8, duration: Duration) {
        if let Some(item) = self.0.iter_mut().find(|item| item.0 == key) {
            *item = (key, duration, None);
        } else {
            self.0.push((key, duration, None));
        }
    }

    fn sample(&mut self, now: Instant) -> Vec<u8> {
        // Start durations at the first input poll, tolerating a late scheduler.
        self.0.retain_mut(|(_, duration, until)| now < *until.get_or_insert(now + *duration));
        self.0.iter().map(|item| item.0).collect()
    }
}

pub(super) fn keys() -> Vec<u8> {
    REPLAY.as_ref().map_or_else(Vec::new, |holds| lock(holds).sample(Instant::now()))
}

pub(super) fn apply_to_pad(pad: &mut PadState, keys: &[u8]) {
    if keys.is_empty() { return; }
    let keys: Vec<_> = keys.iter().map(|&virtual_key| KeyState { virtual_key, ..Default::default() }).collect();
    let replay = fallback(&keys);
    pad.buttons |= replay.buttons;
    pad.left_trigger = pad.left_trigger.max(replay.left_trigger);
    pad.right_trigger = pad.right_trigger.max(replay.right_trigger);
    for (axis, value) in [(&mut pad.left_x, replay.left_x), (&mut pad.left_y, replay.left_y),
        (&mut pad.right_x, replay.right_x), (&mut pad.right_y, replay.right_y)] {
        if value != 0.0 { *axis = value; }
    }
}

fn event(line: &[u8]) -> Option<(u8, Duration)> {
    let mut words = std::str::from_utf8(line).ok()?.trim_start_matches('\u{feff}').split_ascii_whitespace();
    let name = words.next()?.to_ascii_uppercase();
    let ms: u64 = words.next()?.parse().ok()?;
    if !(25..=60_000).contains(&ms) || words.next().is_some() { return None; }
    let key = match name.as_str() {
        "ENTER" | "PADA" => 0x0D,
        "BACK" | "PADB" => 0x08,
        "PADX" => b'X', "PADY" => b'Y',
        "ESC" | "MENU" => 0x1B, "TAB" | "VIEW" => 0x09,
        "UP" => 0x26, "DOWN" => 0x28, "LEFT" => 0x25, "RIGHT" => 0x27,
        "LB" => b'Q', "RB" => b'E', "LT" => b'Z', "RT" => b'C',
        "LS" => b'F', "RS" => b'H',
        "LOOKUP" => b'I', "LOOKDOWN" => b'K', "LOOKLEFT" => b'J', "LOOKRIGHT" => b'L',
        s if s.len() == 1 && s.as_bytes()[0].is_ascii_alphanumeric() => s.as_bytes()[0],
        _ => return None,
    };
    Some((key, Duration::from_millis(ms)))
}

#[derive(Default)]
struct Lines {
    partial: Vec<u8>,
    discard: bool,
}

impl Lines {
    fn feed(&mut self, chunk: &[u8], mut emit: impl FnMut(u8, Duration)) {
        for &b in chunk {
            if b == b'\n' {
                if !self.discard && let Some((key, duration)) = event(&self.partial) {
                    emit(key, duration);
                }
                self.partial.clear();
                self.discard = false;
            } else if self.partial.len() < MAX_LINE {
                self.partial.push(b);
            } else {
                self.discard = true;
            }
        }
    }
}

fn start() -> Option<Arc<Mutex<Holds>>> {
    let path = PathBuf::from(std::env::var_os("VICTORY_INPUT_REPLAY")?);
    let mut components = path.components();
    let drive = match components.next() {
        Some(Component::Prefix(p)) => match p.kind() { Prefix::Disk(drive) => drive, _ => return None },
        _ => return None,
    };
    let volume = [u16::from(drive), b':' as u16, b'\\' as u16, 0];
    // GetDriveTypeW returns 2 for removable media, 3 for a local disk, 6 for a RAM disk.
    let local = matches!(unsafe { GetDriveTypeW(volume.as_ptr()) }, 2 | 3 | 6);
    if !local || components.next() != Some(Component::RootDir)
        || !components.all(|p| matches!(p, Component::Normal(_))) {
        tracing::warn!("input replay needs a precreated local file");
        return None;
    }
    let parents: Vec<_> = path.ancestors().collect();
    for part in parents.into_iter().rev() {
        let meta = std::fs::symlink_metadata(part).ok()?;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
            || (part == path && !meta.is_file()) { return None; }
    }
    let mut file = File::open(path).inspect_err(|e| tracing::warn!(%e, "input replay open failed")).ok()?;
    let holds = Arc::new(Mutex::new(Holds::default()));
    let output = Arc::clone(&holds);
    std::thread::Builder::new().name("victory-input-replay".into()).spawn(move || {
        let mut parser = Lines::default();
        let mut buf = [0u8; 512];
        loop {
            match file.read(&mut buf) {
                Ok(n) => parser.feed(&buf[..n], |key, duration| {
                    lock(&output).queue(key, duration);
                    tracing::info!(key, ms = duration.as_millis() as u64, "live benchmark input queued");
                }),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => { tracing::warn!(%e, "input replay stopped"); return; }
            }
            std::thread::sleep(INTERVAL);
        }
    }).inspect_err(|e| tracing::warn!(%e, "input replay thread unavailable")).ok()?;
    tracing::info!("local benchmark input replay ready");
    Some(holds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn appended_file_event_reaches_a_poll_and_expires() {
        const CHILD: &str = "VICTORY_TEST_INPUT_REPLAY_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("commands.txt");
            File::create(&path).unwrap();
            let run = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "modules::gameinput::replay::tests::appended_file_event_reaches_a_poll_and_expires",
                    "--nocapture"]).env(CHILD, "1").env("VICTORY_INPUT_REPLAY", &path).output().unwrap();
            assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
            return;
        }
        assert!(keys().is_empty());
        let mut out = std::fs::OpenOptions::new().append(true)
            .open(std::env::var_os("VICTORY_INPUT_REPLAY").unwrap()).unwrap();
        out.write_all(b"Look").unwrap();
        std::thread::sleep(INTERVAL * 2);
        assert!(keys().is_empty());
        out.write_all(b"Right 80\r\n").unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while !keys().contains(&b'L') {
            assert!(Instant::now() < deadline, "appended event was not observed");
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(Duration::from_millis(100));
        assert!(keys().is_empty());
    }

    #[test]
    fn partial_lines_limits_and_recovery() {
        let mut parser = Lines::default();
        let mut seen = Vec::new();
        parser.feed(b"Pad", |k,d| seen.push((k,d)));
        assert!(seen.is_empty());
        parser.feed(b"A 90\r\nW 1250\ninvalid 50\nX 24\nY 60001\nA 500 extra\n", |k,d| seen.push((k,d)));
        parser.feed(&[b'L'; MAX_LINE + 50], |k,d| seen.push((k,d)));
        parser.feed(b"\nLookRight 50\n", |k,d| seen.push((k,d)));
        assert_eq!(seen, [(0x0D,Duration::from_millis(90)),(b'W',Duration::from_millis(1250)),
            (b'L',Duration::from_millis(50))]);
    }

    #[test]
    fn key_intervals_and_both_sticks() {
        let mut holds = Holds::default();
        let t = Instant::now();
        for (key,ms) in [(b'W',200),(b'L',75),(0x0D,50)] { holds.queue(key, Duration::from_millis(ms)); }
        let mut pad = PadState { left_x: -0.5, ..Default::default() };
        apply_to_pad(&mut pad, &holds.sample(t));
        assert_eq!((pad.buttons,pad.left_y,pad.right_x,pad.left_x), (super::super::A,1.0,1.0,-0.5));
        assert_eq!(holds.sample(t + Duration::from_millis(75)), [b'W']);
        holds.queue(b'W', Duration::from_millis(40));
        assert_eq!(holds.sample(t + Duration::from_millis(300)),[b'W']);
        assert!(holds.sample(t + Duration::from_millis(340)).is_empty());
    }
}
