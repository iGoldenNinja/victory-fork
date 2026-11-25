# NVIDIA fork PR 688 on the RTX 5080

Windows 11 Pro build 26200, Ryzen 9 5900X, 32 GB RAM, GeForce RTX 5080 16 GB, driver 617.14, Series X profile, SATA SSD and DRED enabled. The diagnostic builds used the same local archive reader and scripted gamepad replay. The base was `310c6eb6c16822ab95076aad61fce19bba86bb98`. The candidate was PR 688 at `7e7bb180625298dd418cb7ce71923140363ff02f`, with `VICTORY_RELAY=1` and `VICTORY_CARD_INLINE_STAGE=1`.

Each capture covered five minutes of controllable gameplay in the same prison room. The candidate repeated the base's three camera holds and five movement events at the same intervals from the first camera input. The two runs had warm caches, separate starts and small scene differences.

| Measurement | Base `310c6eb` | PR 688, staging on |
| --- | ---: | ---: |
| Displayed application frames | 1,264 | 1,375 |
| Presented FPS | 4.209 | 4.579 |
| P99 display interval | 417.475 ms | 393.971 ms |
| Worst display interval | 523.282 ms | 876.101 ms |
| Mean PresentMon GPU busy per interval | 182.73 ms | 140.34 ms |
| Median reported wait for card-copy fetches per frame | 143.30 ms (271 reports) | 34.10 ms (263 reports) |
| Median reported copy-back time per frame | 175.25 ms (54 reports) | 65.35 ms (52 reports) |

Copy-back and PresentMon GPU busy counters can overlap. The patch reduced the readback counters in this run, and the end-to-end FPS gain was smaller. The staged build showed missing wall and ceiling surfaces in the prison at a similar interior angle after retreating. Those surfaces were visible in the base capture. The captures show a visual discrepancy. A paired comparison on the same PR binary with staging toggled is needed to isolate it. DRED reported no device removal and the test ended deliberately. The result missed the 30 FPS gameplay target.

All 13 selected relay, card and comparison tests passed. A test-only rerun required WARP initialization so the two card comparison tests could not silently return. All 13 passed again, including ring overflow and COPY-queue readback. The title benchmark used the compiled PR code with the same local SSD and replay patches as the base.

[PR 688](https://github.com/fxceless/victory-fork/pull/688) · [Base benchmark](https://github.com/iGoldenNinja/victory-fork/blob/87f4d9236ae99eb01426bd96ca03d7c711f8306a/docs/ssd-world-entry.md)

Testing and reporting assisted by OpenAI Codex.
