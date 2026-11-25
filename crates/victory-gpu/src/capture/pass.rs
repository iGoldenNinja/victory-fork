//! Record a bounded region beside one compute dispatch in the originating GPU list.
//! VICTORY_CAPTURE_PASS=shader,table,srv_dword,uav_dword,left,top,width,height.
//! The first matching dispatch on each one-shot frame captures the SRV before,
//! and the UAV before and after, with no per-draw readback waits.

use std::io::{BufWriter, Write};
use std::sync::{Arc, OnceLock, atomic::{AtomicU64, Ordering}};
use windows::Win32::Graphics::{Direct3D12::*, Dxgi::Common::*};
use crate::{descriptor::View, host::{HostList, host, transition}, object::This, resource::HostTexture};

pub(crate) struct Config {
    hash: String,
    pub table: u8,
    pub srv: u32,
    pub uav: u32,
    rect: [u32; 4],
}

fn parse(text: &str) -> Option<Config> {
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
    static CONFIG: OnceLock<Option<Config>> = OnceLock::new();
    let cfg = CONFIG.get_or_init(|| {
        let text = std::env::var("VICTORY_CAPTURE_PASS").ok()?;
        let parsed = parse(&text);
        if parsed.is_none() { tracing::warn!("invalid compute pass capture selection"); }
        parsed
    }).as_ref()?;
    (cfg.hash == hash).then_some((frame, cfg))
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
    // SAFETY: this list holds the tracked resource.
    let d = unsafe { copy.texture.GetDesc() };
    let [x, y, w, h] = rect;
    if d.Dimension != D3D12_RESOURCE_DIMENSION_TEXTURE2D || d.MipLevels != 1
        || d.DepthOrArraySize != 1 || d.SampleDesc.Count != 1
        || u64::from(x + w) > d.Width || y + h > d.Height { return None; }
    if let Some(words) = desc {
        let dim = if output { D3D12_UAV_DIMENSION_TEXTURE2D.0 } else { D3D12_SRV_DIMENSION_TEXTURE2D.0 };
        if words[1] != dim as u32 || crate::resource::pc_format(words[0]) != d.Format.0 as u32 { return None; }
    }
    let bpp = match d.Format {
        DXGI_FORMAT_R11G11B10_FLOAT | DXGI_FORMAT_R8G8B8A8_UNORM | DXGI_FORMAT_R32_FLOAT => 4,
        DXGI_FORMAT_R16G16B16A16_FLOAT => 8,
        _ => return None,
    };
    Some(Texture { copy, state, format: d.Format, bpp })
}

pub(crate) struct Region {
    back: ID3D12Resource,
    frame: u64,
    name: &'static str,
    format: DXGI_FORMAT,
    rect: [u32; 4],
    row: u32,
    pitch: u32,
}

impl Region {
    fn new(t: &Texture, frame: u64, name: &'static str, rect: [u32; 4]) -> Option<Self> {
        let row = rect[2] * t.bpp;
        let pitch = row.next_multiple_of(D3D12_TEXTURE_DATA_PITCH_ALIGNMENT);
        let back = host()?.read_back_buffer(u64::from(pitch) * u64::from(rect[3]))?;
        Some(Self { back, frame, name, format: t.format, rect, row, pitch })
    }

    fn record(&self, list: &HostList, t: &Texture) {
        let [left, top, width, height] = self.rect;
        let area = D3D12_BOX { left, top, front: 0, right: left + width, bottom: top + height, back: 1 };
        let src = D3D12_RESOURCE_STATE_COPY_SOURCE;
        // SAFETY: the state is from this list, both resources remain alive through its fence,
        // and the single-sample, single-subresource box and pitched buffer were checked above.
        unsafe {
            list.list.ResourceBarrier(&[transition(&t.copy.texture, t.state, src)]);
            let from = D3D12_TEXTURE_COPY_LOCATION {
                pResource: std::mem::transmute_copy(&t.copy.texture), Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 { SubresourceIndex: 0 },
            };
            let to = D3D12_TEXTURE_COPY_LOCATION {
                pResource: std::mem::transmute_copy(&self.back), Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 { PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
                    Offset: 0, Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                        Format: self.format, Width: width, Height: height, Depth: 1, RowPitch: self.pitch,
                    },
                }},
            };
            list.list.CopyTextureRegion(&to, 0, 0, 0, &from, Some(&area));
            list.list.ResourceBarrier(&[transition(&t.copy.texture, src, t.state)]);
        }
    }

    pub(crate) fn report(&self) {
        let Some(dir) = super::folder() else { return };
        let [x, y, w, h] = self.rect;
        let path = dir.join(format!("frame-{}-pass-{}-x{x}-y{y}-{w}x{h}-f{}.bin", self.frame, self.name, self.format.0));
        let write = (|| -> std::io::Result<()> {
            let mut file = BufWriter::new(std::fs::File::create(path)?);
            let len = (h - 1) as usize * self.pitch as usize + self.row as usize;
            let mut ptr = std::ptr::null_mut();
            // SAFETY: report runs only after list completion. Row padding is excluded from the file.
            unsafe {
                self.back.Map(0, Some(&D3D12_RANGE { Begin: 0, End: len }), Some(&mut ptr))
                    .map_err(std::io::Error::other)?;
                let bytes = std::slice::from_raw_parts(ptr.cast::<u8>(), len);
                let result = bytes.chunks(self.pitch as usize).try_for_each(|line| file.write_all(&line[..self.row as usize]));
                self.back.Unmap(0, Some(&D3D12_RANGE { Begin: 0, End: 0 }));
                result?;
            }
            file.flush()
        })();
        if let Err(error) = write { tracing::warn!(%error, frame = self.frame, "compute pass region was not saved"); }
    }
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
