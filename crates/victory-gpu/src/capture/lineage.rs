//! Join a compute output and its graphics readers to the host descriptors and submissions.
//! VICTORY_CAPTURE_READERS=table,dword,pixel_hash[,pixel_hash...], with VICTORY_CAPTURE_PASS.
use std::sync::{Mutex, OnceLock, atomic::{AtomicU64, Ordering}};
use windows::core::Interface;
use windows::Win32::Graphics::Direct3D12::*;
use crate::{descriptor::{View, host::WatchedSlot}, host::HostList, object::This};

struct Readers {
    table: u8,
    dword: u32,
    hashes: Vec<String>,
}

fn parse(text: &str) -> Option<Readers> {
    let mut parts = text.split(',').map(str::trim);
    let table = parts.next()?.parse().ok()?;
    let dword: u32 = parts.next()?.parse().ok()?;
    let mut hashes = Vec::new();
    for word in parts {
        let hash = word.to_ascii_lowercase();
        if hash.len() != 16 || !hash.bytes().all(|c| c.is_ascii_hexdigit())
            || hashes.contains(&hash) || hashes.len() == 8 { return None; }
        hashes.push(hash);
    }
    (!hashes.is_empty() && dword.is_multiple_of(8)).then_some(Readers { table, dword, hashes })
}

fn readers() -> Option<&'static Readers> {
    static READERS: OnceLock<Option<Readers>> = OnceLock::new();
    READERS.get_or_init(|| {
        let text = std::env::var("VICTORY_CAPTURE_READERS").ok()?;
        let configured = parse(&text);
        if configured.is_none() { tracing::warn!("invalid graphics reader selection"); }
        configured
    }).as_ref()
}

pub(crate) fn active(frame: u64) -> bool {
    (super::trigger::listed(frame) || (frame > 1 && super::trigger::listed(frame - 1)))
        && enabled() && super::pass::config().is_some()
}

pub(crate) fn enabled() -> bool { readers().is_some() || super::producer::config().is_some() || super::producer::mask().is_some() }

pub(crate) struct Target {
    pub table: u8,
    pub dword: u32,
    shader: &'static str,
    writer: bool,
    pub input_dword: Option<u32>,
    pub extra_dword: Option<u32>,
    pub feed_constant: Option<u32>,
    pub counter_dword: Option<u32>,
    producer: bool,
    mask: bool,
    rect: [u32; 4],
}

pub(crate) fn target(hash: &str, compute: bool) -> Option<Target> {
    let pass = super::pass::config()?;
    let upstream = super::producer::config();
    if compute && let Some(cfg) = super::producer::mask() && cfg.pass.hash == hash {
        let mask = &cfg.pass;
        return Some(Target { table: mask.table, dword: mask.uav, shader: mask.hash.as_str(), writer: true,
            input_dword: Some(mask.srv), extra_dword: Some(cfg.list), feed_constant: Some(cfg.constant),
            counter_dword: cfg.counters, producer: false, mask: true, rect: mask.rect });
    }
    if compute && let Some(cfg) = upstream
        && let Some(shader) = cfg.shaders.iter().find(|h| h.as_str() == hash) {
        return Some(Target { table: cfg.table, dword: cfg.output, shader, writer: true,
            input_dword: Some(cfg.list), extra_dword: Some(cfg.constant), feed_constant: None,
            counter_dword: None, producer: true, mask: false, rect: cfg.rect });
    }
    let (table, dword, shader) = if compute {
        if pass.hash != hash { return None; }
        (pass.table, pass.uav, pass.hash.as_str())
    } else {
        let config = readers()?;
        let name = config.hashes.iter().find(|h| h.as_str() == hash)?;
        (config.table, config.dword, name.as_str())
    };
    let extra_dword = upstream.filter(|cfg| compute && cfg.table == table).map(|cfg| cfg.downstream);
    Some(Target { table, dword, shader, writer: compute, input_dword: compute.then_some(pass.srv),
        extra_dword, feed_constant: upstream.filter(|_| extra_dword.is_some()).and_then(|c| c.downstream_constant),
        counter_dword: None, producer: false, mask: false, rect: pass.rect })
}

pub(crate) fn host_index(root_bytes: u32, dword: u32) -> Option<u32> {
    root_bytes.checked_add(dword.checked_mul(4)?).map(|v| v / 32)
}

pub(crate) struct Binding {
    pub frame: u64,
    pub target: Target,
    pub slot: WatchedSlot,
    pub guest: usize,
    pub input: Option<(WatchedSlot, Option<View>)>,
    pub extra: Option<(WatchedSlot, Option<View>)>,
    pub feed_constant: Option<(WatchedSlot, Option<View>)>,
    pub counters: Option<(WatchedSlot, Option<View>)>,
}

pub(crate) fn guest_resource(view: &View) -> usize {
    let placed = crate::descriptor::host::placed_texture(view);
    match placed.as_ref().unwrap_or(view) {
        View::Shader(id, _) | View::Unordered(id, _, _) => *id,
        _ => 0,
    }
}

