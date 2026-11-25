# Dark water patches at the helm on RTX 5080

Three dark patches visible at the idle helm also appeared in the preview of the 2560 × 1440 water-transmission target. A second native run measured the destination immediately before and after the transmission dispatch. One mark was already zero and stayed zero. At a second mark the dispatch filled every sampled texel with color comparable to adjacent water. The end-of-frame preview and presented image for that recording interval still showed the second mark. The first command responsible for the visible contrast remains open.

The test used an ASUS ROG Astral RTX 5080, driver 617.14, Ryzen 9 5900X, 32 GB RAM and native Windows 11 build 26200. Renderer `ba723deae61ac9206b677b8c30fa4f078ffcf66a` builds on the guarded water-copyback branch. A verified writable copy of a warm Series X profile reached the stationary boat and the "Collect the package" objective with `VICTORY_TEXTURE_BACK=64x64f16,128x128f36`. DRED was enabled. The diagnostic run ended deliberately. The original save was verified after exit.

## Same-frame evidence

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

The two stationary recording intervals each contained 19 raster draw bindings to the destination. The under-throttle interval contained zero such bindings, yet also showed a colored post-dispatch result and a dark end preview at the same coordinates. A shared CPU frame number cannot establish submission order across queues or which allocation supplied the final image. A zero word on both sides also leaves open whether a shader wrote zero or skipped that texel.

The next test is to track the destination's allocation, submitted command list and fence through the copy or resolve that supplies the presented image, including adjacent presents. For the marks that stay zero, capture the depth-family input at table 1 dword 56 and measure whether the shader actually wrote the sampled texels. The probe is opt-in. The five focused capture tests passed. The full GPU library suite passed 198 tests with six ignored. Performance needs a separate run with readback disabled.

## Repeating the capture

Use a diagnostics build, an existing private directory in `VICTORY_CAPTURE`, and `VICTORY_CAPTURE_TRIGGER=1`. At the desired view, rename a text file containing `ops` to `capture-next` in that directory. Inspect the frame's operation log, target inventory and presented image. Publish a second marker in the same directory containing `targets` followed by up to eight space-separated hexadecimal IDs from that process. Match the selected resources, format and dimensions in the second frame's inventory and trace. A resource can rotate out between frames. Leave `VICTORY_DUMP_TARGETS` unset for selective readback. The image readbacks stall the instrumented frame, so use a separate run for frame-time comparisons.

For the second run, set `VICTORY_CAPTURE_PASS=cad2edef5ecea3ba,1,32,64,0,660,1184,480` before launch. The fields give the shader, table, input dword, output dword and the rectangle's left, top, width and height in native texels. A triggered frame captures the first matching dispatch. Position the rectangle for the camera and render resolution being tested. Keep raw readbacks and images local. Report the zero-word counts, controls, tested revision, driver and last verified stage.
