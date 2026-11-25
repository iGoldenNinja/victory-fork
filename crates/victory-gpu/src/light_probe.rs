//! Sample the Series X reflective-caustics temporal resolve's original output.
//! Only numeric summaries leave the completed readback. The view address follows
//! the probe-resolve binding identified in fxceless/victory-fork PR 917.

use std::sync::{
    Once, OnceLock,
    atomic::{AtomicU64, Ordering},
};
use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R32_UINT;

use crate::{
    descriptor::View,
    host::{HostList, host, transition},
    object::This,
};

pub(crate) const SHADER: &str = "34fb51b8a0312e82";
const ROWS: u32 = 8;

fn enabled(value: Option<&std::ffi::OsStr>) -> bool {
    value.is_some_and(|v| v == "1")
}

/// Sample the first matching recorded dispatch and then every 64th after it.
pub(crate) fn sample(hash: &str) -> Option<u64> {
    static ON: OnceLock<bool> = OnceLock::new();
    static COUNT: AtomicU64 = AtomicU64::new(0);
    if !*ON.get_or_init(|| enabled(std::env::var_os("VICTORY_PROBE_LIGHT").as_deref()))
        || hash != SHADER
    {
        return None;
    }
    let index = COUNT.fetch_add(1, Ordering::Relaxed);
    index.is_multiple_of(64).then_some(index + 1)
}

macro_rules! reject {
    ($($arg:tt)*) => {{
        static ONCE: Once = Once::new();
        ONCE.call_once(|| tracing::warn!($($arg)*));
        return;
    }};
}

pub(crate) fn record(list: &HostList, view: Option<View>, sequence: u64) {
    let Some(View::Unordered(resource, 0, words)) = view else {
        reject!("light probe skipped: expected a texture UAV");
    };
    // A default view on the single-mip, single-slice typed resource checked
    // below selects mip and plane zero. Check the same for an explicit view.
    if let Some(d) = words
        && (crate::resource::pc_format(d[0]) != DXGI_FORMAT_R32_UINT.0 as u32
            || d[1] != D3D12_UAV_DIMENSION_TEXTURE2D.0 as u32
            || d[2] != 0
            || d[3] != 0)
    {
        reject!(
            format = d[0],
            dimension = d[1],
            mip = d[2],
            plane = d[3],
            "light probe skipped: unsupported explicit texture UAV"
        );
    }
    let Some(copy) = list.probe_source(resource as This) else {
        reject!("light probe skipped: texture is absent from this list's unordered-access state");
    };
    let texture = &copy.texture;
    // SAFETY: the list owns the host copy.
    let d = unsafe { texture.GetDesc() };
    if d.Dimension != D3D12_RESOURCE_DIMENSION_TEXTURE2D
        || d.Format != DXGI_FORMAT_R32_UINT
        || d.DepthOrArraySize != 1
        || d.MipLevels != 1
        || d.SampleDesc.Count != 1
        || d.Width == 0
        || d.Width > 16384
        || d.Height == 0
        || d.Height > 16384
        || !d.Flags.contains(D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS)
    {
        reject!(
            format = d.Format.0,
            width = d.Width,
            height = d.Height,
            dimension = d.Dimension.0,
            layers = d.DepthOrArraySize,
            mips = d.MipLevels,
            samples = d.SampleDesc.Count,
            flags = d.Flags.0,
            "light probe skipped: unsupported resource"
        );
    }
    let width = d.Width as u32;
    let rows = ROWS.min(d.Height);
    // A 512-byte-aligned single row also meets the 256-byte pitch requirement.
    let stride = (4 * width).next_multiple_of(D3D12_TEXTURE_DATA_PLACEMENT_ALIGNMENT);
    let Some(buffer) = host().and_then(|h| h.read_back_buffer(u64::from(stride * rows))) else {
        reject!("light probe skipped: readback allocation unavailable");
    };
    let sample = Readback {
        buffer,
        width,
        height: d.Height,
        rows,
        stride,
        frame: crate::frame::current(),
        sequence,
    };
    sample.copy_from(&list.list, texture);
    list.defer_light_sample(sample);
}

fn sample_row(height: u32, band: u32, rows: u32) -> u32 {
    (2 * band + 1) * height / (2 * rows)
}

#[derive(Default)]
struct Counts {
    zero_words: u32,
    nonzero_channels: [u32; 3],
    exponents: [u32; 32],
}

impl Counts {
    /// Count zero RGB even if the shared exponent is nonzero.
    fn row(&mut self, words: &[u32]) -> u32 {
        let mut zero_rgb = 0;
        for &word in words {
            self.zero_words += u32::from(word == 0);
            zero_rgb += u32::from(word & 0x07ff_ffff == 0);
            for (i, n) in self.nonzero_channels.iter_mut().enumerate() {
                *n += u32::from((word >> (i * 9)) & 511 != 0);
            }
            self.exponents[(word >> 27) as usize] += 1;
        }
        zero_rgb
    }
}

