//! With `VICTORY_READ_WAIT_STATS=1`, split the existing WHY_READ wait time
//! by the first blocking read in the submission and its requested byte count.
//! One settle may unblock several reads. Its full time is attributed once.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Kind {
    Copy,
    Fill,
    Texture,
}

type Key = (Kind, usize);
type Counts = BTreeMap<Key, Totals>;

#[derive(Default)]
struct Totals {
    timed_intervals: u64,
    wait_us: u64,
    checked: u64,
    source_changed: u64,
    destination_changed: u64,
}

enum Observation {
    Wait(u64),
    Check {
        source_changed: bool,
        destination_changed: bool,
    },
}

fn observe(counts: &mut Counts, key: Key, observation: Observation) {
    let totals = counts.entry(key).or_default();
    match observation {
        Observation::Wait(us) => {
            totals.timed_intervals += 1;
            totals.wait_us += us;
        }
        Observation::Check {
            source_changed,
            destination_changed,
        } => {
            totals.checked += 1;
            totals.source_changed += u64::from(source_changed);
            totals.destination_changed += u64::from(destination_changed);
        }
    }
}

fn counts() -> Option<&'static Mutex<Counts>> {
    static COUNTS: OnceLock<Option<Mutex<Counts>>> = OnceLock::new();
    COUNTS
        .get_or_init(|| {
            std::env::var_os("VICTORY_READ_WAIT_STATS")
                .is_some_and(|value| value == "1")
                .then(|| Mutex::new(Counts::new()))
        })
        .as_ref()
}

// The submission lock spans the initial classification, the wait, and its
// completion on this thread. Rechecking the predicate on wake never changes
// the attributed blocker. An immediately satisfied predicate records no time.
thread_local! {
    static BLOCKER: Cell<Option<Key>> = const { Cell::new(None) };
}

pub(super) fn first_blocker(kind: Kind, len: usize) {
    if counts().is_some() {
        BLOCKER.set(Some((kind, len)));
    }
}

pub(super) fn finished() {
    if counts().is_some() {
        BLOCKER.set(None);
    }
}

pub(super) fn waited(us: u64) {
    let Some(counts) = counts() else { return };
    if let Some(key) = BLOCKER.get() {
        let mut counts = counts.lock().unwrap_or_else(|e| e.into_inner());
        observe(&mut counts, key, Observation::Wait(us));
    }
}

// Use successful existing before/after comparisons only. They inspect up to
// 1 MiB per side, and may finish in the reporting interval after the wait.
pub(super) fn checked(kind: Kind, len: usize, source_changed: bool, destination_changed: bool) {
    let Some(counts) = counts() else { return };
    let mut counts = counts.lock().unwrap_or_else(|e| e.into_inner());
    observe(
        &mut counts,
        (kind, len),
        Observation::Check {
            source_changed,
            destination_changed,
        },
    );
}

pub(super) fn line() {
    let Some(counts) = counts() else { return };
    let interval = std::mem::take(&mut *counts.lock().unwrap_or_else(|e| e.into_inner()));
    for ((kind, bytes), totals) in interval {
        tracing::info!(
            ?kind,
            bytes,
            timed_intervals = totals.timed_intervals,
            wait_us = totals.wait_us,
            checked = totals.checked,
            source_changed = totals.source_changed,
            destination_changed = totals.destination_changed,
            "title read wait attribution"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_and_comparisons_are_grouped_by_kind_and_requested_length() {
        let mut counts = Counts::new();
        let fill = (Kind::Fill, 480);
        let copy = (Kind::Copy, 480);
        let large_fill = (Kind::Fill, 2048);
        observe(&mut counts, fill, Observation::Wait(30));
        observe(&mut counts, fill, Observation::Wait(70));
        observe(
            &mut counts,
            fill,
            Observation::Check {
                source_changed: false,
                destination_changed: false,
            },
        );
        observe(
            &mut counts,
            copy,
            Observation::Check {
                source_changed: true,
                destination_changed: true,
            },
        );
        observe(&mut counts, large_fill, Observation::Wait(11));

        assert_eq!(counts.len(), 3);
        assert_eq!(
            (counts[&fill].timed_intervals, counts[&fill].wait_us),
            (2, 100)
        );
        assert_eq!(
            (counts[&fill].checked, counts[&fill].source_changed),
            (1, 0)
        );
        assert_eq!((counts[&copy].wait_us, counts[&copy].checked), (0, 1));
        assert_eq!(
            (
                counts[&copy].source_changed,
                counts[&copy].destination_changed
            ),
            (1, 1)
        );
        assert_eq!(
            (counts[&large_fill].wait_us, counts[&large_fill].checked),
            (11, 0)
        );
    }
}