fn native(resource: Option<&ID3D12Resource>) -> usize {
    resource.map_or(0, |r| r.as_raw() as usize)
}

// A fresh number lives in the list's deferred data through reset/detach and
// submission. A recycled list pointer cannot join two different observations.
fn next_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

pub(crate) struct Event {
    id: u64,
    frame: u64,
    slot: Option<WatchedSlot>,
}

impl Event {
    pub(crate) fn submit(&self, submission: u64, fence: u64, position: usize) {
        let at_submit = self.slot.as_ref().and_then(WatchedSlot::resource);
        tracing::info!(id=self.id, frame=self.frame, submission, fence, position,
            native=native(at_submit.as_ref()), "descriptor trace submit");
    }
    pub(crate) fn completed(&self) {
        let kept = self.slot.as_ref().and_then(WatchedSlot::resource);
        tracing::info!(id=self.id, frame=self.frame, native=native(kept.as_ref()), "descriptor trace complete");
    }
}

pub(crate) fn marker(list: &HostList, frame: u64, role: &str, guest: usize, texture: &ID3D12Resource) {
    if !active(frame) { return; }
    let id = next_id();
    tracing::info!(id, frame, role, guest, native=native(Some(texture)), "descriptor trace marker");
    list.defer_descriptor_event(Event { id, frame, slot: None });
}

pub(crate) struct Pending {
    id: u64,
    frame: u64,
    target: Target,
    output: Option<ID3D12Resource>,
    list_buffer: Option<(ID3D12Resource, u64, u64)>,
    counter_buffer: Option<(ID3D12Resource, u64, u64)>,
}

impl Binding {
    pub(crate) fn begin(self, list: &HostList, rt: Option<usize>) -> Option<Pending> {
        // At most one instance of each configured program in each capture interval.
        static SEEN: Mutex<Vec<(&'static str, u64)>> = Mutex::new(Vec::new());
        let t = self.target;
        {
            let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((_, last)) = seen.iter_mut().find(|(hash, _)| *hash == t.shader) {
                if *last >= self.frame { return None; }
                *last = self.frame;
            } else { seen.push((t.shader, self.frame)); }
        }
        let id = next_id();
        let frame = self.frame;
        let actual = self.slot.resource();
        let expected = (self.guest != 0).then(|| crate::resource::host_made(self.guest as This)).flatten();
        let target = rt.and_then(|r| crate::resource::host_made(r as This)).cloned();
        let (source_state, role) = if t.writer {
            (D3D12_RESOURCE_STATE_UNORDERED_ACCESS, "writer")
        } else {
            (D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE, "reader")
        };
        let sample = |native: &ID3D12Resource, state, tag: &str| {
            super::pass::sample_native(list, native, state, frame, format!("slot-{id}-{tag}"), t.rect)
        };
        if let Some((slot, view)) = self.input {
            let input = slot.resource();
            let input_id = next_id();
            let guest = view.as_ref().map_or(0, guest_resource);
            let expected = (guest != 0).then(|| crate::resource::host_made(guest as This)).flatten();
            let tracked = input.as_ref().and_then(|r| list.tracked_texture_state(r));
            let sampled = if t.producer {
                super::producer::sample_slot(list, &slot, view.as_ref(), frame, format!("slot-{id}-list-before"))
            } else { input.as_ref().is_some_and(|r| super::pass::sample_native(list, r,
                D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE, frame, format!("slot-{id}-input-before"), t.rect)) };
            let view_words = view.as_ref().and_then(|v| match v { View::Shader(_, words) => *words, _ => None });
            tracing::info!(id=input_id, frame, role="compute-input", writer=id, shader=t.shader, dword=t.input_dword,
                heap=slot.heap, index=slot.index, guest, expected=native(expected), native=native(input.as_ref()),
                state=tracked.as_ref().map(|(_, s)| s.0), ?view_words, sampled, "descriptor trace binding");
            list.defer_descriptor_event(Event { id: input_id, frame, slot: Some(slot) });
        }
        let mut list_buffer = None;
        if let Some((slot, view)) = self.extra {
            let resource = slot.resource();
            let extra_id = next_id();
            let sampled = if t.mask {
                if t.counter_dword.is_some() {
                    list_buffer = super::producer::output_range(&slot, view.as_ref());
                }
                false
            } else if t.producer {
                super::producer::sample_slot(list, &slot, view.as_ref(), frame, format!("slot-{id}-constants-before"))
            } else {
                resource.as_ref().zip(super::producer::config()).is_some_and(|(r,cfg)|
                    super::pass::sample_native(list, r, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE, frame,
                        format!("slot-{id}-feed-before"), cfg.rect))
            };
            let role = if t.mask { "list-target" } else { "extra-input" };
            tracing::info!(id=extra_id, frame, role, writer=id, shader=t.shader, dword=t.extra_dword,
                heap=slot.heap, index=slot.index, native=native(resource.as_ref()), sampled, "descriptor trace binding");
            list.defer_descriptor_event(Event { id: extra_id, frame, slot: Some(slot) });
        }
        let mut counter_buffer = None;
        if let Some((slot, view)) = self.counters {
            counter_buffer = super::producer::output_range(&slot, view.as_ref());
            let counter_id = next_id();
            tracing::info!(id=counter_id, frame, role="counter-target", writer=id, shader=t.shader,
                dword=t.counter_dword, heap=slot.heap, index=slot.index, native=native(slot.resource().as_ref()),
                usable=counter_buffer.is_some(), "descriptor trace binding");
            list.defer_descriptor_event(Event { id: counter_id, frame, slot: Some(slot) });
        }
        if let Some((slot, view)) = self.feed_constant {
            let input_id = next_id();
            let resource = slot.resource();
            let sampled = super::producer::sample_slot(list, &slot, view.as_ref(), frame, format!("slot-{id}-feed-constants"));
            tracing::info!(id=input_id, frame, role="feed-constant", writer=id, shader=t.shader, dword=t.feed_constant,
                heap=slot.heap, index=slot.index, native=native(resource.as_ref()), sampled, "descriptor trace binding");
            list.defer_descriptor_event(Event { id: input_id, frame, slot: Some(slot) });
        }
        let sampled = actual.as_ref().is_some_and(|r| sample(r, source_state, "bound-before"));
        let rt_before = target.as_ref().is_some_and(|r| sample(r, D3D12_RESOURCE_STATE_RENDER_TARGET, "rt-before"));
        tracing::info!(id, frame, role, shader=t.shader, heap=self.slot.heap, index=self.slot.index,
            guest=self.guest, expected=native(expected), native=native(actual.as_ref()),
            rt=native(target.as_ref()), sampled, rt_before, "descriptor trace binding");
        list.defer_descriptor_event(Event { id, frame, slot: Some(self.slot) });
        let output = if t.writer { actual } else { target };
        Some(Pending { id, frame, target: t, output, list_buffer, counter_buffer })
    }
}

impl Pending {
    pub(crate) fn indirect(&self, list: &HostList, source: &ID3D12Resource, at: u64,
                           second: Option<u64>, maximum: u32, uses_counter: bool) {
        if !self.target.producer { return; }
        let state = list.indirect_state(source);
        if state != D3D12_RESOURCE_STATE_INDIRECT_ARGUMENT { return; }
        let capture = |offset: u64, tag: &str| super::pass::sample_buffer(list, source, offset, 12, Some(state),
            self.frame, format!("slot-{}-args-{tag}", self.id));
        let first = capture(at, "first");
        let overflow = second.and_then(|extra| at.checked_add(extra)).map(|offset| capture(offset, "second"));
        tracing::info!(id=self.id, frame=self.frame, maximum, uses_counter, first, ?overflow, "producer indirect arguments");
    }

