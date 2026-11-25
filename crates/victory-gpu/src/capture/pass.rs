//! Record a bounded region beside one compute dispatch in the originating GPU list.
//! VICTORY_CAPTURE_PASS=shader,table,srv_dword,uav_dword,left,top,width,height.
//! The first matching dispatch on each one-shot frame captures the SRV before,
//! and the UAV before and after, with no per-draw readback waits.

use std::io::{BufWriter, Write};
use std::sync::{Arc, OnceLock, atomic::{AtomicU64, Ordering}};
use windows::Win32::Graphics::{Direct3D12::*, Dxgi::Common::*};
use crate::{descriptor::View, host::{HostList, host, transition}, object::This, resource::HostTexture};

pub(crate) struct Config {
    pub(super) hash: String,
    pub table: u8,
    pub srv: u32,
    pub uav: u32,
    pub(super) rect: [u32; 4],
}

pub(super) fn parse(text: &str) -> Option<Config> {
    let mut fields = text.split(',').map(str::trim);
    let hash = fields.next()?;
    if hash.len() != 16 || !hash.bytes().all(|c| c.is_ascii_hexdigit()) { return None; }
    let table = fields.next()?.parse().ok()?;
    let srv = fields.next()?.parse().ok()?;
    let uav = fields.next()?.parse().ok()?;
    let [x, y, w, h] = std::array::from_fn(|_| fields.next()?.parse::<u32>().ok());
    let rect = [x?, y?, w?, h?];
    if fields.next().is_some() || srv == uav || rect[2] == 0 || rect[3] == 0
        || u64::from(rect[2]) * u64::from(rect[3]) > 1024 * 1024
        || rect[0].checked_add(rect[2]).is_none() || rect[1].checked_add(rect[3]).is_none() {
        return None;
    }
    Some(Config { hash: hash.to_ascii_lowercase(), table, srv, uav, rect })
}

pub(crate) fn selected(hash: &str) -> Option<(u64, &'static Config)> {
    let frame = crate::queue::presented() + 1;
    if !super::trigger::listed(frame) { return None; }
    let cfg = config()?;
    (cfg.hash == hash).then_some((frame, cfg))
}

pub(super) fn config() -> Option<&'static Config> {
    static CONFIG: OnceLock<Option<Config>> = OnceLock::new();
    CONFIG.get_or_init(|| {
        let text = std::env::var("VICTORY_CAPTURE_PASS").ok()?;
        let parsed = parse(&text);
        if parsed.is_none() { tracing::warn!("invalid compute pass capture selection"); }
        parsed
    }).as_ref()
}

struct Texture {
    copy: Arc<HostTexture>,
    state: D3D12_RESOURCE_STATES,
    format: DXGI_FORMAT,
    bpp: u32,
}

fn binding(list: &HostList, view: View, output: bool, rect: [u32; 4]) -> Option<Texture> {
    let resolved = crate::descriptor::host::placed_texture(&view);
    let (id, desc) = match (resolved.as_ref().unwrap_or(&view), output) {
        (View::Shader(id, desc), false) | (View::Unordered(id, 0, desc), true) => (*id, desc),
        _ => return None,
    };
    let (copy, state) = list.pass_texture_state(id as This)?;
    if (output && state != D3D12_RESOURCE_STATE_UNORDERED_ACCESS)
        || (!output && !state.contains(D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE)) { return None; }
    let texture = Texture::checked(copy, state, rect)?;
    if let Some(words) = desc {
        let dim = if output { D3D12_UAV_DIMENSION_TEXTURE2D.0 } else { D3D12_SRV_DIMENSION_TEXTURE2D.0 };
        let view_format = crate::resource::pc_format(words[0]) as i32;
        let depth_srv = !output && matches!(texture.format, DXGI_FORMAT_R32_TYPELESS | DXGI_FORMAT_D32_FLOAT)
            && view_format == DXGI_FORMAT_R32_FLOAT.0 && words[4] == 0 && words[6] == 0;
        if words[1] != dim as u32 || (!depth_srv && view_format != texture.format.0) { return None; }
    }
    Some(texture)
}

