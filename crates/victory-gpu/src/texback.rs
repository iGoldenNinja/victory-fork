//! Textures shaders write that the title's own code then reads on the CPU.
//! On the console the CPU and the GPU share memory, so what a compute pass
//! stores into a texture is in title memory for the CPU to read. Here it stays
//! in the texture's host copy. The title's ocean is one: its wave passes write
//! a 256x256 height map (R32_FLOAT) and a displacement map (R32G32_FLOAT), and
//! its CPU code samples both, which is what boats float on; it read memory
//! nothing had written (run rw1, `VICTORY_READ_WATCH=1`: about 30 places in
//! the title read those two kinds), and boats sank.
//!
//! For textures of the kinds in `VICTORY_TEXTURE_BACK` (comma-separated
//! `<width>x<height>f<format>`; `0` or unset for none), a list in which shaders write one copies it
//! into a read-back buffer of ours when it closes, and once the list has run
//! the texels are tiled into the texture's title memory in the console's
//! layout, as the console's GPU would have left them.
//!
//! shortcut: by kind, not by texture: textures of those kinds the title never
//! reads are copied back too (about a megabyte per such list). The copy shows
//! what the list's last write left; a title read in between sees the frame
//! before, as it would before the console's GPU has run.

use std::sync::{Arc, OnceLock};

use windows::Win32::Graphics::Direct3D12::*;

use crate::object::This;

mod destination;
use destination::{Registry, Ticket};

static DESTINATIONS: OnceLock<Registry> = OnceLock::new();

pub(crate) fn released(owner: usize) {
    if let Some(pending) = DESTINATIONS.get() {
        pending.released(owner);
    }
}

pub(crate) fn installed(resource: This, at: usize) {
    if kinds().is_empty() {
        return;
    }
    let size = if wanted(resource) {
        crate::resource::title_texture(resource).map(|(_, _, n)| n)
    } else {
        None
    };
    DESTINATIONS.get_or_init(Registry::new).installed(resource as usize, at, size);
}

pub(crate) fn remapped(at: usize, size: usize) {
    if let Some(pending) = DESTINATIONS.get() {
        pending.remapped(at, size);
    }
}

/// The kinds copied back: (width, height, format).
fn kinds() -> &'static [(u64, u32, u32)] {
    static KINDS: OnceLock<Vec<(u64, u32, u32)>> = OnceLock::new();
    KINDS.get_or_init(|| {
        let list = std::env::var("VICTORY_TEXTURE_BACK").unwrap_or_default();
        let kinds: Vec<(u64, u32, u32)> = list
            .split(',')
            .filter_map(|k| {
                let (size, format) = k.trim().split_once('f')?;
                let (w, h) = size.split_once('x')?;
                Some((w.parse().ok()?, h.parse().ok()?, format.parse().ok()?))
            })
            .collect();
        if !kinds.is_empty() {
            tracing::info!(?kinds, "textures of these kinds that shaders write are copied back into title memory (VICTORY_TEXTURE_BACK)");
        }
        kinds
    })
}

/// Whether title texture `resource` is of a kind copied back.
pub(crate) fn wanted(resource: This) -> bool {
    let kinds = kinds();
    !kinds.is_empty()
        && crate::resource::texture_kind(resource).is_some_and(|k| kinds.contains(&k))
}

/// Capture the guest destination at shader-write recording time. Retaining
/// only the host copy lets the title destroy its own resource normally.
pub(crate) struct Target {
    pub(crate) copy: Arc<crate::resource::HostTexture>,
    layout: victory_formats::texture::Layout,
    kind: (u64, u32, u32),
    ticket: Ticket,
}

impl Target {
    pub(crate) fn new(resource: This) -> Option<Self> {
        let copy = crate::resource::host_copy(resource)?;
        let _mapping = crate::watch::title_memory_reading_now()?;
        let kind = crate::resource::texture_kind(resource)?;
        let (layout, at, size) = crate::resource::title_texture(resource)?;
        let ticket = DESTINATIONS.get()?.capture(resource as usize, at, size)?;
        Some(Self { copy, layout, kind, ticket })
    }

    pub(crate) fn matches(&self, owner: This) -> bool {
        self.ticket.matches(owner as usize)
    }
}

/// A texture's copy into a read-back buffer, recorded into a list, and where
/// its texels go once the list has run.
pub(crate) struct Back {
    buffer: ID3D12Resource,
    ticket: Ticket,
    footprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT,
    total: usize,
    layout: victory_formats::texture::Layout,
}