pub(crate) struct Readback {
    buffer: ID3D12Resource,
    width: u32,
    height: u32,
    rows: u32,
    stride: u32,
    frame: u64,
    sequence: u64,
}

impl Readback {
    fn copy_from(&self, list: &ID3D12GraphicsCommandList, texture: &ID3D12Resource) {
        let uav = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
        let source = D3D12_RESOURCE_STATE_COPY_SOURCE;
        // All transitions and copies are adjacent to the original dispatch in
        // its list. The second barrier restores the tracked state before any
        // following command. CopyTextureRegion handles the texture's native layout.
        // SAFETY: this list holds the validated texture. Its deferred readback
        // owns the destination until the recorded work finishes and is reset.
        unsafe {
            list.ResourceBarrier(&[transition(texture, uav, source)]);
            let from = D3D12_TEXTURE_COPY_LOCATION {
                pResource: std::mem::transmute_copy(texture),
                Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    SubresourceIndex: 0,
                },
            };
            for band in 0..self.rows {
                let top = sample_row(self.height, band, self.rows);
                let line = D3D12_BOX {
                    left: 0,
                    right: self.width,
                    top,
                    bottom: top + 1,
                    front: 0,
                    back: 1,
                };
                let to = D3D12_TEXTURE_COPY_LOCATION {
                    pResource: std::mem::transmute_copy(&self.buffer),
                    Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
                    Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                        PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
                            Offset: u64::from(band * self.stride),
                            Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                                Format: DXGI_FORMAT_R32_UINT,
                                Width: self.width,
                                Height: 1,
                                Depth: 1,
                                RowPitch: self.stride,
                            },
                        },
                    },
                };
                list.CopyTextureRegion(&to, 0, 0, 0, &from, Some(&line));
            }
            list.ResourceBarrier(&[transition(texture, source, uav)]);
        }
    }

    fn counts(&self) -> Option<(Counts, [u32; ROWS as usize])> {
        let mut data = std::ptr::null_mut();
        let read = D3D12_RANGE {
            Begin: 0,
            End: ((self.rows - 1) * self.stride + self.width * 4) as usize,
        };
        let mut counts = Counts::default();
        let mut zero_by_band = [0u32; ROWS as usize];
        // SAFETY: the queue has completed this list. Each source row was
        // copied to a 512-byte-aligned destination with width texels. The
        // slice excludes padding and only the CPU reads the returned memory.
        unsafe {
            if self.buffer.Map(0, Some(&read), Some(&mut data)).is_err() || data.is_null() {
                return None;
            }
            for band in 0..self.rows {
                let line = data
                    .cast::<u8>()
                    .add((band * self.stride) as usize)
                    .cast::<u32>();
                zero_by_band[band as usize] =
                    counts.row(std::slice::from_raw_parts(line, self.width as usize));
            }
            self.buffer
                .Unmap(0, Some(&D3D12_RANGE { Begin: 0, End: 0 }));
        }
        Some((counts, zero_by_band))
    }

    pub(crate) fn report(&self) {
        let Some((counts, zero_by_band)) = self.counts() else {
            reject!("light probe skipped: completed readback could not be mapped");
        };
        let zero_rgb: u32 = zero_by_band.iter().sum();
        let pixels = self.width * self.rows;
        tracing::info!(hash = SHADER, sequence = self.sequence, recorded_frame = self.frame,
            format = "R32_UINT", width = self.width, height = self.height, sampled_rows = self.rows,
            pixels, zero_rgb, nonzero_rgb = pixels - zero_rgb, zero_words = counts.zero_words,
            zero_by_band = ?&zero_by_band[..self.rows as usize], nonzero_channels = ?counts.nonzero_channels,
            exponents = ?counts.exponents, "original temporal resolve light sample");
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn the_probe_requires_an_explicit_one() {
        assert!(!enabled(None));
        assert!(!enabled(Some("0".as_ref())));
        assert!(!enabled(Some("true".as_ref())));
        assert!(enabled(Some("1".as_ref())));
    }

    #[test]
    fn band_centers_are_distinct_and_in_bounds() {
        for height in [1, 2, 7, 8, 9, 1440, 16384] {
            let count = height.min(ROWS);
            let ys: Vec<_> = (0..count).map(|i| sample_row(height, i, count)).collect();
            assert!(ys.windows(2).all(|y| y[0] < y[1]));
            assert!(ys[count as usize - 1] < height);
        }
    }

    #[test]
    fn packed_black_can_have_a_nonzero_exponent() {
        let mut c = Counts::default();
        assert_eq!(c.row(&[0, 31 << 27, 1, 3 << 9, 511 << 18, u32::MAX]), 2);
        assert_eq!(c.zero_words, 1);
        assert_eq!(c.nonzero_channels, [2, 2, 2]);
        assert_eq!(c.exponents[31], 2);
        assert_eq!(c.exponents[0], 4);
    }

    /// A real texture with padded row pitches and four discarded rows. The
    /// completed copy is compared with the source pattern independently.
    pub(crate) fn check_readback(
        device: &ID3D12Device,
        list: &ID3D12GraphicsCommandList,
        wait: impl FnOnce(),
    ) {
        use windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC;
        let width = 3u32;
        let height = 12u32;
        let make = |kind: D3D12_HEAP_TYPE, desc: D3D12_RESOURCE_DESC, state| -> ID3D12Resource {
            let heap = D3D12_HEAP_PROPERTIES {
                Type: kind,
                ..Default::default()
            };
            let mut resource = None;
            // SAFETY: valid dimensions and initial state for the selected heap.
            unsafe {
                device
                    .CreateCommittedResource(
                        &heap,
                        D3D12_HEAP_FLAG_NONE,
                        &desc,
                        state,
                        None,
                        &mut resource,
                    )
                    .unwrap()
            };
            resource.unwrap()
        };
        let samples = DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        };
        let texture = make(
            D3D12_HEAP_TYPE_DEFAULT,
            D3D12_RESOURCE_DESC {
                Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
                Width: u64::from(width),
                Height: height,
                MipLevels: 1,
                DepthOrArraySize: 1,
                SampleDesc: samples,
                Format: DXGI_FORMAT_R32_UINT,
                Flags: D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS,
                ..Default::default()
            },
            D3D12_RESOURCE_STATE_COPY_DEST,
        );
        let buffer = |bytes| D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
            Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
            Width: bytes,
            Height: 1,
            MipLevels: 1,
            DepthOrArraySize: 1,
            SampleDesc: samples,
            ..Default::default()
        };
        let upload = make(
            D3D12_HEAP_TYPE_UPLOAD,
            buffer(u64::from(256 * height)),
            D3D12_RESOURCE_STATE_GENERIC_READ,
        );
        let back = make(
            D3D12_HEAP_TYPE_READBACK,
            buffer(u64::from(512 * ROWS)),
            D3D12_RESOURCE_STATE_COPY_DEST,
        );
        let sample = Readback {
            buffer: back,
            width,
            height,
            rows: ROWS,
            stride: 512,
            frame: 0,
            sequence: 1,
        };
        // R, G and B differ by row. Column one has noncanonical black.
        let pattern = |y: u32| [0, (y + 1) << 27, ((y + 1) << 27) | (1 << ((y % 3) * 9))];
        // SAFETY: upload is large enough for every pitched row. All resources
        // live through the queued work and the test waits before reading.
        unsafe {
            let mut data = std::ptr::null_mut();
            upload.Map(0, None, Some(&mut data)).unwrap();
            for y in 0..height {
                let dst = data.cast::<u8>().add((y * 256) as usize);
                std::ptr::copy_nonoverlapping(
                    pattern(y).as_ptr().cast::<u8>(),
                    dst,
                    width as usize * 4,
                );
            }
            upload.Unmap(0, None);
            let from = D3D12_TEXTURE_COPY_LOCATION {
                pResource: std::mem::transmute_copy(&upload),
                Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
                        Offset: 0,
                        Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                            Format: DXGI_FORMAT_R32_UINT,
                            Width: width,
                            Height: height,
                            Depth: 1,
                            RowPitch: 256,
                        },
                    },
                },
            };
            let to = D3D12_TEXTURE_COPY_LOCATION {
                pResource: std::mem::transmute_copy(&texture),
                Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    SubresourceIndex: 0,
                },
            };
            list.CopyTextureRegion(&to, 0, 0, 0, &from, None);
            let uav = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
            list.ResourceBarrier(&[transition(&texture, D3D12_RESOURCE_STATE_COPY_DEST, uav)]);
            sample.copy_from(list, &texture);
            list.ResourceBarrier(&[transition(&texture, uav, D3D12_RESOURCE_STATE_COPY_SOURCE)]);
        }
        let (original, placed) = crate::capture::read_back(device, list, &texture).unwrap();
        wait();
        let (counts, black) = sample.counts().unwrap();
        assert_eq!(black, [2; ROWS as usize]);
        assert_eq!(counts.zero_words, ROWS);
        // Centers of the 8 equal-height bands across this 12-row texture.
        let expected_rows = [0, 2, 3, 5, 6, 8, 9, 11];
        let mut exponents = [0; 32];
        exponents[0] = ROWS;
        for y in expected_rows {
            exponents[y + 1] += 2;
        }
        assert_eq!(counts.exponents, exponents);
        assert_eq!(counts.nonzero_channels, [4, 0, 4]);
        // SAFETY: the copy of all twelve rows has completed. Read only the
        // three valid words per row, including the rows the probe skipped.
        unsafe {
            let mut data = std::ptr::null_mut();
            original.Map(0, None, Some(&mut data)).unwrap();
            for y in 0..height {
                let offset = placed.Offset as usize + (y * placed.Footprint.RowPitch) as usize;
                assert_eq!(
                    data.cast::<u8>().add(offset).cast::<[u32; 3]>().read(),
                    pattern(y)
                );
            }
            original.Unmap(0, Some(&D3D12_RANGE { Begin: 0, End: 0 }));
        }
    }
}
