# RTX 5080 buffer hashing at the boat helm and in the prison cell

Wide hashing reduced the stationary GPU time for storing and comparing host-buffer hashes from 48.0–48.2 ms per frame to 6.4–6.5 ms. Stationary displayed FPS rose by 20.3% in the first pair and 23.7% in the reverse pair. Under throttle the gains were 24.5% and 15.8%. Moving P99 improved in the first pair and worsened in the reverse pair. The cloudy, smooth-water appearance in the first wide trial did not repeat in the second wide trial. The normal gameplay launcher continues to use the verified water repair.

A separate prison-cell pair increased displayed FPS from 3.204 to 3.696 (+15.4%). The same character, barred window, wall and bunk were visible in both runs. The results support the reduction in GPU hash cost in two gameplay scenes. Moving frame-time tails and the occasional smooth-water state still need controlled repeats before a default switch.

## Boat measurements

| Run and mode | Helm FPS | Helm P50 / P95 / P99 (ms) | Helm GPU hash + compare (ms) | Moving FPS | Moving P50 / P95 / P99 (ms) | Moving GPU hash + compare (ms) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 · serial | 3.995 | 247.0 / 276.4 / 299.9 | 48.0 | 3.299 | 276.3 / 329.3 / 981.9 | 50.5 |
| 2 · wide | 4.806 | 205.8 / 235.1 / 264.6 | 6.5 | 4.107 | 229.3 / 276.4 / 670.3 | 6.8 |
| 3 · wide | 4.921 | 199.9 / 235.2 / 252.9 | 6.4 | 4.276 | 217.6 / 270.5 / 682.1 | 6.6 |
| 4 · serial | 3.977 | 247.0 / 288.1 / 299.9 | 48.2 | 3.693 | 264.9 / 305.8 / 329.3 | 49.2 |

The percentiles are intervals between fresh displayed application frames. GPU columns report the median of the renderer’s periodic per-frame GPU timing summaries. In time order, the stationary windows contained 481, 578, 592 and 479 displayed frames. The moving windows contained 150, 186, 194 and 168. A few long frames have a large effect on each moving P99.

| Run | Compared pieces | Fetched pieces | Draws / missing pipeline / dropped indirect | Other GPU lists (ms) | Water detail, left / right | Sky detail (luma) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 17,826 | 706 | 3869 / 77 / 0 | 104.6 | 2.28% / 2.26% | 0.17 |
| 2 | 17,826 | 700 | 3848 / 78 / 0 | 106.7 | 0.46% / 0.75% | 2.73 |
| 3 | 17,826 | 710 | 3779.5 / 75.5 / 0 | 104.5 | 1.57% / 1.65% | 0.17 |
| 4 | 17,826 | 707 | 3885 / 76 / 0 | 105.7 | 2.34% / 2.24% | 0.17 |

Piece and draw counts are medians from the stationary windows. Fetched pieces stayed within 700–710 per frame. Missing pipelines persisted in both modes. In the reverse pair, the submitting thread’s median wait for the GPU was 127.3 ms serial and 80.4 ms wide. During those same windows the GPU had no queued work for 90.2 and 85.4 ms per frame. These measurements overlap across threads and cannot be added as a frame budget.

## What the boat rendered

The boat stayed afloat and left wakes in all four replays. In the 43 stationary samples the original black rectangles did not visibly return. A dark-run detector found zero pixels in both fixed water regions at 50% of local median luminance in every sample. At 65% it flagged up to 297 pixels on the left in the cloudy trial. The other three trials had zero at both thresholds.

The first wide trial had clouds, broad smooth bands on the water and a concentrated reflection of the sun. Both serial trials and the second wide trial had clear skies and ripples. The water-detail metric in the table is the mean absolute luminance difference between neighboring pixels divided by the region’s median luminance. Sky detail is the mean absolute residual from a five-pixel Gaussian blur in a fixed sky region, in 8-bit luma units at 640 × 352. Each reported value is the median over that trial’s stationary images. The clear-sky wide trial also had finer-scale differences from the serial trials. Weather and illumination remain potential confounders.

By the end of each stationary window the renderer had bound the same 238 shader identities with labels related to water, waves, foam, clouds or sky. Only the cloudy trial had also bound one additional foam vertex/pixel pair. These are cumulative first-bind events. They do not establish that a particular dispatch ran or what values a draw sampled. The cause of the occasional smooth surface remains open.

## Prison cell

Two more launches used private copies of the original prison save and settings, the same 2,495-file warm cache and the same executable as the boat trials. The camera was behind Lucia looking through the barred window. Three consecutive image matches on the character and window preceded 24 seconds of settling and a 120-second idle sample. Serial ran first. Both modes decoded and bound the repaired water program. The protected saves and settings verified after each run.

| Mode | Displayed frames | FPS | P50 / P95 / P99 (ms) | GPU hash + compare (ms) | Other GPU lists (ms) | Pieces compared / fetched | Draws / missing pipeline / dropped indirect |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Serial | 386 | 3.204 | 311.6 / 341.0 / 358.7 | 50.1 | 141.1 | 16,801 / 1,464 | 4,910 / 339 / 0 |
| Wide | 445 | 3.696 | 270.5 / 299.9 / 323.4 | 8.6 | 142.8 | 16,795 / 1,483 | 4,915 / 345 / 0 |

