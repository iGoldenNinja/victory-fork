# NVIDIA fork PR 688 on the RTX 5080

Windows 11 Pro build 26200, Ryzen 9 5900X, 32 GB RAM, GeForce RTX 5080 16 GB, driver 617.14, Series X profile, SATA SSD and DRED enabled. The diagnostic builds used the same local archive reader and scripted gamepad replay. The base was `310c6eb6c16822ab95076aad61fce19bba86bb98`. The candidate was PR 688 at `7e7bb180625298dd418cb7ce71923140363ff02f`, with `VICTORY_RELAY=1` and `VICTORY_CARD_INLINE_STAGE=1`.

Each capture covered five minutes of controllable gameplay in the same prison room. The candidate repeated the base's three camera holds and five movement events at the same intervals from the first camera input. Both builds logged zero shader-cache hits during startup. The two runs had separate starts and small scene differences.

| Measurement | Base `310c6eb` | PR 688, staging on |
| --- | ---: | ---: |
| Displayed application frames | 1,264 | 1,375 |
| Presented FPS | 4.209 | 4.579 |
| P99 display interval | 417.475 ms | 393.971 ms |
| Worst display interval | 523.282 ms | 876.101 ms |
| Mean PresentMon GPU busy per interval | 182.73 ms | 140.34 ms |
| Median reported wait for card-copy fetches per frame | 143.30 ms (271 reports) | 34.10 ms (263 reports) |
| Median reported copy-back time per frame | 175.25 ms (54 reports) | 65.35 ms (52 reports) |

Copy-back and PresentMon GPU busy counters can overlap. The patch reduced the readback counters in this run, and the end-to-end FPS gain was smaller. The staged build showed missing wall and ceiling surfaces in the prison at a similar interior angle after retreating. Those surfaces were visible in the base capture. The captures show a visual discrepancy. The same-binary follow-up below tests inline staging separately. DRED reported no device removal and the test ended deliberately. The result missed the 30 FPS gameplay target.

All 13 selected relay, card and comparison tests passed. A test-only rerun required WARP initialization so the two card comparison tests could not silently return. All 13 passed again, including ring overflow and COPY-queue readback. The title benchmark used the compiled PR code with the same local SSD and replay patches as the base.

[PR 688](https://github.com/fxceless/victory-fork/pull/688) · [Base benchmark](https://github.com/iGoldenNinja/victory-fork/blob/87f4d9236ae99eb01426bd96ca03d7c711f8306a/docs/ssd-world-entry.md)

Testing and reporting assisted by OpenAI Codex.

## Same-binary comparison with inline staging off

The staging-off run used the same PR 688 executable (`7e7bb18`), Series X profile, direct SSD archive routing and timed camera/movement sequence. The setting changed from `VICTORY_CARD_INLINE_STAGE=1` to `0` and `VICTORY_RELAY` stayed at `1`. The staging-on run logged zero shader-cache hits during startup. The same binary hit its shader cache on the staging-off run. Both runs were separate launches, one measured run per setting. Both ended deliberately after the captured interval, with no reported DRED device removal.

| Five-minute prison measurement | Staging on | Staging off |
| --- | ---: | ---: |
| Displayed application frames | 1,375 | 1,358 |
| Presented FPS | 4.579 | 4.522 |
| P99 display interval | 393.971 ms | 441.020 ms |
| Worst display interval | 876.101 ms | 1,034.859 ms |
| Mean PresentMon GPU busy per interval | 140.34 ms | 139.56 ms |
| Median reported wait for card-copy fetches | 34.10 ms (263 reports) | 75.25 ms (266 reports) |
| Median reported copy-back | 65.35 ms (52 reports) | 110.90 ms (53 reports) |

Average presented FPS differed by 0.057 between these two runs despite the 41.15 ms difference in median card-copy fetch wait. Missing wall and ceiling surfaces remained visible with staging both on and off. I compared the same interior angle after the first backward move, during the idle interval before the next input. The same mesh/pixel PSO pair returned `0x80070057` on the base and the two PR runs. [Independent ZTRX report on that pipeline pair](https://github.com/Zettrox/ztrx/pull/36#issuecomment-7153365073)
