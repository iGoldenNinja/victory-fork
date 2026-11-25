//! Bounded GPU readback at the blend-shape translations extractor. The
//! diagnostic records per-binding numeric summaries after the list's fence.

use std::sync::{OnceLock, atomic::{AtomicU64, Ordering}};
use victory_shader::{ResourceType, UsedResource};
use windows::Win32::Graphics::Direct3D12::*;

use crate::{descriptor::View, host::{HostList, host, transition}};

const EXTRACTOR: &str = "970c4fbb3b017c0e";
const WHOLE_LIMIT: u64 = 4 * 1024 * 1024;

pub(crate) fn sample(hash: &str) -> Option<u64> {
    static ON: OnceLock<bool> = OnceLock::new();
    static COUNT: AtomicU64 = AtomicU64::new(0);
    if hash != EXTRACTOR || !*ON.get_or_init(|| std::env::var_os("VICTORY_FACE_PROBE").is_some_and(|v| v == "1")) {
        return None;
    }
    let n = COUNT.fetch_add(1, Ordering::Relaxed);
    n.is_multiple_of(32).then_some(n + 1)
}

pub(crate) struct Readback {
    buffer: ID3D12Resource,
    // D3D12 command recording does not keep a resource reference.
    _source: ID3D12Resource,
    sequence: u64,
    frame: u64,
    slot: u8,
    dword: u32,
    uav: bool,
    format: u32,
    view_bytes: u64,
    resource_bytes: u64,
    bytes: usize,
}

pub(crate) fn record(list: &HostList, view: &View, used: &UsedResource, sequence: u64) -> bool {
    let (format, uav) = match view {
        View::Shader(_, Some(d)) if d[1] == 1 => (d[0], false),
        View::Unordered(_, 0, Some(d)) if d[1] == 1 => (d[0], true),
        View::Placed(uav, resource, d) if resource[0] == 1 && d[1] == 1 => (d[0], *uav),
        _ => return false,
    };
    if uav != (used.kind == ResourceType::Uav) {
        return false;
    }
    let Some((_, resource_bytes)) = crate::descriptor::host::buffer_extent(view) else { return false };
    let Some((shadow, address, view_bytes)) = crate::descriptor::host::buffer_copy(view) else { return false };
    let Some(offset) = shadow.offset_in(address) else { return false };
    // Large arena views get a bounded prefix. The 1 MiB network output and
    // 2.8 MiB blend-shape output both fit in full.
    let bytes = if view_bytes <= WHOLE_LIMIT { view_bytes } else { 64 * 1024 };
    // SAFETY: the shadow holds the host resource.
    let desc = unsafe { shadow.buffer.GetDesc() };
    if bytes < 4 || desc.Dimension != D3D12_RESOURCE_DIMENSION_BUFFER
        || offset.checked_add(bytes).is_none_or(|end| end > desc.Width)
        || bytes > shadow.left(address) {
        return false;
    }
    let Some(buffer) = host().and_then(|h| h.read_back_buffer(bytes)) else { return false };
    let src = &shadow.buffer;
    let read = D3D12_RESOURCE_STATE_COPY_SOURCE;
    // The just-submitted compute SRV is implicitly promoted for reads;
    // COPY_SOURCE is another compatible read. The extractor's UAV requires
    // an explicit transition away from its write state. Return to COMMON so
    // later title commands can again promote the shadow according to use.
    // SAFETY: both source and readback are held through completion. All
    // ranges were checked against the actual resource immediately above.
    unsafe {
        if uav {
            list.list.ResourceBarrier(&[transition(src, D3D12_RESOURCE_STATE_UNORDERED_ACCESS, read)]);
        }
        list.list.CopyBufferRegion(&buffer, 0, src, offset, bytes);
        if uav {
            list.list.ResourceBarrier(&[transition(src, read, D3D12_RESOURCE_STATE_COMMON)]);
        }
    }
    list.defer_face_sample(Readback {
        buffer, _source: src.clone(), sequence, frame: crate::frame::current(),
        slot: used.slot, dword: used.table_offset_dwords, uav,
        format: crate::resource::pc_format(format), view_bytes, resource_bytes, bytes: bytes as usize,
    });
    true
}

impl Readback {
    pub(crate) fn report(&self) {
        let mut mapped = std::ptr::null_mut();
        let range = D3D12_RANGE { Begin: 0, End: self.bytes };
        // SAFETY: the host has waited for the submission. Map only the
        // completed private readback, then discard it after counting.
        unsafe {
            if self.buffer.Map(0, Some(&range), Some(&mut mapped)).is_err() || mapped.is_null() {
                tracing::warn!(sequence = self.sequence, "face extractor buffer sample could not be mapped");
                return;
            }
            let data = std::slice::from_raw_parts(mapped.cast::<u8>(), self.bytes);
            let summary = counts(data);
            self.buffer.Unmap(0, Some(&D3D12_RANGE { Begin: 0, End: 0 }));
            tracing::info!(
                hash = EXTRACTOR, d16_bits = victory_shader::emit::d16_uint(), sequence = self.sequence,
                recorded_frame = self.frame, access = if self.uav { "UAV" } else { "SRV" },
                slot = self.slot, dword = self.dword, dxgi_format = self.format,
                resource_bytes = self.resource_bytes, view_bytes = self.view_bytes, sampled_bytes = self.bytes,
                nonzero_bytes = summary.0, nonzero_finite_if_f16 = summary.1,
                nonzero_finite_if_f32 = summary.2, "face extractor buffer sample"
            );
        }
    }
}

fn counts(data: &[u8]) -> (usize, usize, usize) {
    let nonzero_bytes = data.iter().filter(|&&v| v != 0).count();
    let halves = data.chunks_exact(2).filter(|v| {
        let magnitude = u16::from_le_bytes([v[0], v[1]]) & 0x7fff;
        magnitude != 0 && magnitude < 0x7c00
    }).count();
    let floats = data.chunks_exact(4).filter(|v| {
        let value = f32::from_le_bytes([v[0], v[1], v[2], v[3]]);
        value != 0.0 && value.is_finite()
    }).count();
    (nonzero_bytes, halves, floats)
}

#[cfg(test)]
mod tests {
    #[test]
    fn counts_signed_zero_finite_and_special_patterns_separately() {
        let words = [0x0000_8000u32, 0x7c00_3c00, 0x7f80_0000, 0xffc0_0001];
        let bytes: Vec<_> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        assert_eq!(super::counts(&bytes), (8, 2, 2));
    }
}