    pub(crate) fn end(self, list: &HostList) {
        let (state, tag) = if self.target.writer { (D3D12_RESOURCE_STATE_UNORDERED_ACCESS, "bound-after") }
            else { (D3D12_RESOURCE_STATE_RENDER_TARGET, "rt-after") };
        let sampled = self.output.as_ref().is_some_and(|r| super::pass::sample_native(
            list, r, state, self.frame, format!("slot-{}-{tag}", self.id), self.target.rect));
        if self.target.counter_dword.is_some() {
            let copy = |range: Option<(ID3D12Resource, u64, u64)>, name: &str| {
                range.is_some_and(|(buffer, at, size)| super::pass::sample_buffer(list, &buffer, at, size,
                    Some(D3D12_RESOURCE_STATE_UNORDERED_ACCESS), self.frame, format!("slot-{}-{name}-after", self.id)))
            };
            let list_after = copy(self.list_buffer, "list");
            let counters_after = copy(self.counter_buffer, "counters");
            tracing::info!(id=self.id, frame=self.frame, list_after, counters_after, "generator buffer outputs");
        }
        tracing::info!(id=self.id, frame=self.frame, sampled, "descriptor trace end");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn readers_require_unique_hashes_and_aligned_descriptor_offsets() {
        let h = "0123456789abcdef";
        assert!(parse(&format!("1,72,{h}")).is_some());
        for bad in ["1,72,", "1,72,ok", "1,72,0123456789abcdef,0123456789ABCDEF", "1,2,0123456789abcdef"] {
            assert!(parse(bad).is_none(), "{bad}");
        }
    }
    #[test]
    fn bound_index_uses_the_actual_root_value_including_relocated_tables() {
        assert_eq!(host_index(32 * 400, 72), Some(409));
        assert_eq!(host_index(32 * 900_000, 72), Some(900_009));
        assert_eq!(host_index(u32::MAX, 72), None);
        assert_eq!(host_index(0, u32::MAX), None);
    }
}
