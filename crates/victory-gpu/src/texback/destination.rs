//! Identity of guest memory promised to an outstanding texture readback.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

struct Allocation {
    at: usize,
    end: usize,
    valid: Arc<AtomicBool>,
}

pub(super) struct Ticket {
    owner: usize,
    pub(super) at: usize,
    pub(super) size: usize,
    valid: Arc<AtomicBool>,
}

pub(super) struct Registry(Mutex<HashMap<usize, Allocation>>);

impl Registry {
    pub(super) fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }

    /// A new resource may replace a water allocation even if the new one
    /// is not itself a copyback candidate.
    pub(super) fn installed(&self, owner: usize, at: usize, size: Option<usize>) {
        let mut live = self.0.lock().unwrap_or_else(|e| e.into_inner());
        live.retain(|&key, old| {
            let keep = key != owner && old.at != at;
            if !keep {
                old.valid.store(false, Ordering::Release);
            }
            keep
        });
        if let Some(end) = size
            .and_then(|n| at.checked_add(n))
            .filter(|&end| at != 0 && at < end)
        {
            live.insert(
                owner,
                Allocation {
                    at,
                    end,
                    valid: Arc::new(AtomicBool::new(true)),
                },
            );
        }
    }

    pub(super) fn capture(&self, owner: usize, at: usize, size: usize) -> Option<Ticket> {
        let live = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let allocation = live.get(&owner)?;
        (allocation.at == at && allocation.end - at == size).then(|| Ticket {
            owner,
            at,
            size,
            valid: Arc::clone(&allocation.valid),
        })
    }

    pub(super) fn released(&self, owner: usize) {
        let mut live = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(old) = live.remove(&owner) {
            old.valid.store(false, Ordering::Release);
        }
    }

    pub(super) fn remapped(&self, at: usize, size: usize) {
        if size == 0 {
            return;
        }
        let end = at.saturating_add(size);
        let mut live = self.0.lock().unwrap_or_else(|e| e.into_inner());
        for allocation in live.values_mut() {
            if allocation.at < end && at < allocation.end {
                allocation.valid.store(false, Ordering::Release);
                // A later shader write can register against the new mapping.
                allocation.valid = Arc::new(AtomicBool::new(true));
            }
        }
    }

    /// A release or replacement must wait until the checked guest write ends.
    pub(super) fn landing(&self, ticket: &Ticket) -> Option<impl Drop + '_> {
        let live = self.0.lock().unwrap_or_else(|e| e.into_inner());
        ticket.is_current().then_some(live)
    }
}

impl Ticket {
    pub(super) fn is_current(&self) -> bool {
        self.valid.load(Ordering::Acquire)
    }

    pub(super) fn matches(&self, owner: usize) -> bool {
        self.owner == owner && self.is_current()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_destination_remap_only_invalidates_intersecting_generations() {
        let live = Registry::new();
        live.installed(1, 0x1000, Some(0x1000));
        live.installed(2, 0x2000, Some(0x1000));
        let old = live.capture(1, 0x1000, 0x1000).unwrap();
        let next = live.capture(2, 0x2000, 0x1000).unwrap();
        live.remapped(0x1800, 0);
        live.remapped(0x0800, 0x0800);
        let landing = live.landing(&old).unwrap();
        assert!(live.0.try_lock().is_err());
        drop(landing);
        live.remapped(0x1800, 0x0800);
        assert!(live.landing(&old).is_none());
        assert!(live.landing(&next).is_some());
        assert!(live.capture(1, 0x1000, 0x1000).unwrap().matches(1));
    }

    #[test]
    fn water_destination_release_or_replacement_never_revives_an_old_ticket() {
        let live = Registry::new();
        live.installed(7, 0x1000, Some(256));
        let old = live.capture(7, 0x1000, 256).unwrap();
        live.released(7);
        assert!(live.capture(7, 0x1000, 256).is_none());
        live.installed(7, 0x1000, Some(256));
        let reused = live.capture(7, 0x1000, 256).unwrap();
        assert!(!old.matches(7) && reused.matches(7));
        live.installed(8, 0x1000, None);
        assert!(live.landing(&reused).is_none());
        assert!(live.capture(7, 0x1000, 256).is_none());
        live.installed(9, usize::MAX - 1, Some(4));
        assert!(live.capture(9, usize::MAX - 1, 4).is_none());
    }
}