Ten stationary images per mode showed the same sunlit prison yard, barred window and pillar, nearby bunk, and Lucia’s uniform and hair. At a 320 × 176 comparison resolution, the per-pixel median images differed by 0.61/255 mean absolute RGB in the window and pillar, 0.18 on the right wall and bunk, and 0.20 on the left wall. The character region differed by 2.41/255. Splitting each mode’s own captures into even and odd groups gave 5.30/255 and 6.54/255 in that region as the character idled. These samples cover one camera and one pair. The close-up eyes, mouth timing, other rooms and normal simulation speed need their own checks. Each prison launch also had only the informational Display 4127 event in the NVIDIA and Display system logs.

On another RTX 5080, wildheathen independently observed cheaper GPU hash and compare passes with driver 621.24 and a different streaming stack. They describe the smaller whole-frame difference in their prologue runs as within scene variation. Their result and the fixed-camera measurements here cover different workloads. [Independent A/B/A/B report](https://github.com/fxceless/victory-fork/pull/1014#issuecomment-7174823244)

## Changed-piece validation

Both exact comparison kernels ran on the physical RTX 5080. Each matched 2,412 CPU bytewise decisions across updates and restores using 67 pieces and a nonzero flag-buffer offset. The aligned interior range had 449 changed flags out of 1,206. The range with clipped end pieces and unaligned internal boundaries had 610 out of 1,206. The reference includes the renderer’s documented 16-byte load rounding at shared boundaries. Output guard words held throughout. Two WARP tests also passed, including 52 individual mutations and 52 restorations per mode with neighbor-flag checks and a mutation at the true final byte. These tests exercise the synthetic buffers supplied to the kernels. The individual live-game fetch decisions were not replayed byte for byte. [Native and WARP tests](https://github.com/iGoldenNinja/victory-fork/blob/d17bf6e148c3d0780cb522e202cc915ce5f5d539/crates/victory-gpu/src/indirect.rs) · [Related unaligned-boundary coverage in PR 992](https://github.com/fxceless/victory-fork/pull/992)

To repeat the native fixture on Windows, run `cargo test -p victory-gpu --features diagnostics hardware_card_hashes_match_bytewise_piece_decisions -- --ignored --nocapture --test-threads=1`. The test reports the physical adapter name. The two software checks can be selected with `cargo test -p victory-gpu --features diagnostics warp_card_compare -- --nocapture --test-threads=1`.

## Test setup

Martin Hartfelder's change assigns 64 GPU threads to each 64 KiB piece of a host buffer. The previous implementation assigns one thread to each piece. The renderer chooses one hash algorithm per process for both storing and comparing its piece hashes. This test changes only `VICTORY_CARD_HASH_WIDE` between fresh launches of the same executable. [Original PR and source](https://github.com/fxceless/victory-fork/pull/1014)

The machine used an ASUS ROG Astral GeForce RTX 5080, NVIDIA 617.14, Windows 11 Pro 25H2 build 26200.9974, a Ryzen 9 5900X and 32 GB of RAM. The scene was the Series X “Follow the plane” boat helm. The renderer was `a0bb6f97b0d4141d17cf0e637bc3494082f3ad67`, with upstream `55572246864888974da5bf8f418a092e73d66932` integrated on the repaired-water gameplay base `13b5c65`. The instrumented executable also had the same local archive-access, input and lifecycle support across launches. The comparison-pipeline creation log identified the mode in use. [Renderer revision](https://github.com/iGoldenNinja/victory-fork/commit/a0bb6f97b0d4141d17cf0e637bc3494082f3ad67)

Each launch used its own copy of the same nine-file save and the same 2,495-file warm cache. Both flotation copy-backs and the per-tile allocation repair were enabled throughout. Each process decoded the repaired cache entry, compiled the compute pipeline and bound it. The title window was 2542 × 1396. The benchmark used displayed application frames from PresentMon and the renderer's GPU timestamps. No GPU resource captures ran during the timing windows. [Water and flotation measurements](https://github.com/iGoldenNinja/victory-fork/blob/d380bef4e1f873c0c488af7dc38a6b6c5d6be021/docs/rtx5080-water-depth-input.md#independent-continuous-gameplay-replay)

The sequence was serial, wide, wide, serial. The stationary window was 120 seconds. The moving window covered the first 45 seconds of a 52-second throttle command. In the first wide trial the camera was confirmed from the hull and bridge after the full-color scene matcher missed it. Its stationary window started later and used the original route sampling cadence. The reverse pair used identical collection code, requiring three consecutive hull and bridge matches of at least 0.95 correlation, then a 24-second settling interval. Weather and time of day were not pinned.

All four sessions closed on request and the protected save verified after each. The Windows System log showed one informational Display 4127 Auto HDR suggestion per launch and no NVIDIA driver reset in those intervals. Boat flotation, wakes, the fixed helm and the short throttle segment were checked. Mission completion, dialogue synchronization and simulation speed were outside these timing windows.
