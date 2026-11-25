# Dark water patches at the helm on RTX 5080

The follow-up replay measured depth at the same 495 sites and identified which shader store wrote black. [Native depth and store results](rtx5080-water-depth-input.md)

On a native RTX 5080 at the stationary boat helm, the water-transmission compute dispatch turned 495 of 550 previously colored texels in one visible dark patch to exact zero. All four water-resolve draws then received the same texture, byte-for-byte. The end preview matched the preceding dispatch output at all 142,080 sampled sites. A later dispatch filled the isolated patch with color after the copy for this presentation. An adjacent open-water control stayed colored. The broad patch was already black when the preceding dispatch began.

The test used an ASUS ROG Astral RTX 5080, driver 617.14, Ryzen 9 5900X, 32 GB RAM and native Windows 11 build 26200. Renderer `ba723deae61ac9206b677b8c30fa4f078ffcf66a` builds on the guarded water-copyback branch. A verified writable copy of a warm Series X profile reached the stationary boat and the "Collect the package" objective with `VICTORY_TEXTURE_BACK=64x64f16,128x128f36`. DRED was enabled. The diagnostic run ended deliberately. The original save was verified after exit.

## Initial observations from one recording interval

Guest present 2610 contained 5,955 numbered CPU recording entries and 388 distinct output bindings in the inventory. Three sample pairs compare the interior of a dark patch with adjacent open water on the same scan lines. A and B are left and right of the occupants. C is at the far left. Regions exclude the hull, HUD and sun glint.

| Region | Presented RGB, patch / water | Water-transmission target preview RGB, patch / water |
| --- | --- | --- |
| A | 57, 79, 92 / 114, 161, 148 | 0, 0, 0 / 55, 78, 66 |
| B | 87, 105, 110 / 176, 202, 179 | 0, 0, 0 / 66, 93, 74 |
| C | 25, 41, 56 / 77, 127, 115 | 0, 0, 0 / 47, 69, 61 |

These are per-channel median 8-bit image codes. The diagnostic normalizes and curves the floating-point image for display. The scene-color target also showed a darker value in all three patch regions. The underwater mask and classification previews had equal patch and water medians within each pair. The depth preview differed by at most one 8-bit level. A cinematic frame independently showed two dark marks in both the presented image and the scene-color target.

In frame 2610, the trace records one indirect compute call with shader `cad2edef5ecea3ba` and a 2560 × 1440 R11G11B10_FLOAT UAV at table 1, dword 64. Its recorded list appears in a kind-2 submission. We found zero raster draw bindings to that resource in this frame. In an earlier helm frame, 19 raster draws also bound that resource.

## Values immediately around the dispatch

Renderer `d0b78344c3f92d01f8aeef6600c454a894a1ce61` ran on the same RTX 5080, driver, OS, two water selections, warm save and shader cache. Six local launcher and input changes were held constant across both runs. In the same D3D12 command list as the selected dispatch, the diagnostic copied the scene-color input at table 1 dword 32, then the destination immediately before and immediately after the call. The counts below are exact zero words in the native packed RGB destination. Each adjacent control was sampled on the same rows. The end-of-frame preview was sampled at half resolution and reports exact black display pixels. Region names in this table refer only to this second run.

| View and region | Zero texels before | Zero texels after | Adjacent water zero after | End preview black, mark / water |
| --- | ---: | ---: | ---: | --- |
| Cinematic 2273, left mark | 1,300/1,300 | 1,300/1,300 | 0/1,300 | 330/330 and 0/330 |
| Idle helm 2475, broad forward mark | 1,156/1,156 | 1,156/1,156 | 0/1,156 | 306/306 and 0/306 |
| Idle helm 2475, isolated mark | 994/994 | 0/994 | 0/994 | 245/245 and 0/252 |
| Idle helm 2545, broad forward mark | 1,540/1,540 | 1,540/1,540 | 0/1,540 | 390/390 and 0/380 |
| Idle helm 2545, isolated mark | 333/333 | 0/333 | 0/324 | 76/76 and 0/72 |
| Under throttle 2658, broad forward mark | 2,686/2,686 | 2,528/2,686 | 0/2,720 | 680/680 and 0/663 |
| Under throttle 2658, isolated mark | 744/798 | 0/798 | 0/798 | 194/203 and 0/196 |

