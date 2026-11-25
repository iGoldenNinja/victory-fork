# NVIDIA fork f85101b default replay on the RTX 5080

Tested `f85101b6095e8cf3cecf8f0647cc7f5428daa177` against its parent `9376fd60c0ba0e76a74c39b49b1407dbac0a64c7` on an RTX 5080 16 GB, driver 617.14, Windows 11 Pro 25H2 build 26200.9974, Ryzen 9 5900X and 32 GB RAM. Both builds used the same local archive and replay support. [NVIDIA fork commit](https://github.com/fxceless/victory-fork/commit/f85101b6095e8cf3cecf8f0647cc7f5428daa177)

I reused an earlier parent replay and drove the same inputs on the new build. Both runs used the Series X profile, byte-matched copies of the same 3,067 cache entries and the same starting pipeline library. Both logged zero shader-cache misses. The first pan matched within 7 ms of player selection and later input offsets within 15 ms of the first pan. Async submission, the dedicated copy queue and indirect draw windows were all left at defaults.

| 90-second prison replay | Parent | New default |
| --- | ---: | ---: |
| Displayed application frames | 322 | 349 |
| Displayed FPS | 3.56 | 3.86 |
| P99 displayed frame time | 470.38 ms | 376.31 ms |
| Worst displayed frame time | 658.55 ms | 446.90 ms |
| Mean GPU busy per displayed frame | 202.79 ms | 195.54 ms |
| Median card fetch wait in host telemetry | 136.25 ms | 131.20 ms |
| Median copy-back in host telemetry | 175.50 ms | 170.90 ms |
| Median reported draws | 5,770 | 5,845 |

Both reached controllable prison, with the same eight distinct pipeline refusals (`E_INVALIDARG`) and zero logged device removals. The new build still showed a blocky band below the courtyard window. The matched post-retreat view showed similar lighting and room geometry, with a stepped character silhouette.

The timings describe one run per revision, in fixed order. They measure the defaults together. Individual switch effects and run-to-run variance remain to be measured.

## Dedicated copy-queue fetches

I repeated the route on the identical `f85101b` executable with `VICTORY_QUEUE2=1`, using a fresh verified copy of the same cache seed. The earlier `f85101b` default arm ran first. Both had 2,124 startup shader-cache hits and zero misses at the first tally. The effective relay mode was 2 in both. Async submission, indirect draw windows and the earlier in-flight fetch switch were all left at defaults. The first pan matched within 90 ms from player selection and later inputs within 26 ms from the pan.

| 90-second prison replay | Default | Dedicated copy queue |
| --- | ---: | ---: |
| Displayed application frames | 349 | 309 |
| Displayed FPS | 3.86 | 3.41 |
| P99 displayed frame time | 376.31 ms | 540.96 ms |
| Mean GPU busy per displayed frame | 195.54 ms | 230.36 ms |
| Median relay fetch wait | 131.20 ms | 11.00 ms |
| Median copy-back in host telemetry | 170.90 ms | 79.60 ms |
| Median reported draws per frame | 5,845 | 5,901 |

The queue reported a median 14.5 readback jobs transferring 74.9 MiB per frame over 77 samples. Upload jobs on the copy queue stayed at zero. The relay also reported a median 59.8 pieces per frame fetched again at their own landing. Both runs reached controllable prison and showed the same eight distinct pipeline refusals, with zero logged device removals. The blocky band below the courtyard window and stepped character edges appeared in both settings. The post-retreat frames showed similar room geometry and lighting.

The copy-queue run had much shorter fetch waits and copy-back time, alongside higher GPU busy time and worse displayed frame pacing. Each setting ran once, in fixed order.
