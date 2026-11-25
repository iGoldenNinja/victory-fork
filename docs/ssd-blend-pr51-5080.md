# NVIDIA integer render target blend fix on the RTX 5080

Tested Windows 11 Pro build 26200, Ryzen 9 5900X, 32 GB RAM and GeForce RTX 5080 16 GB, driver 617.14. The candidate adds the integer-target blend change from NVIDIA PR 51 (`d82acefa8e3005a4293f327b4099b6c547602d9e`) to the earlier diagnostic PR 688 build (`7e7bb180625298dd418cb7ce71923140363ff02f`). The saved Series X profile, SATA SSD package, warmed cache, `VICTORY_RELAY=1`, `VICTORY_CARD_INLINE_STAGE=1` and DRED matched the previous staging-on run. Both builds included the same local archive and replay helpers. These were separate game launches.

## Pipeline and visibility

The earlier build logged `0x80070057` for mesh `f5e50eb2108c425b` and pixel `ce3f0432a08f1d35`. The patched run logged the successful host mesh pipeline for that exact pair, with no refusal of the pair. That reproduces the pipeline fix on the RTX 5080.

The matched interior captures still showed major wall and ceiling gaps with the blend change applied. The two images were taken at the same camera angle after the first backward input, about 58 seconds into each timed route. The previously rejected mesh pipeline now loads, and the broader geometry problem remains open.

## Five-minute gameplay timing

| Prison capture | PR 688, staging on | Same build plus PR 51 blend fix |
| --- | ---: | ---: |
| Displayed application frames | 1,375 | 1,371 |
| Presented FPS | 4.579 | 4.565 |
| P99 display interval | 393.971 ms | 435.139 ms |
| Worst display interval | 876.101 ms | 711.421 ms |
| Mean PresentMon GPU busy per interval | 140.34 ms | 139.90 ms |
| Median reported card-copy wait | 34.10 ms | 35.20 ms |
| Median host-reported missing-pipeline drops per frame | 319 | 318 |

The run reached player control and responded to the replayed camera and movement sequence. DRED reported no device removal and the benchmark ended deliberately. This is one run per build and the measured ranges remain far from the 30 FPS gameplay target.

[NVIDIA PR 51](https://github.com/fxceless/victory-fork/pull/51) · [Earlier PR 688 staging comparison](https://github.com/iGoldenNinja/victory-fork/blob/a15f3c6c513f9075fdd08d73d4d7502bdb6db157/docs/ssd-readback-pr688-5080.md)

Testing and reporting assisted by OpenAI Codex.
