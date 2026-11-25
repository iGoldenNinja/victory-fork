# PR 834 indirect dispatch split on an RTX 5080

Tested NVIDIA fork PR 834 at `4b44212d6ba571092793f44e77dc8cf60297f427` applied to PR 823 at `aaf658a40f3e512550165e489a0f0c411e3c9544`. The cherry-pick applied without conflicts and produced tree `afcc76c77d28f8b1b6ff69e341e3170499598e1d`. Both title launches used the same local archive and replay support as the earlier PR 823 check. The machine has an RTX 5080 16 GB, driver 617.14, Windows 11 Pro 25H2 build 26200.9974, Ryzen 9 5900X and 32 GB RAM. [PR 834](https://github.com/fxceless/victory-fork/pull/834) · [PR 823 replay comparison](https://github.com/iGoldenNinja/victory-fork/blob/b85ccb58d6eeff9742a0a0b17c5509a8fef99952/docs/nvidia-pr823-5080.md#same-build-allowlist-comparison)

## Synthetic validation

The split-dispatch fixture passed on the RTX 5080 and WARP after making device creation mandatory in the fixture and explicitly selecting the RTX 5080 for the hardware run. An 89,968-group request hit every group exactly once with splitting and stopped at 65,535 without it. The fixture also checked the 131,070-group cap, the requested y/z dimensions, and rejection when the split exceeds the dispatch budget. The shader fixture passed DXIL validation and RTX 5080 readback for group bases 0 and 65,535. The hardware selection and strict checks were confined to the tests.

## Matched prison replays

I ran the same executable with `VICTORY_DISPATCH_SPLIT=0` and then with the switch unset. Both used the Series X profile, identical copies of the same 3,067 cache entries and the same starting pipeline library. Both used PR 823's default Wave32 allowlist. Both created and bound `2dc364df5d3278b8` and the four programs restored by PR 823. Both logged the same four distinct `E_INVALIDARG` pipeline refusals. The first camera pan matched within 16 ms relative to player selection. The largest later discrepancy was 129 ms relative to the first pan.

| 90-second gameplay window | Split off | Split on |
| --- | ---: | ---: |
| Displayed application frames | 275 | 265 |
| Displayed FPS | 3.04 | 2.93 |
| Displayed frame-time P99 | 599.79 ms | 611.48 ms |
| Mean GPU busy per displayed frame | 206.51 ms | 209.80 ms |
| Median reported draws per frame | 6,196 | 6,027.5 |
| Median reported copy-back time | 166.4 ms | 183.5 ms |

The title reached controllable prison in both arms and finished both routes without logged device removal. The off run spent longer in the white transition, so the two initial wall-clock captures were at different stages. In the visually matched courtyard view, both snapshots were free of the reported stale lower-right blocks. Both interiors already appeared dark in that view. After retreat, comparable room frames had mean encoded luma of 20.78 off and 20.60 on (Rec.709-weighted RGB on a 0–255 scale). Stepped edges and shading artifacts remained.

This is one 90-second replay per setting, in fixed order. I verified that the affected compute pipeline was bound. Submitted group dimensions and how often the second execution carried nonzero work remain unmeasured in the replay. The visual observation applies to the matched frames I sampled.