impl Texture {
    fn checked(copy: Arc<HostTexture>, state: D3D12_RESOURCE_STATES, rect: [u32; 4]) -> Option<Self> {
        // SAFETY: the host copy owns the live resource.
        let d = unsafe { copy.texture.GetDesc() };
        let [x, y, w, h] = rect;
        if d.Dimension != D3D12_RESOURCE_DIMENSION_TEXTURE2D || d.MipLevels != 1
            || d.DepthOrArraySize != 1 || d.SampleDesc.Count != 1
            || u64::from(x + w) > d.Width || y + h > d.Height { return None; }
        let bpp = match d.Format {
            DXGI_FORMAT_R11G11B10_FLOAT | DXGI_FORMAT_R10G10B10A2_UNORM | DXGI_FORMAT_R8G8B8A8_UNORM | DXGI_FORMAT_R32_FLOAT
            | DXGI_FORMAT_R32_TYPELESS | DXGI_FORMAT_R32_UINT | DXGI_FORMAT_D32_FLOAT => 4,
            DXGI_FORMAT_R16G16B16A16_FLOAT => 8,
            _ => return None,
        };
        Some(Texture { copy, state, format: d.Format, bpp })
    }
}

pub(crate) struct Region {
    back: ID3D12Resource,
    frame: u64,
    name: String,
    format: DXGI_FORMAT,
    rect: [u32; 4],
    row: u32,
    footprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT,
    area: Option<D3D12_BOX>,
    start: usize,
}

impl Region {
    fn new(t: &Texture, frame: u64, name: impl Into<String>, rect: [u32; 4]) -> Option<Self> {
        Self::allocate(&t.copy.texture, t.bpp, frame, name.into(), rect)
    }

    fn allocate(resource: &ID3D12Resource, bpp: u32, frame: u64, name: String, rect: [u32; 4]) -> Option<Self> {
        let host = host()?;
        // SAFETY: the caller holds the resource and it has a single sample, mip and slice.
        let desc = unsafe { resource.GetDesc() };
        let [x, y, w, h] = rect;
        let row = w.checked_mul(bpp)?;
        let whole = desc.Flags.contains(D3D12_RESOURCE_FLAG_ALLOW_DEPTH_STENCIL);
        let (footprint, area, start, size) = if whole {
            // D3D12 requires the whole depth subresource with no source box. The
            // device supplies its copy format and row pitch; crop only after the fence.
            let mut layout = D3D12_PLACED_SUBRESOURCE_FOOTPRINT::default();
            let (mut rows, mut bytes, mut size) = (0u32, 0u64, 0u64);
            unsafe { host.device.GetCopyableFootprints(&desc, 0, 1, 0, Some(&mut layout),
                Some(&mut rows), Some(&mut bytes), Some(&mut size)) };
            if rows != desc.Height || bytes != desc.Width.checked_mul(u64::from(bpp))?
                || size == 0 || size == u64::MAX { return None; }
            let start = layout.Offset + u64::from(y) * u64::from(layout.Footprint.RowPitch) + u64::from(x) * u64::from(bpp);
            (layout, None, start, size)
        } else {
            let pitch = row.checked_next_multiple_of(D3D12_TEXTURE_DATA_PITCH_ALIGNMENT)?;
            let layout = D3D12_PLACED_SUBRESOURCE_FOOTPRINT { Offset: 0, Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                Format: desc.Format, Width: w, Height: h, Depth: 1, RowPitch: pitch,
            }};
            let area = D3D12_BOX { left: x, top: y, front: 0, right: x + w, bottom: y + h, back: 1 };
            (layout, Some(area), 0, u64::from(pitch) * u64::from(h))
        };
        let end = start + u64::from(h - 1) * u64::from(footprint.Footprint.RowPitch) + u64::from(row);
        if end > size { return None; }
        let back = host.read_back_buffer(size)?;
        Some(Self { back, frame, name, format: desc.Format, rect, row, footprint, area, start: start.try_into().ok()? })
    }

    fn record(&self, list: &HostList, t: &Texture) {
        self.record_resource(list, &t.copy.texture, t.state);
    }

    fn record_resource(&self, list: &HostList, resource: &ID3D12Resource, state: D3D12_RESOURCE_STATES) {
        let src = D3D12_RESOURCE_STATE_COPY_SOURCE;
        // SAFETY: the resource's tracked state is restored on the same list. The
        // footprint is valid for the crop or, for depth, the full subresource.
        unsafe {
            list.list.ResourceBarrier(&[transition(resource, state, src)]);
            let from = D3D12_TEXTURE_COPY_LOCATION {
                pResource: std::mem::transmute_copy(resource), Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 { SubresourceIndex: 0 },
            };
            let to = D3D12_TEXTURE_COPY_LOCATION {
                pResource: std::mem::transmute_copy(&self.back), Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 { PlacedFootprint: self.footprint },
            };
            list.list.CopyTextureRegion(&to, 0, 0, 0, &from, self.area.as_ref().map(std::ptr::from_ref));
            list.list.ResourceBarrier(&[transition(resource, src, state)]);
        }
    }

    fn write_rows(&self, out: &mut impl Write) -> std::io::Result<()> {
        let pitch = self.footprint.Footprint.RowPitch as usize;
        let span = (self.rect[3] - 1) as usize * pitch + self.row as usize;
        let mut ptr = std::ptr::null_mut();
        // SAFETY: the submission completed before reporting. The mapped range
        // includes the crop's final row and the inter-row padding.
        unsafe {
            self.back.Map(0, Some(&D3D12_RANGE { Begin: self.start, End: self.start + span }), Some(&mut ptr))
                .map_err(std::io::Error::other)?;
            let bytes = std::slice::from_raw_parts(ptr.cast::<u8>().add(self.start), span);
            let result = bytes.chunks(pitch).try_for_each(|line| out.write_all(&line[..self.row as usize]));
            self.back.Unmap(0, Some(&D3D12_RANGE { Begin: 0, End: 0 }));
            result
        }
    }

    pub(crate) fn report(&self) {
        let Some(dir) = super::folder() else { return };
        let [x, y, w, h] = self.rect;
        let path = dir.join(format!("frame-{}-pass-{}-x{x}-y{y}-{w}x{h}-f{}.bin", self.frame, self.name, self.format.0));
        let write = (|| -> std::io::Result<()> {
            let mut file = BufWriter::new(std::fs::File::create(path)?);
            self.write_rows(&mut file)?;
            file.flush()
        })();
        if let Err(error) = write { tracing::warn!(%error, frame = self.frame, "compute pass region was not saved"); }
    }
}

