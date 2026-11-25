//! Select one recording frame from a running diagnostics build.

use std::collections::BTreeMap;
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::resource::HostTexture;

static FRAME: AtomicU64 = AtomicU64::new(0);
static REQUEST: Mutex<Request> = Mutex::new(Request {
    frame: 0,
    targets: Vec::new(),
    seen: BTreeMap::new(),
});

struct Request {
    frame: u64,
    targets: Vec<usize>,
    seen: BTreeMap<usize, Target>,
}

struct Target {
    width: u64,
    height: u32,
    format: i32,
    samples: u32,
    copy: Option<Arc<HostTexture>>,
}

pub(super) fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| super::folder().is_some() && std::env::var("VICTORY_CAPTURE_TRIGGER").as_deref() == Ok("1"))
}

pub(super) fn listed(frame: u64) -> bool {
    frame != 0 && FRAME.load(Ordering::Acquire) == frame
}

fn parse(text: &str) -> Option<Vec<usize>> {
    let mut words = text.split_whitespace();
    match words.next()? {
        "ops" if words.next().is_none() => Some(Vec::new()),
        "targets" => {
            let mut ids = Vec::new();
            for word in words {
                let id = usize::from_str_radix(word.strip_prefix("0x").unwrap_or(word), 16).ok()?;
                if id == 0 || ids.len() == 8 || ids.contains(&id) {
                    return None;
                }
                ids.push(id);
            }
            (!ids.is_empty()).then_some(ids)
        }
        _ => None,
    }
}

fn take_request(dir: &Path) -> io::Result<Option<Vec<usize>>> {
    let active = dir.join("capture-next.active");
    match std::fs::rename(dir.join("capture-next"), &active) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        result => result?,
    }
    let read = (|| {
        let mut contents = String::new();
        std::fs::File::open(&active)?.take(257).read_to_string(&mut contents)?;
        if contents.len() <= 256 && let Some(ids) = parse(&contents) {
            return Ok(Some(ids));
        }
        Err(io::Error::new(io::ErrorKind::InvalidData, "expected ops or targets followed by 1 to 8 distinct hexadecimal resource IDs"))
    })();
    std::fs::remove_file(active)?;
    read
}

/// Called before the guest present counter advances. An adjacent request stays
/// on disk until the already selected frame has finished its own present.
pub(super) fn poll(presenting: u64) {
    if !enabled() || FRAME.load(Ordering::Acquire) >= presenting {
        return;
    }
    let Some(dir) = super::folder() else { return };
    let mut request = REQUEST.lock().unwrap_or_else(|e| e.into_inner());
    if request.frame >= presenting { return; }
    match take_request(dir) {
        Ok(Some(targets)) => {
            let frame = presenting + 1;
            tracing::info!(frame, targets = targets.len(), "one-frame capture armed");
            *request = Request { frame, targets, seen: BTreeMap::new() };
            FRAME.store(frame, Ordering::Release);
        }
        Ok(None) => {}
        Err(error) => tracing::warn!(%error, "could not consume one-frame capture request"),
    }
}

/// Inventory the first observed binding for each guest resource on this frame.
/// Only selected resources enter the existing full-image readback path.
pub(super) fn note(frame: u64, resource: crate::object::This) {
    let mut request = REQUEST.lock().unwrap_or_else(|e| e.into_inner());
    if request.frame != frame { return; }
    let id = resource as usize;
    let selected = request.targets.contains(&id);
    if let std::collections::btree_map::Entry::Vacant(entry) = request.seen.entry(id)
        && let Some(copy) = crate::resource::host_copy(resource)
    {
        // SAFETY: the host copy keeps the texture alive for this query.
        let d = unsafe { copy.texture.GetDesc() };
        entry.insert(Target {
            width: d.Width, height: d.Height, format: d.Format.0, samples: d.SampleDesc.Count,
            copy: selected.then_some(copy),
        });
    }
}

pub(super) fn finish(frame: u64) -> Vec<(usize, Arc<HostTexture>)> {
    if !listed(frame) { return Vec::new(); }
    let Some(dir) = super::folder() else { return Vec::new() };
    let seen = {
        let mut request = REQUEST.lock().unwrap_or_else(|e| e.into_inner());
        if request.frame != frame { return Vec::new(); }
        request.frame = 0;
        std::mem::take(&mut request.seen)
    };
    let write = (|| -> io::Result<()> {
        let mut file = io::BufWriter::new(std::fs::File::create(dir.join(format!("frame-{frame}-targets.tsv")))?);
        writeln!(file, "resource\twidth\theight\tformat\tsamples\tselected")?;
        for (id, target) in &seen {
            writeln!(file, "{id:#x}\t{}\t{}\t{}\t{}\t{}", target.width, target.height,
                target.format, target.samples, target.copy.is_some())?;
        }
        file.flush()
    })();
    if let Err(error) = write {
        tracing::warn!(frame, %error, "could not save frame target inventory");
    }
    seen.into_iter().filter_map(|(id, target)| target.copy.map(|copy| (id, copy))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_parses_ops_and_bounded_targets() {
        assert_eq!(parse("ops\r\n"), Some(vec![]));
        assert_eq!(parse("targets 0xab cd"), Some(vec![0xab, 0xcd]));
        for bad in ["", "ops 1", "targets", "targets 0", "targets x", "targets 1 01",
                    "targets 1 2 3 4 5 6 7 8 9"] {
            assert_eq!(parse(bad), None);
        }
    }

    #[test]
    fn trigger_consumes_each_request_once_and_recovers_from_bad_input() {
        let dir = std::env::temp_dir().join(format!("victory-frame-trigger-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let marker = dir.join("capture-next");
        assert_eq!(take_request(&dir).unwrap(), None);
        std::fs::write(&marker, " ".repeat(257) + "ops").unwrap();
        assert_eq!(take_request(&dir).unwrap_err().kind(), io::ErrorKind::InvalidData);
        std::fs::write(&marker, "targets 0x42").unwrap();
        assert_eq!(take_request(&dir).unwrap(), Some(vec![0x42]));
        assert_eq!(take_request(&dir).unwrap(), None);
        std::fs::remove_dir(dir).unwrap();
    }
}
