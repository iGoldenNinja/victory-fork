# Fresh-process restoration after the prologue

Two new processes loaded independent writable copies of the same `SARC0015` v2 save. Both reached the post-arrest interview sequence. Each started with the same verified 818-file warm cache and renderer binary. The second launch enabled `VICTORY_TEXTURE_NO_WAIT=1`. The Series X profile ran on Windows 11 Pro 25H2 build 26200.9974, an ASUS ROG Astral RTX 5080 16 GB, driver 617.14, Ryzen 9 5900X and 32 GB RAM. The renderer was `92a72df7b310a4e816ce07a155be1420a6537672` with a local archive and controller-replay overlay. [Original story run and save creation](rtx5080-prison-story.md)

The account-picker prompt matched a reference on two consecutive native snapshots before each selection. In both trials the v2 manifest and both payloads in the writable profile acquired new access times while an untouched comparison file retained its baseline timestamp. Both protected save-tree content hashes matched after exit. These timestamps corroborate access to the copies. The rendered images establish where playback resumed.

The first visible narrative image sampled in the control, 191.2 seconds after selection, was a heavily obscured facial close-up. The subsequent shot identified the interview. In the opt-in launch, two officers and a marked patrol car were visible 69.8 seconds after selection. The exterior crowd and the interview followed. Eight-second sampling brackets these observations, and background compiler activity differed during loading. Both measured windows began after that work stopped. The observed sequences occur later in the story than the shop tutorial.

## Interview and subsequent display timing

Both timings start at a close-up of the seated woman's face, identified by the shoulder silhouette, the shelf and the highlights on the wall. Frame intervals use successive distinct PresentMon display timestamps for the application, with nearest-rank P99. Both launches ended deliberately and their per-process OS engine counters confirmed RTX 5080 3D activity.

| Launch | Display intervals | Elapsed | Average FPS | P99 and longest interval |
| --- | ---: | ---: | ---: | ---: |
| Control | 24 | 61.821 s | 0.388 | 9,649.053 ms |
| Texture no-wait | 89 | 60.287 s | 1.476 | 4,492.245 ms |

The camera and visible content diverged after the anchor. At 15 seconds the control showed a vehicle behind a figure and the opt-in launch showed the interviewer. At 30 seconds one showed a white-shirted character beside water and the other showed an aircraft. The mean absolute RGB difference between the two downsampled images rose from 14.8/255 at the anchor to 53.6 at 15 seconds and 77.0 at 30 seconds. The different workloads prevent attributing an FPS gain to the setting. The shop scene was outside both windows.

The renderer's eight complete control reports had a median of 1,504.7 ms/frame waiting for read access to title memory and 1,672.6 ms/frame in all-thread texture refresh. Across nine opt-in reports the corresponding medians were 0.0 and 61.7 ms/frame. Refresh time includes the read-access wait. The opt-in path read through the kernel in all nine reports (0.5–459.0 reads/frame, median 202.5) and deferred missing-page refreshes in seven (0.0–313.0 deferrals/frame, median 53.3). The control recorded zero for both fallback counters. Each value summarizes a per-report rate rounded by the renderer. [Counter implementation](https://github.com/iGoldenNinja/victory-fork/blob/92a72df7b310a4e816ce07a155be1420a6537672/crates/victory-gpu/src/host.rs#L7516-L7531)

## Visible artifacts

In the opt-in launch, a dark face with bright eyes transitioned to visible skin detail across successive pre-anchor images. The interviewer's face later showed colored speckling. The control close-up had filament-like lines extending onto the wall. At 960 × 540, the same 17,120-pixel wall region had 0, 234 and 0 narrow dark-line pixels in the earlier continuous run, the fresh control and the fresh opt-in image respectively. A pixel counted when it was at least eight luma levels darker than both neighbors two pixels away on either axis. Median wall luma was 52, 108 and 95 respectively. Head pose, lighting and scene timing differed, so this single pair cannot identify the cause.