/// Sample the resource the actual host slot holds only when this list owns
/// its state and the selected rectangle is valid for its subresource.
pub(super) fn sample_native(list: &HostList, native: &ID3D12Resource, required: D3D12_RESOURCE_STATES,
                            frame: u64, name: String, rect: [u32; 4]) -> bool {
    let Some((copy, state)) = list.tracked_texture_state(native) else { return false };
    if !state.contains(required) { return false; }
    let Some(texture) = Texture::checked(copy, state, rect) else { return false };
    let Some(region) = Region::new(&texture, frame, name, rect) else { return false };
    region.record(list, &texture);
    list.defer_pass_region(region);
    true
}

/// Shadow CBVs and SRVs use implicitly promoted read states like the ordinary
/// shadow readbacks. An explicitly transitioned indirect buffer is restored.
pub(super) fn sample_buffer(list: &HostList, source: &ID3D12Resource, offset: u64, bytes: u64,
                            state: Option<D3D12_RESOURCE_STATES>, frame: u64, name: String) -> bool {
    if bytes == 0 || bytes > 1_048_576 || !bytes.is_multiple_of(4) { return false; }
    // SAFETY: the caller retains the source across recording and submission.
    let desc = unsafe { source.GetDesc() };
    if desc.Dimension != D3D12_RESOURCE_DIMENSION_BUFFER
        || offset.checked_add(bytes).is_none_or(|end| end > desc.Width) { return false; }
    let Some(back) = host().and_then(|h| h.read_back_buffer(bytes)) else { return false };
    let copy = D3D12_RESOURCE_STATE_COPY_SOURCE;
    unsafe {
        if let Some(before) = state { list.list.ResourceBarrier(&[transition(source, before, copy)]); }
        list.list.CopyBufferRegion(&back, 0, source, offset, bytes);
        if let Some(before) = state { list.list.ResourceBarrier(&[transition(source, copy, before)]); }
    }
    list.keep(source);
    let mut footprint = D3D12_PLACED_SUBRESOURCE_FOOTPRINT::default();
    footprint.Footprint.RowPitch = bytes as u32;
    list.defer_pass_region(Region { back, frame, name, format: DXGI_FORMAT_R32_UINT,
        rect: [0, 0, bytes as u32 / 4, 1], row: bytes as u32, footprint, area: None, start: 0 });
    true
}

pub(crate) struct Pending(Texture, Region);

