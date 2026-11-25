# Descriptor page capacity on the RTX 5080

Tested with an NVIDIA GeForce RTX 5080 16 GB, driver 617.14, Ryzen 9 5900X, Windows 11 Pro 25H2 build 26200.9974, Rust 1.98.1 MSVC and DXIL 1.9.2609.5. Both launches used diagnostics, DRED, the Series X profile, the same relay and inline staging settings, and the same timed input sequence.

The control was NVIDIA fork PR 688 head `7e7bb180625298dd418cb7ce71923140363ff02f`, with the previously measured integer-blend and descriptor-read fixes. The candidate adapts [VIctory PR 2121](https://github.com/Marvisak/VIctory/pull/2121) at `907922cf645282afbc019c91435488053aebdcac` to the fork's heap and draw implementation. It gives the page table its own root SRV so that a page lookup does not trigger the fork's stride-table buffer refresh. `VICTORY_VIEW_PAGES=1` enabled it on the capped adapter.

## What fit

The title created a 1,245,184-slot virtual view heap backed by one 1,000,000-slot physical heap. At 256 slots per page that provides 3,906 whole physical pages. The runtime reserved 1,026 for dynamic tables and null/scratch access and had 2,880 for the title. It reported 2,048 pages in use 41.0 seconds after player selection, then filled all 3,906 pages and reported exhaustion 150.5 seconds after selection, during world loading. Newly requested pages had no physical slot and read null from that point. This records the capacity reached. It does not count title pages requested after exhaustion.

The port's two host-heap fixtures passed. One checked an allocation above the millionth virtual slot and contiguous dynamic slots crossing a page boundary. Both previously recovered pixel shaders also validated and created hardware test pipelines with paging enabled, and the live runtime accepted both mesh/pixel pairs.

## Five-minute prison replay

The candidate reached player control and responded to the replay. The prison still showed missing wall and ceiling surfaces. The post-retreat view also showed darker clothing and fewer visible interior surfaces. The camera angles and rendering progress differed slightly, so this comparison is qualitative.

| Displayed frames and timing | Control | Local paging port |
| --- | ---: | ---: |
| Displayed application frames | 1,364 | 1,204 |
| Average presented FPS | 4.543 | 4.007 |
| P99 display interval | 435.072 ms | 658.550 ms |
| Worst display interval | 535.089 ms | 1,358.229 ms |
| First 150 seconds | 4.50 FPS | 3.78 FPS |
| Last 150 seconds | 4.58 FPS | 4.23 FPS |
| Host samples with pending pipelines | 0 of 53 | 10 of 54 |

Both runs began with zero reported startup shader-cache hits. The page-table port was still compiling pipelines during the early part of the replay, so the first-run timing includes that work. Performance also remained lower in the later half of this run. The benchmark ended deliberately with no DRED device removal logged. Further title runs would be needed to separate shader warm-up, descriptor capacity and steady-state costs.

[Previous descriptor-read comparison](https://github.com/iGoldenNinja/victory-fork/blob/b2ab0beee1d7a6aade0d9184c6647afc5868674e/docs/ssd-pr203-descriptors-5080.md)

OpenAI Codex (GPT-6) adapted the branch, ran the tests and drafted this report.
