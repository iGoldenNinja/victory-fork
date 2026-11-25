//! Match the temporal resolve's table SRVs to earlier completed direct dispatches
//! with a table UAV for the same host texture. This is a binding observation.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock, Weak, atomic::{AtomicU64, Ordering}},
};

use victory_shader::{ResourceType, RootType, UsedResource};

use crate::resource::HostTexture;

pub(crate) fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_PROBE_LIGHT_INPUTS").is_some_and(|v| v == "1"))
}

pub(crate) fn sampled_frame() -> Option<u64> {
    if !enabled() {
        return None;
    }
    let frame = crate::frame::current();
    (frame != 0 && frame.is_multiple_of(32)).then_some(frame)
}

#[derive(Clone, Copy)]
struct Slot {
    root: u8,
    dword: u32,
}

struct Binding {
    slot: Slot,
    texture: Option<Weak<HostTexture>>,
    untracked_reason: Option<&'static str>,
    // Dimensions at recording time: a title can release a texture before retirement.
    shape: Option<(i32, u64, u32)>,
}

pub(crate) struct Dispatch {
    frame: u64,
    hash: u64,
    ordinal: Option<u64>,
    reads: Vec<Binding>,
    writes: Vec<Binding>,
}

pub(crate) struct Sample {
    pub(crate) frame: u64,
    pub(crate) ordinal: u64,
    pub(crate) expected_output: Option<Weak<HostTexture>>,
}

#[derive(Default)]
struct Selection {
    latest_frame: AtomicU64,
    ordinal: AtomicU64,
}

impl Selection {
    fn claim(&self, frame: u64) -> Option<u64> {
        if frame == 0 || !frame.is_multiple_of(32)
            || self.latest_frame.fetch_max(frame, Ordering::Relaxed) >= frame
        {
            return None;
        }
        Some(self.ordinal.fetch_add(1, Ordering::Relaxed) + 1)
    }
}

impl Dispatch {
    /// Called only after recording a direct dispatch with nonzero group counts.
    pub(crate) fn select(&mut self) -> Option<Sample> {
        static SELECTION: OnceLock<Selection> = OnceLock::new();
        if self.reads.is_empty() {
            return None;
        }
        self.ordinal = SELECTION.get_or_init(Selection::default).claim(self.frame);
        let Some(ordinal) = self.ordinal else {
            self.reads.clear();
            return None;
        };
        let expected_output = self.writes.iter()
            .find(|binding| (binding.slot.root, binding.slot.dword) == (1, 176))
            .and_then(|binding| binding.texture.clone());
        Some(Sample { frame: self.frame, ordinal, expected_output })
    }
}

pub(crate) fn bindings(
    frame: u64,
    hash: &str,
    used: &[UsedResource],
    mut observed: impl FnMut(&UsedResource) -> Result<Arc<HostTexture>, &'static str>,
) -> Option<Dispatch> {
    let resolve = hash == crate::light_probe::SHADER;
    let hash = u64::from_str_radix(hash, 16).ok()?;
    let mut event = Dispatch { frame, hash, ordinal: None, reads: vec![], writes: vec![] };
    for u in used.iter().filter(|u| u.root == RootType::Table) {
        let input = resolve && u.kind == ResourceType::Srv;
        if !input && u.kind != ResourceType::Uav {
            continue;
        }
        let result = observed(u);
        if !input && result.is_err() {
            continue;
        }
        let untracked_reason = result.as_ref().err().copied();
        let texture = result.ok();
        let shape = texture.as_ref().filter(|_| input).map(|copy| {
            // SAFETY: the Arc owns this resource throughout the query.
            let desc = unsafe { copy.texture.GetDesc() };
            (desc.Format.0, desc.Width, desc.Height)
        });
        let binding = Binding {
            slot: Slot { root: u.slot, dword: u.table_offset_dwords },
            texture: texture.as_ref().map(Arc::downgrade),
            untracked_reason,
            shape,
        };
        if input {
            event.reads.push(binding);
        } else {
            event.writes.push(binding);
        }
    }
    (!event.reads.is_empty() || !event.writes.is_empty()).then_some(event)
}

#[derive(Copy, Clone)]
struct Writer {
    shader: u64,
    slot: Slot,
    position: (u64, usize),
}

struct History<T> {
    frame: Option<u64>,
    position: Option<(u64, usize)>,
    // Keeping the weak token alive prevents allocator address reuse. It does not
    // pin GPU memory or hold the underlying D3D12 texture alive.
    writers: HashMap<usize, (Weak<T>, Writer)>,
}

impl<T> Default for History<T> {
    fn default() -> Self {
        Self { frame: None, position: None, writers: HashMap::new() }
    }
}

impl<T> History<T> {
    fn begin_list(&mut self, position: (u64, usize)) {
        if self.position.is_some_and(|last| position <= last) {
            self.writers.clear();
        }
        self.position = Some(position);
    }

