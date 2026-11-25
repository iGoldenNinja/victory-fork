# In-flight card readback on the RTX 5080

I compared the in-flight readback option integrated into NVIDIA fork main `9376fd60c0ba0e76a74c39b49b1407dbac0a64c7`. Both arms use the same diagnostics executable, the Series X profile, default second-stage relay, DRED and the same archive and replay support. Hardware was an RTX 5080 16 GB with NVIDIA driver 617.14, Ryzen 9 5900X and 32 GB RAM on Windows 11 Pro 25H2 build 26200.9974. Both arms were offered the same 3840 × 2160 display mode. [Integrated change](https://github.com/fxceless/victory-fork/commit/9376fd60c0ba0e76a74c39b49b1407dbac0a64c7) · [PR 648](https://github.com/fxceless/victory-fork/pull/648)

I ran the default arm first and enabled `VICTORY_CARD_FETCH_INFLIGHT=1` for the second. Both arms started with separate byte-matched copies of the same 3,067 warm shader and host pipeline cache files. The measurement uses a 90-second prison route with the same camera and two movement inputs in each arm. Two five-minute runs are still required for the playability benchmark.

Both runs accepted camera and movement input. At the comparable angle after the first retreat, the ceiling lights, walls and bunks rendered in both runs. Both views still showed a sharp lighting boundary on the back wall and stepped edges on the character's shoulder.

| 90-second gameplay measurement | Default | In-flight |
| --- | ---: | ---: |
| Displayed application frames | 322 | 229 |
| Displayed application FPS | 3.56 | 2.53 |
| P99 displayed frame time | 470.38 ms | 676.21 ms |
| Worst displayed frame time | 658.55 ms | 1,128.91 ms |
| Mean GPU busy per displayed frame | 202.79 ms | 333.06 ms |
| Median reported card-copy fetch wait per frame | 136.25 ms | 0.00 ms |
| Median reported host copy-back time per frame | 175.50 ms | 61.70 ms |
| Median relay-reported draws per frame | 5,770 | 5,642.5 |

The option removed the median reported fetch wait and cut the reported copy-back time in this route. Displayed FPS fell by 29%. The enabled arm reported a median of 1,180 pieces and 73.1 MiB of in-flight readback per frame across 76 gameplay samples, with zero reported fallbacks or overflows in that interval. One earlier cutscene interval logged both. Neither run logged device removal.

This is one pair in fixed order. The measurements establish what happened in these two runs. Identifying which work accounted for the additional GPU busy time would require a separate diagnostic.
