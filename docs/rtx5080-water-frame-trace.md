# Dark water patches at the helm on RTX 5080

Three dark patches visible at the idle helm also appeared in the full-resolution water-transmission target before the 4K image reached the window. This narrows the investigation to rendering at or before the 2560 × 1440 scene-color stage. The first command that produces the contrast is still open.

The test used an ASUS ROG Astral RTX 5080, driver 617.14, Ryzen 9 5900X, 32 GB RAM and native Windows 11 build 26200. Renderer `ba723deae61ac9206b677b8c30fa4f078ffcf66a` builds on the guarded water-copyback branch. A verified writable copy of a warm Series X profile reached the stationary boat and the "Collect the package" objective with `VICTORY_TEXTURE_BACK=64x64f16,128x128f36`. DRED was enabled. The diagnostic run ended deliberately. The original save was verified after exit.

## Same-frame evidence

Guest present 2610 contained 5,955 numbered CPU recording entries and 388 distinct output bindings in the inventory. Three sample pairs compare the interior of a dark patch with adjacent open water on the same scan lines. A and B are left and right of the occupants. C is at the far left. Regions exclude the hull, HUD and sun glint.

| Region | Presented RGB, patch / water | Water-transmission target preview RGB, patch / water |
| --- | --- | --- |
| A | 57, 79, 92 / 114, 161, 148 | 0, 0, 0 / 55, 78, 66 |
| B | 87, 105, 110 / 176, 202, 179 | 0, 0, 0 / 66, 93, 74 |
| C | 25, 41, 56 / 77, 127, 115 | 0, 0, 0 / 47, 69, 61 |

These are per-channel median 8-bit image codes. The diagnostic normalizes and curves the floating-point image for display. A zero in that preview establishes the displayed contrast. The underlying floating-point bits still need a direct measurement. The scene-color target also showed a darker value in all three patch regions. The underwater mask and classification previews had equal patch and water medians within each pair. The depth preview differed by at most one 8-bit level. A cinematic frame independently showed two dark marks in both the presented image and the scene-color target.

In frame 2610, the trace records one indirect compute call with shader `cad2edef5ecea3ba` and a 2560 × 1440 R11G11B10_FLOAT UAV at table 1, dword 64. Its recorded list appears in a kind-2 submission. We found zero raster draw bindings to that resource in this frame. In an earlier helm frame, 19 raster draws also bound that resource. End-of-frame images and CPU recording order alone cannot distinguish a dispatch producing the pattern from an input or retained texels carrying it through. The next comparison is the scene-color SRV at table 1 dword 32 and the destination at dword 64 immediately before and after that indirect call.

## Repeating the capture

Use a diagnostics build, an existing private directory in `VICTORY_CAPTURE`, and `VICTORY_CAPTURE_TRIGGER=1`. At the desired view, rename a text file containing `ops` to `capture-next` in that directory. Inspect the frame's operation log, target inventory and presented image. Publish a second marker in the same directory containing `targets` followed by up to eight space-separated hexadecimal IDs from that process. Match the selected resources, format and dimensions in the second frame's inventory and trace. A resource can rotate out between frames. Leave `VICTORY_DUMP_TARGETS` unset for selective readback. The image readbacks stall the instrumented frame, so use a separate run for frame-time comparisons.