pub(crate) fn begin(list: &HostList, frame: u64, cfg: &Config, srv: Option<View>, uav: Option<View>) -> Option<Pending> {
    static LAST: AtomicU64 = AtomicU64::new(0);
    if LAST.load(Ordering::Relaxed) >= frame { return None; }
    let output = binding(list, uav?, true, cfg.rect)?;
    let before = Region::new(&output, frame, "output-before", cfg.rect)?;
    let after = Region::new(&output, frame, "output-after", cfg.rect)?;
    if LAST.fetch_max(frame, Ordering::Relaxed) >= frame { return None; }
    if let Some(input) = srv.and_then(|view| binding(list, view, false, cfg.rect))
        && let Some(region) = Region::new(&input, frame, "input-before", cfg.rect) {
        region.record(list, &input);
        list.defer_pass_region(region);
    }
    before.record(list, &output);
    list.defer_pass_region(before);
    Some(Pending(output, after))
}

impl Pending {
    pub(crate) fn end(self, list: &HostList) {
        self.1.record(list, &self.0);
        list.defer_pass_region(self.1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a native D3D12 device"]
    fn depth_readback_preserves_texel_bits_in_offset_crop() {
        use windows::Win32::{Foundation::RECT, Graphics::Dxgi::*};
        let host = host().expect("D3D12 device");
        let adapter = unsafe {
            let factory: IDXGIFactory4 = CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0)).unwrap();
            factory.EnumAdapterByLuid::<IDXGIAdapter1>(host.device.GetAdapterLuid()).unwrap().GetDesc1().unwrap()
        };
        println!("depth probe adapter: {}", String::from_utf16_lossy(&adapter.Description).trim_end_matches(char::from(0)));
        let desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D, Width: 97, Height: 41, MipLevels: 1,
            DepthOrArraySize: 1, Format: DXGI_FORMAT_R32_TYPELESS, SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Flags: D3D12_RESOURCE_FLAG_ALLOW_DEPTH_STENCIL, ..Default::default()
        };
        let heap = D3D12_HEAP_PROPERTIES { Type: D3D12_HEAP_TYPE_DEFAULT, CreationNodeMask: 1,
            VisibleNodeMask: 1, ..Default::default() };
        let write = D3D12_RESOURCE_STATE_DEPTH_WRITE;
        let read = D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE;
        let mut texture: Option<ID3D12Resource> = None;
        unsafe { host.device.CreateCommittedResource(&heap, D3D12_HEAP_FLAG_NONE, &desc, write, None, &mut texture).unwrap() };
        let texture = texture.unwrap();
        let dsv_desc = D3D12_DEPTH_STENCIL_VIEW_DESC { Format: DXGI_FORMAT_D32_FLOAT,
            ViewDimension: D3D12_DSV_DIMENSION_TEXTURE2D, ..Default::default() };
        let dsv = host.dsv(&texture, Some(&dsv_desc));
        let rect = RECT { left: 16, top: 12, right: 43, bottom: 20 };
        let region = Region::allocate(&texture, 4, 0, String::new(), [13, 9, 39, 15]).unwrap();
        assert!(region.area.is_none());
        let list = host.new_list().unwrap();
        unsafe {
            list.list.ClearDepthStencilView(dsv, D3D12_CLEAR_FLAG_DEPTH, 0.375, 0, None);
            list.list.ClearDepthStencilView(dsv, D3D12_CLEAR_FLAG_DEPTH, 0.0, 0, Some(&[rect]));
            list.list.ResourceBarrier(&[transition(&texture, write, read)]);
        }
        region.record_resource(&list, &texture, read);
        list.close();
        host.execute(&[&list]);
        let mut bytes = Vec::new();
        region.write_rows(&mut bytes).unwrap();
        let values: Vec<_> = bytes.chunks_exact(4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).collect();
        let expected: Vec<_> = (9..24).flat_map(|y| (13..52).map(move |x| {
            if (16..43).contains(&x) && (12..20).contains(&y) { 0 } else { 0.375f32.to_bits() }
        })).collect();
        assert_eq!(values, expected);
    }

    #[test]
    fn pass_region_checks_coordinates_and_limits() {
        let prefix = "0123456789ABCDEF,1,32,64,";
        let cfg = parse(&format!("{prefix}4,7,1024,1024")).unwrap();
        assert_eq!(cfg.rect, [4, 7, 1024, 1024]);
        assert_eq!(cfg.hash, "0123456789abcdef");
        for rect in ["1,1,0,1", "1,1,1025,1024", "4294967295,1,1,1", "0,0,1,1,2"] {
            assert!(parse(&format!("{prefix}{rect}")).is_none());
        }
    }
}
