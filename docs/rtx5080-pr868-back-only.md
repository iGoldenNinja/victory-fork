# PR 868 back-only readback on an RTX 5080

Skipping GPU-to-CPU copies from write-combining allocations substantially reduced readback traffic in the first prison cell. Average displayed FPS improved, and the back-only sample also contained several long pauses.

The Windows build used the same executable for both runs. It combines [PR 868 at 4e34eb0](https://github.com/fxceless/victory-fork/pull/868) with an early-depth candidate using the tiled dispatch split and paired-wave changes. The integration revision was `dc3c191953f0c8e4adfad5260331ddbc6ec21910`. The executable also included the same local archive and input support in both arms. Hardware was an ASUS ROG Astral GeForce RTX 5080 16 GB, driver 617.14, Ryzen 9 5900X, 32 GB RAM and Windows 11 Pro 25H2 build 26200.9974.

Each Series X launch started with a fresh copy of the same profile and warm cache. The census ran in both. GPU comparisons stayed enabled, in-flight fetch stayed off, and only `VICTORY_SKIP_WC_BACK` changed from 0 to 1. Both launches reached the prison. Two reference images taken at least 12 seconds apart confirmed the camera at the cell window before each sample. The 2542 × 1396 client size stayed constant, and no replay input was sent during either sample.

| Measure | Control | Back-only |
| --- | ---: | ---: |
| Displayed application frame intervals | 256 in 90.18 s | 344 in 90.11 s |
| Average displayed FPS | 2.84 | 3.82 |
| P99 display interval | 429.25 ms | 1,217.15 ms |
| Longest display interval | 464.50 ms | 2,628.33 ms |
| Median draws per renderer report | 4,985 | 4,996 |
| Median renderer copy-back time | 199.7 ms | 98.9 ms |
| Census MB fetched per frame | 96.07 | 3.59 |
| Census MB fetched from write-combining allocations per frame | 92.52 | 0.00 |
| Census MB sent to the card per frame | 140.08 | 27.33 |
| Census pieces compared per frame | 16,780 | 16,763 |

PresentMon classified all displayed rows in the samples as application frames. Each census average is frame-weighted over eight complete reports contained in the display interval, covering 229 control frames and 329 back-only frames. Renderer medians use 15 and 14 complete report intervals respectively. The control preceded the treatment.

The before and after images from each arm show the same prison window, wall and character. The character pose and outdoor figures varied. Dark areas remained, and the four images showed no obvious new visual defect. Both runs stopped deliberately after measurement with no logged device removal. The disabled in-flight path keeps the independently reproduced [census correction](https://github.com/iGoldenNinja/victory-fork/commit/49039ced23d922d0e04112b02f0dc690da329d8f) outside this comparison.

The control stayed in PresentMon's hardware-composed independent-flip mode and had zero intervals over 500 ms. The treatment had five consecutive intervals over one second. All five started on frames reported in composed-flip mode on the same swap chain. They ended between 63.47 and 69.67 seconds into the sample. The longest other interval was 335.13 ms. No capture or replay action occurred during those five intervals. The full-sample P99 includes them. For the directly adjacent displayed frame pairs with both endpoints in hardware-composed independent flip, nearest-rank P99 was 429.25 ms (256 control intervals) and 305.77 ms (337 treatment intervals). This pair cannot establish what triggered the change of presentation mode or how much of the tail the readback switch caused.

This single stationary pair establishes a traffic reduction in this scene. The slower tail needs repetition. Neither run traced CPU reads of skipped data, tested later story progression or qualified for the two five-minute 30 FPS goal.

## Lower-frame visual check

An RTX 4070 tester subsequently reported light-grey stepped rectangles in two of three treatment runs and cyan specks in one. We re-examined the existing RTX 5080 pair at `dc3c191` in response. Each process contributed four cell-window images, the two matched camera anchors and the first and last sample of the 90-second measurement. The 4070 tester used a later build and longer observation times. [RTX 4070 follow-up](https://github.com/fxceless/victory-fork/pull/868#issuecomment-7167445429)

We inspected the bottom fifth of each 2542 × 1396 image at its captured resolution, 711,760 pixels per image. All eight crops contained zero light-grey pixels with all RGB channels at least 110 and a channel spread of at most 18. They also contained zero cyan pixels with both green and blue at least 110 and at least 48 above red. With the neutral colour tolerance widened to a channel spread of 24, the brightest minimum channel per image ranged from 82 to 96 with the switch off and from 89 to 98 with it on. These crops give a negative result for the defined colours on this earlier build. Darker defects, different camera views and the later 4070 result remain open.