pub(crate) fn record(
    list: &ID3D12GraphicsCommandList,
    target: Target,
) -> Option<Back> {
    if !target.ticket.is_current() {
        return None;
    }
    let host = crate::host::host()?;
    let texture = &target.copy.texture;
    // SAFETY: the list owns the host texture.
    let desc = unsafe { texture.GetDesc() };
    let (width, height, format) = target.kind;
    if desc.Dimension != D3D12_RESOURCE_DIMENSION_TEXTURE2D
        || desc.MipLevels != 1 || desc.DepthOrArraySize != 1 || desc.SampleDesc.Count != 1
        || (desc.Width, desc.Height, desc.Format.0 as u32) != (width, height, format)
    {
        return None;
    }
    let mut footprint = D3D12_PLACED_SUBRESOURCE_FOOTPRINT::default();
    let mut total = 0;
    // SAFETY: one single-plane 2D subresource, one footprint.
    unsafe {
        host.device.GetCopyableFootprints(
            &desc, 0, 1, 0, Some(&mut footprint), None, None, Some(&mut total),
        );
    }
    if footprint.Offset != 0 || total == 0 || total == u64::MAX {
        return None;
    }
    let buffer = host.read_back_buffer(total)?;
    let to = D3D12_TEXTURE_COPY_LOCATION {
        // SAFETY: borrowed interface retained by Back.
        pResource: unsafe { std::mem::transmute_copy(&buffer) },
        Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
        Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 { PlacedFootprint: footprint },
    };
    let from = D3D12_TEXTURE_COPY_LOCATION {
        // SAFETY: borrowed interface retained by the list.
        pResource: unsafe { std::mem::transmute_copy(texture) },
        Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
        Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 { SubresourceIndex: 0 },
    };
    // SAFETY: open list with the source in COPY_SOURCE.
    unsafe { list.CopyTextureRegion(&to, 0, 0, 0, &from, None) };
    Some(Back { buffer, ticket: target.ticket, footprint, total: total as usize, layout: target.layout })
}

/// Merge the completed plane with the current guest padding while mappings are stable.
pub(crate) fn land(back: &Back) {
    use std::sync::atomic::{AtomicU64, Ordering};
    use windows_sys::Win32::System::{
        Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory}, Threading::GetCurrentProcess,
    };
    let Some(_reading) = crate::watch::title_memory_reading_now() else {
        return;
    };
    // Lock in the same order as a mapping change: mapping first, tickets second.
    let Some(_destination) = DESTINATIONS.get().and_then(|pending| pending.landing(&back.ticket)) else {
        static STALE: AtomicU64 = AtomicU64::new(0);
        let skipped = STALE.fetch_add(1, Ordering::Relaxed) + 1;
        if skipped.is_power_of_two() {
            tracing::info!(skipped, "water readback skipped after destination changed");
        }
        return;
    };
    let at = back.ticket.at;
    let mut tiled = vec![0u8; back.ticket.size];
    let mut done = 0;
    // SAFETY: the kernel checks guest pages and tiled owns the full destination.
    let process = unsafe { GetCurrentProcess() };
    let ok = unsafe {
        ReadProcessMemory(process, at as _, tiled.as_mut_ptr().cast(), tiled.len(), &mut done)
    };
    if ok == 0 || done != tiled.len() {
        return;
    }
    let mut data = std::ptr::null_mut();
    // SAFETY: the list has completed and the host retains the readback buffer.
    if unsafe { back.buffer.Map(0, None, Some(&mut data)) }.is_err() || data.is_null() {
        return;
    }
    // SAFETY: the allocation is exactly GetCopyableFootprints' total byte count.
    let linear = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), back.total) };
    let pitch = back.footprint.Footprint.RowPitch as usize;
    let depth_pitch = pitch * back.footprint.Footprint.Height as usize;
    let result = back.layout.tile(&mut tiled, 0, 0, linear, pitch, depth_pitch);
    // SAFETY: the CPU only read the completed buffer.
    unsafe { back.buffer.Unmap(0, None) };
    if result.is_err() {
        return;
    }
    crate::watch::writing(at, tiled.len());
    // SAFETY: tiled is valid for size bytes; the kernel checks the guest destination.
    done = 0;
    let ok = unsafe {
        WriteProcessMemory(process, at as _, tiled.as_ptr().cast(), tiled.len(), &mut done)
    };
    if done != 0 {
        crate::watch::wrote(at, done);
    }
    if ok != 0 && done == tiled.len() {
        static WRITES: [AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];
        let f = &back.footprint.Footprint;
        let index = match (f.Width, f.Format.0) {
            (64, 16) => 0, (128, 36) => 1, (256, 16) => 2, (256, 41) => 3, _ => return,
        };
        let completed = WRITES[index].fetch_add(1, Ordering::Relaxed) + 1;
        if completed.is_power_of_two() {
            tracing::info!(width = f.Width, height = f.Height, format = f.Format.0, completed,
                "water texture written to guest memory");
        }
    }
}
