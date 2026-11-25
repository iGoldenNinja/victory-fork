# RTX 5080 story progression and frame pacing from the prison to the shop

The Series X profile reached the prison-window movement tutorial on Windows 11 Pro 25H2 build 26200.9974, an ASUS ROG Astral GeForce RTX 5080 16 GB, NVIDIA driver 617.14, Ryzen 9 5900X and 32 GB RAM. The renderer was `92a72df7b310a4e816ce07a155be1420a6537672` with a local diagnostics-only archive and controller-replay overlay. The ray builder, ambient override and original-light GPU readback were off. Inputs targeted the running title. [Renderer revision](https://github.com/iGoldenNinja/victory-fork/commit/92a72df7b310a4e816ce07a155be1420a6537672)

Camera and movement controls turned the player away from the window and toward an occupied table. The player spoke with Yolanda, received the objective to accompany the guard, and walked through the opened A102 gate and the populated hall. Entering the officer's open side door near A107 triggered an interview scene with the player seated across from prison staff. Lighting and character appearance changed with the scene and camera. Dark, patchy figures remained visible in the hall. The RTX 5070 Ti tester's earlier inspection and zoom observations helped identify the contextual trigger controls. [Earlier inspection report](https://github.com/Marvisak/VIctory/issues/49#issuecomment-7164740824)

A shop cinematic and a playable robbery tutorial followed. Retrying the checkpoint from the pause menu returned to the robbery in this same launch. Holding LT brought up a reticle and a right-stick instruction. Aiming at the clerk advanced the tutorial to an X-button prompt, then to a cash objective. The Y-button interaction triggered a struggle with another X-button prompt. After a load, the run showed Lucia and police officers at a patrol car outside the shop.

The prologue completed in this launch. At 14:48:34 UTC, the pause menu identified the beach scene as Chapter 1, *Gone Fishing*. An earlier, briefly streamed character and HUD did not identify the mission. At 14:44:42 UTC the title had committed the `SARC0015` v2 save with two payloads of 256,736 and 296 bytes. We verified the private checkpoint copy against the complete save-tree hashes before and after copying. Fresh-process restoration from that save is a separate test.

Review of an earlier local run on `c96b4f2b8365bf43d54ab074940923ef8267f4c6` also confirms the guard objective, open dorm door and corridor up to a barred gate. That run's replay recorded camera and movement commands and provides no evidence about how the intervening dialogue advanced. The new run documents the conversation directly. [Earlier revision](https://github.com/fxceless/victory-fork/commit/c96b4f2b8365bf43d54ab074940923ef8267f4c6)

## Two five-minute windows in the same launch

The room was confirmed visually before marking the start against the Windows performance counter. The first window includes camera turns, walking, contextual interaction and the cellmate dialogue. The second starts at a dialogue choice and continues into the guard route. Both exclude the account picker, loading and opening cutscenes. PresentMon 2.6.0 recorded this process and swap chain. We measured successive distinct display timestamps from CPU start plus display latency, beginning with the first displayed event after each marker and ending at the first display at least 300 seconds later. P99 is the nearest-rank 99th percentile of those intervals.

| Start marker | Elapsed | Displayed application frames | Intervals | Average FPS | P99 interval | Longest interval |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Prison window | 300.293 s | 769 | 768 | 2.558 | 893.734 ms | 1,358.243 ms |
| Conversation choice | 300.257 s | 774 | 773 | 2.574 | 923.116 ms | 2,222.620 ms |

The measured windows were 1.85 seconds apart. Every interval exceeded 40 ms. These counts measure displayed application frames. Whether each display contained newly rendered scene content remains unverified. The project requires two independent runs at 29.5 FPS or faster with P99 at or below 40 ms.

## Audio

The Windows audio meter found the title's session in all 5,864 polls during the first marked 300 seconds. Every peak was zero. All 293 session-control polls showed an active, unmuted session at unity gain. A positive control sent a quiet two-second tone from a separate process to the same muted endpoint. The same meter detected the control in 41 polls, peaking at 0.015624 against an expected 0.015625. All 40 concurrent title-session polls read zero.

Later, a capture restricted to the title's process tree measured audio during the cinematic inside a shop. In a 15-second window beginning approximately four seconds after the shop was visually confirmed, the 48 kHz stereo PCM peaked at 0.307 full scale with RMS 0.046 (−26.7 dBFS). Signal was present in all 15 one-second bins while all 294 title-session meter polls read zero. The earlier prison conversation had only session-meter coverage. Its audible content and synchronization remain unverified.

## Renderer workload on the route

We also compared periodic renderer summaries wholly contained within two 45-second intervals, one beginning at the prison window and the other at the crossing into the populated hall. The values below are medians of the per-period averages. The camera, nearby characters and visible environment changed along the route.

| 45-second interval | Reports | Frame time | Draws/frame | Draws dropped for missing pipelines/frame | Copy-back per frame | Host-reported GPU time |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Starting at the dorm window | 7 | 356.7 ms | 4,992 | 379 | 71.2 MB in 187.2 ms | 166.9 ms |
| Crossing the first barred gate into the populated hall | 8 | 390.6 ms | 6,105 | 520 | 98.3 MB in 239.4 ms | 149.8 ms |

The first minute after confirming the shop cinematic showed long waits for read access to title memory during original-texture refresh. Each of its eight complete renderer reports averaged over one second per frame and over 300 ms waiting for read access. The corresponding counts at the prison window were zero across ten complete reports. The values below are medians across those reporting spans. The refresh timer includes the read-lock wait and covers work on all participating threads. [Timer implementation](https://github.com/iGoldenNinja/victory-fork/blob/92a72df7b310a4e816ce07a155be1420a6537672/crates/victory-gpu/src/texture.rs#L384-L390)

| First 60 seconds from marker | Reports | Frame time | Original-texture refresh | Remap read-lock wait | Host-reported GPU time | Estimated queue busy |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Prison window | 10 | 387.5 ms | 23.6 ms | 1.2 ms | 167.1 ms | 57% |
| Shop cinematic | 8 | 2,830.5 ms | 1,458.9 ms | 1,280.2 ms | 120.2 ms | 6.5% |

PresentMon measured 2.366 FPS over 142 display intervals in 60.016 seconds at the prison window (P99 781.937 ms). At the shop it measured 0.426 FPS over 26 intervals in 60.998 seconds (P99 5,865.505 ms). With 26 intervals the nearest-rank P99 is the maximum.