At the isolated mark in frame 2475, the post-dispatch median in native RGB was `(3.75, 11.625, 7.75)`, close to adjacent water at `(3.5, 10.875, 7.375)`. The presented RGB medians were `(24, 37, 44)` and `(87, 134, 115)` respectively. In frame 2545, all sampled scene-color input texels were zero in both marks and both open-water controls, despite the different destination outcomes. In cinematic frame 2273 the scene-color input had nonzero RGB throughout both the mark and the control. At the last verified stage, the boat was moving under throttle with visible twin wakes and the objective was “Collect the package.” The run ended deliberately and the protected save and cache matched their starting hashes.

We also converted the raw post-dispatch values using the end preview's own scale and sampled the same even coordinates. In frame 2475 this predicts median RGB `(101, 148, 128)` for the isolated mark, where the actual preview is `(0, 0, 0)`. The adjacent control predicts `(98, 144, 126)` and the preview matches it exactly. In frame 2545 the mark predicts `(104, 151, 131)` and is again black, while the control matches `(103, 149, 130)`. The broad mark and its control also match their predicted medians in both frames. Ordinary 8-bit preview rounding cannot account for the isolated mark.

The two stationary recording intervals each contained 19 raster draw bindings to the destination. The under-throttle interval contained zero such bindings, yet also showed a colored post-dispatch result and a dark end preview at the same coordinates. A shared CPU frame number cannot establish submission order across queues or which allocation supplied the final image. A zero word on both sides also leaves open whether a shader wrote zero or skipped that texel.

The initial pass-only probe established the native values on each side of those calls. Five focused tests passed. The full GPU library suite for that revision passed 198 tests with six ignored.

## Matching the compute output to the displayed patch

The third run used renderer `520e4b8e0db191b6de3770946dcf00a10572027f` with the same six local launcher and input changes, guarded two-output copy-back, warm Series X save and cache. It used the RTX 5080 on driver 617.14, Windows 11 Pro 25H2 build 26200.9974 and Ryzen 9 5900X. At the stationary helm the HUD showed “Follow the plane.” For a visible mark near the bow, an isolated mark farther left and adjacent water on the same rows, the regions were selected from the screen image before inspecting the native values.

The trace captures the shader-visible SRV at table 1, dword 72 for each draw, tracks the native resource through submission and completion, and reads its contents immediately before the draw. For the aligned presentation, the host queue submitted these commands in this order. Fence offsets are relative to the dispatch that supplied the measured preview.

| Fence offset | CPU recording interval | Observation |
| ---: | ---: | --- |
| 0 | 2407 | Transmission compute writes the R11G11B10_FLOAT output at table 1, dword 64. |
| +1 | 2407 | Four draws read that allocation at dword 72. Each input matches all 568,320 native output texels. |
| +6 | 2408 | The transmission-target preview matches that output at every one of 142,080 RGB samples on the preview grid. |
| +7 | 2408 | The source copy for the window runs. The presented image shows both marks. |
| +42 | 2408 | The next transmission dispatch fills the isolated mark. Only 106,299 of its 142,080 converted RGB samples match the earlier preview. |

| Region in presentation 2408 | Exact zero before the supplying dispatch | Exact zero after | Presented median RGB |
| --- | ---: | ---: | --- |
| Broad forward patch | 1,444 / 1,444 | 1,444 / 1,444 | 78, 99, 107 |
| Open water alongside broad patch | 0 / 1,463 | 0 / 1,463 | 126, 170, 157 |
| Isolated patch | 0 / 550 | 495 / 550 | 26, 41, 48 |
| Open water alongside isolated patch | 0 / 550 | 0 / 550 | 91, 142, 120 |

