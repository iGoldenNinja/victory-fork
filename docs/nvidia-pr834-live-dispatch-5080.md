# Live diffuse-lighting dispatch coverage on RTX 5080

Tested PR 834's `0135c4103158e786641e4eb084c50337cdc7464c` rebase onto `f85101b6095e8cf3cecf8f0647cc7f5428daa177`. Hardware was an RTX 5080 16 GB, driver 617.14, Windows 11 Pro 25H2 build 26200.9974, Ryzen 9 5900X and 32 GB RAM. [Dispatch split PR](https://github.com/fxceless/victory-fork/pull/834)

I added a diagnostic clamp readback that samples every 16th matching single-record, uncounted dispatch. It reads the input and clamped record triplets after the host fence. The selected program was `2dc364df5d3278b8`, the Wave32 diffuse-lighting final gather with an 8 × 4 thread group. The WARP fixture verified the original requested triplet and both output records. The split execution fixture also passed. [Diagnostic source](https://github.com/iGoldenNinja/victory-fork/commit/10a4d935c2e72ba6d1dbab0077a5e6b1930dd5bb)

## Separate prison replays

The same executable ran with split off and on, in that order, with separate copies of the same starting profile and cache. Every sampled request had Y and Z equal to one and requested more than 65,535 groups in X. Both runs reached controllable prison and completed the 90-second replay.

| GPU argument readback, from player selection to stop | Split off | Split on |
| --- | ---: | ---: |
| Samples | 46 | 48 |
| Requested X range | 65,704–96,848 | 65,928–96,112 |
| First record X | 65,535 | 65,535 |
| Second record X range | 0 | 393–30,577 |
| Requests covered in full | 0 of 46 | 48 of 48 |
| Largest omission | 31,313 groups | 0 |

The bottom band of stale image tiles was visible below the window with splitting off and cleared in the comparable split-on view. Stepped edges around the character were visible with splitting on. The captures had client dimensions of 1632 × 909 off and 2542 × 1396 on. The replays measured 2.51 displayed FPS and 1,270.04 ms P99 off, and 3.32 FPS and 505.69 ms P99 on. The different client dimensions and single run per setting limit the timing comparison.

## Fixed-camera repeat

I added a diagnostics-only switch that toggles splitting for the selected shader in one process. The camera stayed at the prison window without replay input during all six captures. Client dimensions stayed at 1735 × 911. The scene continued its idle animation and displayed the movement tutorial during the comparison.

| Setting | Requested X | First record X | Second record X | Lower band in both captures |
| --- | --- | --- | --- | --- |
| Initially off | 74,400 and 74,808 | 65,535 | 0 | Visible |
| Enabled | 75,720 and 75,928 | 65,535 | 10,185 and 10,393 | Mostly cleared |
| Disabled again | 74,880 and 73,432 | 65,535 | 0 | Stayed mostly clear |

Mean encoded luma in the same 320 × 60 region below the window fell from 21.53 and 20.26 with splitting initially off to 10.79 and 6.08 with splitting on. It stayed at 5.34 and 5.38 in the two captures after splitting was disabled again. The clip returned in the GPU records immediately. The visual persistence is consistent with the corrected data remaining in the previously unwritten tiles. A short camera pan with splitting off left the broad band clear in the sampled second view. The stepped character edges remained in the fixed-camera comparison.

To reproduce the diagnostics, build `victory` with `--profile diag --features diagnostics` and set `VICTORY_TRACE_INDIRECT` to the selected compute shader hash. To toggle splitting for only that pass during one launch, set `VICTORY_TRACE_SPLIT_FILE` to a local marker path. A regular file at the path enables the split. The readback log gives the requested triplet and the two clamped records when splitting is enabled.

Testing and reporting assisted by OpenAI Codex.