    fn begin_frame(&mut self, frame: u64) {
        if self.frame != Some(frame) {
            self.writers.clear();
            self.frame = Some(frame);
        }
    }

    fn remember(&mut self, texture: &Weak<T>, writer: Writer) {
        self.writers.insert(texture.as_ptr() as usize, (texture.clone(), writer));
    }

    fn writer(&self, texture: &Weak<T>) -> Option<Writer> {
        self.writers.get(&(texture.as_ptr() as usize)).map(|(_, writer)| *writer)
    }
}

pub(crate) fn completed(position: (u64, usize), events: &[Dispatch]) {
    if events.is_empty() {
        return;
    }
    static HISTORY: OnceLock<Mutex<History<HostTexture>>> = OnceLock::new();
    let mut history = HISTORY.get_or_init(Mutex::default).lock().unwrap_or_else(|e| e.into_inner());
    history.begin_list(position);
    for event in events {
        history.begin_frame(event.frame);
        for input in &event.reads {
            let previous = input.texture.as_ref().and_then(|texture| history.writer(texture));
            let status = match (&input.texture, previous) {
                (None, _) => "untracked binding",
                (Some(_), None) => "texture, no earlier tracked UAV",
                (Some(_), Some(_)) => "same texture as earlier tracked UAV",
            };
            tracing::info!(
                frame = event.frame, submission = position.0, list = position.1, probe_ordinal = event.ordinal,
                root = input.slot.root, dword = input.slot.dword, status,
                untracked_reason = input.untracked_reason,
                shape_format_width_height = ?input.shape,
                earlier_shader = ?previous.map(|w| format!("{:016x}", w.shader)),
                earlier_root_dword = ?previous.map(|w| (w.slot.root, w.slot.dword)),
                earlier_submission_list = ?previous.map(|w| w.position),
                "completed temporal resolve input binding"
            );
        }
        // Resolve SRVs see preceding dispatches, even if this very dispatch
        // also binds a UAV for one of the same textures.
        for binding in &event.writes {
            if let Some(texture) = &binding.texture {
                history.remember(texture, Writer { shader: event.hash, slot: binding.slot, position });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_resolves_select_one_command_per_sampled_frame() {
        let selection = Selection::default();
        assert!(selection.claim(0).is_none() && selection.claim(31).is_none());
        let ordinals = std::thread::scope(|scope| {
            let calls: Vec<_> = (0..8).map(|_| scope.spawn(|| selection.claim(32))).collect();
            calls.into_iter().filter_map(|call| call.join().unwrap()).collect::<Vec<_>>()
        });
        assert_eq!(ordinals, [1]);
        assert_eq!(selection.claim(64), Some(2));
        assert!(selection.claim(32).is_none() && selection.claim(64).is_none());
    }

    #[test]
    fn reports_untracked_resolve_inputs_and_filters_unobserved_outputs() {
        let srv = UsedResource {
            kind: ResourceType::Srv, root: RootType::Table, slot: 1,
            table_offset_dwords: 0, lower_bound: 16, stride: 288, dwords: 8, user_data: 1,
        };
        let entries = [
            srv,
            UsedResource { kind: ResourceType::Uav, table_offset_dwords: 176, ..srv },
            UsedResource { root: RootType::Descriptor, ..srv },
            UsedResource { kind: ResourceType::Sampler, ..srv },
        ];
        let mut visited = vec![];
        let event = bindings(32, crate::light_probe::SHADER, &entries, |u| {
            visited.push(u.table_offset_dwords);
            Err("absent")
        }).unwrap();
        assert_eq!(visited, [0, 176]);
        assert_eq!(event.reads.len(), 1);
        assert_eq!(event.reads[0].untracked_reason, Some("absent"));
        assert!(event.reads[0].texture.is_none() && event.writes.is_empty());
        assert!(bindings(32, "00000000000000ab", &entries, |_| Err("absent")).is_none());
    }

    #[test]
    fn tracks_order_identity_and_frame_without_pinning_resources() {
        let mut history = History::default();
        let first = Arc::new(());
        let second = Arc::new(());
        let token = Arc::downgrade(&first);
        let other = Arc::downgrade(&second);
        let position = (5, 1);
        let writer = Writer { shader: 8, slot: Slot { root: 1, dword: 16 }, position };
        history.begin_list(position);
        history.begin_frame(32);
        history.remember(&token, writer);
        assert!(history.writer(&other).is_none());
        drop(first);
        assert!(token.upgrade().is_none());
        history.begin_list((6, 0));
        assert_eq!(history.writer(&token).unwrap().shader, 8);
        history.remember(&token, Writer { shader: 12, ..writer });
        assert_eq!(history.writer(&token).unwrap().shader, 12);
        history.begin_list((5, 0));
        assert!(history.writer(&token).is_none());
        history.remember(&token, writer);
        history.begin_frame(64);
        assert!(history.writer(&token).is_none());
    }
}