The four pixel programs in execution order were `2de553a08b3f5873`, `c706ecc8ef573459`, `49adec105943fef3` and `4536a8bd77616602`. Each retained the same native texture through completion. In the crop, the draws respectively changed 531,846, 0, 305 and 0 whole RGBA16F scene-color texels. At the isolated patch the first draw produced median scene color (0.606, 1.325, 2.431), with adjacent water at (4.262, 12.812, 9.773). Its output contains the contrast seen in the presented image. These draws use filtered sampling. Equal screen coordinates alone cannot identify the UV fetched by an individual fragment.

Four neighboring presentations (2402, 2404, 2406 and 2408) each matched the preceding recorded dispatch output on all 142,080 preview samples. In each case the preview ran six host fences after the supplying dispatch, the window copy ran one fence later, and the dispatch labeled with the preview's CPU interval ran another 35 fences later.

The isolated patch formed during the supplying compute call and cleared during the following one. The broad patch entered and left both calls black. The earlier apparent loss of a colored compute result arose from pairing events by CPU recording interval. The next test is to compare the depth-family input at table 1, dword 56 and record which lanes store to the output at both marks. This would distinguish an incoming value, a skipped store and a computed zero. The capture adds diagnostics. The dark marks remained visible in this run.

## Five minutes at the helm

With the controller idle and the capture trigger quiet, PresentMon counted 1,156 displayed application frames on the title's swap chain. The 1,155 intervals spanned 300.122 seconds, giving **3.848 displayed FPS**, **305.774 ms P99** and a 332.897 ms maximum. No descriptor capture events occurred in this window. The diagnostics build and DRED remained enabled. The counter measures display timing and cannot establish a unique rendered scene per presentation. The project target is at least 29.5 newly rendered FPS and at most 40 ms P99 in each of two five-minute runs.

Over the same interval the renderer emitted 57 rolling performance reports. Their medians were 257.9 ms per frame, 91.8 ms preparing, 134.4 ms on the GPU and 112.1 ms copying back 31.3 MB. Some of these phases overlap. A throttle input after the measured window produced twin wakes behind the moving boat. The game closed deliberately. The nine protected profile files and 818 seed-cache files matched their original hashes.

All seven focused capture tests, the submission-order regression test and offline DXIL validation of all four selected pixel programs passed. Two full GPU-library Cargo runs each had 199 passes, six ignored and one failing WARP indirect-draw budget assertion. The same assertion failed in isolation on both the tested revision and its code-equivalent parent. An earlier direct test-binary run exited during a stack-profiler test, which passed in both Cargo runs.

## Repeating the capture

Use a diagnostics build, an existing private directory in `VICTORY_CAPTURE`, and `VICTORY_CAPTURE_TRIGGER=1`. At the desired view, rename a text file containing `ops` to `capture-next` in that directory. Inspect the frame's operation log, target inventory and presented image. Publish a second marker in the same directory containing `targets` followed by up to eight space-separated hexadecimal IDs from that process. Match the selected resources, format and dimensions in the second frame's inventory and trace. A resource can rotate out between frames. Leave `VICTORY_DUMP_TARGETS` unset for selective readback. The image readbacks stall the instrumented frame, so use a separate run for frame-time comparisons.

For the second run, set `VICTORY_CAPTURE_PASS=cad2edef5ecea3ba,1,32,64,0,660,1184,480` before launch. The fields give the shader, table, input dword, output dword and the rectangle's left, top, width and height in native texels. A triggered frame captures the first matching dispatch. Position the rectangle for the camera and render resolution being tested. Keep raw readbacks and images local. Report the zero-word counts, controls, tested revision, driver and last verified stage.

For the descriptor trace, additionally set `VICTORY_CAPTURE_READERS=1,72,2de553a08b3f5873,c706ecc8ef573459,49adec105943fef3,4536a8bd77616602`. Arm neighboring recording intervals to capture a writer, four reads and a following preview. Run `python tools/descriptor-trace-order.py <local-log> --frame 2408` with the recording interval from your own test. It orders observations by host submission fence and position within the submission. Its last-writer column covers captured operations only. Across gaps where a native allocation could have been freed and its address reused, verify the allocation's lifetime before joining observations. Retain raw captures locally.
