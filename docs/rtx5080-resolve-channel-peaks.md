# Original prison lighting resolve reached 4.40625 on the RTX 5080

In four completed prison-window dispatches, **9,916 of 81,920 sampled texel observations (12.10%) had at least one decoded RGB9E5 channel at or above 0.6**. The largest red, green and blue values were **4.40625, 1.4609375 and 1.2109375**. These measurements supply the mantissas missing from the earlier exponent-only analysis. [Previous results](rtx5080-resolve-exponents.md) · [PR 917 discussion](https://github.com/fxceless/victory-fork/pull/917#issuecomment-7165363545)

| Recorded frame | Nonzero RGB | Texels with any channel ≥0.6 | R / G / B counts ≥0.6 | R / G / B maxima |
| ---: | ---: | ---: | --- | --- |
| 2528 | 6,978 | 2,603 | 2,603 / 1,198 / 1,480 | 4.40625 / 1.2890625 / 1.078125 |
| 2560 | 6,186 | 2,382 | 2,382 / 985 / 1,248 | 3.5234375 / 1.3671875 / 1.1796875 |
| 2592 | 5,720 | 2,174 | 2,174 / 1,060 / 1,511 | 3.9140625 / 1.3203125 / 1.15625 |
| 2624 | 6,185 | 2,757 | 2,757 / 1,158 / 1,593 | 3.8046875 / 1.4609375 / 1.2109375 |
| **Pooled** | **25,069** | **9,916** | **9,916 / 4,401 / 5,832** | **4.40625 / 1.4609375 / 1.2109375** |

Every qualifying texel in these samples had red at or above 0.6. The count of distinct texels meeting the threshold equaled the red-channel count in each frame. The three maxima are independent and may come from different texels. The table pools repeated observations across frames.

PR 917 at `c24462e` multiplies its level by an RGB tint of `(1.0, 0.98, 0.94)` before packing. At the requested level of 0.6, the replacement therefore decodes to `(307, 301, 289) / 512`, or `(0.599609375, 0.587890625, 0.564453125)`. Our common threshold identifies channels brighter than the corresponding replacement components. Additional green or blue values below 0.6 can also exceed their tinted replacement values. The measurements retain no qualifying pixel coordinates or final screen colors, so they cannot distinguish illumination near geometry from colored blocks. A visual comparison of replacement and preservation remains open. [Tint constants](https://github.com/fxceless/victory-fork/blob/c24462e95b02c214f05451249ecf10f623cd29de/crates/victory-gpu/src/ambient.rs#L217) · [Packing shader](https://github.com/fxceless/victory-fork/blob/c24462e95b02c214f05451249ecf10f623cd29de/crates/victory-gpu/src/ambient.rs#L58)

## Measurement

Native Series X profile on 2026-11-25, NVIDIA GeForce RTX 5080 16 GB, driver 617.14, Ryzen 9 5900X, 32 GB RAM and Windows 11 Pro 25H2 build 26200.9974. The renderer used diagnostic revision `8f37cdd893004aef5e1de3e3e9139856994bee2d`, the same host integration as the previous paired run, and a copy of its profile and warm shader cache. The ray builder and ambient substitution were off. The run enabled `VICTORY_PROBE_LIGHT=1` and `VICTORY_PROBE_LIGHT_INPUTS=1`. [Previous paired run](rtx5080-resolve-input-bindings.md)

Two consecutive image matches identified the prison movement-tutorial view. During the following 48.84 seconds, each selected resolve dispatch (`34fb51b8a0312e82`) yielded one completed readback from the original output UAV at root 1, dword 176. Each matched all 23 input binding records by frame, ordinal, submission and list. Every readback copied the centers of eight vertical bands across the 2560 × 1440 `R32_UINT` texture, or 20,480 texels. The endpoint image differed from the preceding run’s endpoint by a mean 3.48 on the 0–255 RGB scale.

The diagnostic reads each 9-bit mantissa and multiplies it by `2^(shared exponent − 24)`. It counts each channel independently and counts a qualifying texel once. Five CPU light-probe tests, three CPU input-binding tests and the WARP readback test passed. The WARP fixture verifies the original source pattern across every row after the readback, including discarded rows and row-pitch padding handling. [Shared-exponent format specification](https://registry.khronos.org/OpenGL/extensions/EXT/EXT_texture_shared_exponent.txt)
