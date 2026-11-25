# Why the water transmission shader writes black at the boat helm

All 495 coordinates from the previously reported isolated water patch turned from colored to exact packed zero again on an RTX 5080. At every one, the water shader's bound depth SRV contained a positive finite float. A separate replay instrumented the shader itself. All 495 sites then reported an exact zero RGB result from the full shading calculation with nonzero final exposure. The zero-depth return and the alternate scene-copy path have distinct signatures. [Earlier measurement](rtx5080-water-frame-trace.md)

## Depth and the displayed image

We used renderer `3fb508351b866d5b90c85d819930fbdf18a00b38`, an ASUS ROG Astral RTX 5080 on driver 617.14, Windows 11 Pro 25H2 build 26200.9974, a Ryzen 9 5900X and 32 GB of RAM. Each Series X replay began from a fresh verified copy of the same story save and warm shader cache. Six local launch and input changes, the renderer executable, and the two flotation copy-back selections were held constant. The boat stayed afloat during the stationary “Follow the plane” test.

The first replay used the ordinary compiled water shader. For each sampled dispatch we read its actual table 1 dword 56 shader-visible binding and the dword 64 destination immediately before and after execution. The slot referenced a 2560 × 1440 R32 typeless depth texture through an R32_FLOAT view of mip zero. The input and output bindings stayed on their respective allocations at recording, submission and completion. The raw input was copied as an entire depth plane and cropped after the fence completed. A 585-texel offset crop with 216 known zeros and 369 known nonzero values passed on the RTX 5080 and on WARP.

| Native observation at the stationary helm | Previously colored sites that turned black | Depth at those sites |
| --- | ---: | --- |
| Original 495 coordinates, writer 434 | 495/495 | 0.0052650 to 0.0053847 |
| Same coordinates, later writer 583 | 495/495 | 0.0052681 to 0.0053877 |
| Screen-selected mark at the left edge, writer 372 | 3,780/3,780 | 0.0038431 to 0.0046478 |
| Screen-selected interior mark, writer 442 | 2,525/2,525 | 0.0038555 to 0.0042256 |

The same-row controls stayed colored at all 3,780 and 2,525 sites for those last two comparisons. The original 495 coordinates had positive depth in all 43 dispatches sampled across two stationary sequences. Their exact zero transitions occurred in four dispatches. The original historical frame had no depth readback.

For writer 583 the four subsequent dword 72 water-resolve inputs matched its output at every sampled preview location. The target preview matched at all 142,080 RGB samples. The physical fences were 51,335 for the writer, 51,341 for the preview and 51,342 for the window copy. We also matched the two separately selected visible marks to their preceding writers at all 142,080 RGB samples each.

## Which store ran

The second replay changed one validated PC shader entry in its own writable cache. A runtime cache hit and distinct marker values in the native R11G11B10_FLOAT output verified that the shader ran. Its markers distinguish the explicit zero-depth store, exact zero from full shading, exact zero from the alternate scene-color path, zero final exposure, nonpositive nonzero output, and NaN. Other shading values passed through.

All 495 original sites changed from an ordinary color to the full-shading zero marker in one dispatch. Two patches selected from a separate presented frame contained the same marker at 336/336 and 105/105 sites. Their adjacent controls were ordinary colored water at 336/336 and 105/105 sites. The supplying dispatch matched the target preview at all 142,080 samples. This locates the black write before the packed-format conversion and the final water-resolve draws.

## Four-sample reconstruction and a correction trial

A third replay alternated four 32-frame windows in one shader on the same executable and save. Two windows used the original shading. One marked the inputs responsible for exact zero. The fourth retained the existing destination texel when the ordinary full shading result was exactly zero in all three channels and both the water depth and final exposure were positive and finite. The explicit depth-zero store continued to run. Other computed colors were written normally.

The shader combines four samples of half-resolution water data using weights. In the measured failure their combined weight failed the positive test, leaving the reconstructed color and its contribution to the final shading at zero. The marker for that case appeared at all 495 original sites together. Across the captured crop, 162,252 ordinary colored samples changed to that marker in the 13 measured witness dispatches. The other three instrumented full-shading causes accounted for zero such transitions. The next input to trace is the alpha channel at table 1 dword 24, which participates in the tap weights, together with the two preceding water-volumetric writers. The measurements establish the failed reconstruction. Which producer or input made those weights unusable is still open.

The stationary analysis covers four cycles and excludes the four counter values at each end of every window. At a transition we observed that the phase in the native output could lead the scene counter printed at CPU recording by one tick. The counts below are observations of native texels at positive, finite water depth. The same coordinates can appear in more than one dispatch.

| Stationary region | Original shading, 25 dispatches | Retain previous color, 11 dispatches |
| --- | ---: | ---: |
| Black after the dispatch across the 1,184 × 480 crop | 463,189 / 13,298,693 (3.48%) | 71,025 / 5,851,771 (1.21%) |
| Previously colored samples turned black across that crop | 202,692 | 0 |
| Black after the dispatch in the fixed open-water region | 10,232 / 3,240,000 | 0 / 1,425,600 |
| Black in the smaller same-row water control | 0 / 36,575 | 0 / 16,093 |

The median per-dispatch black fraction across that crop fell from 3.12% to 1.22%. Retention reduced the measured black surface area but left holes where the destination already contained black. It can also preserve an earlier frame's surface color. The timing figures from the earlier uninstrumented helm window remain the performance reference. This trial did not benchmark frame rate.

Under throttle the boat moved toward the bridge with twin wakes. Across the moving capture, black at positive depth fell from 195,045/8,174,664 observations (2.39%, 16 original dispatches) to 28,003/3,018,281 (0.93%, six retention dispatches). New black writes fell from 88,162 to zero. The views moved between samples. During a cinematic camera change, 429 positive-depth texels in one retention dispatch kept the exact diagnostic color that was already there, and four did so in another. This demonstrates stale color surviving in the destination. Retention is a partial workaround while the upstream water samples are being investigated.

All seven focused capture tests passed. The offset-depth fixture also passed when explicitly run on hardware and WARP. The full GPU-library run had 199 passes, seven ignored and one failed WARP indirect-draw budget assertion. The same assertion failed on the preceding renderer and its code-equivalent parent. All three title runs ended deliberately and the protected save passed its hash check after each exit.

## Four individual taps and a current-frame fallback

A fourth replay on the same RTX 5080 recorded the alpha values actually loaded from dword 24, the source depths at dword 48 and the interpolation factors for the four reconstruction taps. The alpha texture is 1280 × 720 R10G10B10A2_UNORM. Each alpha code can take four values. The diagnostic preserved every code, the positive-interpolation mask and the positive-finite-depth mask. An exhaustive CPU check recovered all 131,072 combinations of the packed diagnostic fields.

| Scene | No positive reconstruction weight | All four alpha codes zero | All four source depths positive and finite | All four interpolation factors positive |
| --- | ---: | ---: | ---: | ---: |
| Stationary boat | 223,286 | 223,286 | 223,286 | 223,286 |
| Moving boat | 122,251 | 122,251 | 122,251 | 122,251 |

These are repeated texel observations within the sampled crop. Across 9,791,236 positive-weight controls, zero had four zero alpha codes. At the original 495 coordinates, the individual-tap witness caught 396 failing observations together in one dispatch. Every one had the same four-zero alpha signature. The earlier combined-weight witness had independently caught all 495 failing together.

In the dispatch with those 396 observations, the two preceding water-volumetric dispatches bound the same half-resolution alpha and depth allocations that the reconstruction shader subsequently read. Their output slots were dwords 152 and 120 respectively. Both used indirect dispatch. Across ten stationary witness dispatches held out from the initial edge search, 2,645/2,952 horizontal-coordinate transitions and 5,703/7,950 vertical-coordinate transitions lay exactly one full-resolution pixel from a multiple of 32. The corresponding moving counts were 1,680/2,019 and 2,856/4,246.

The same capture identifies an earlier operation for testing. Tile generation (`1aa247ed3653b616`, dispatch 20 × 12 × 1) bound a 58,880-byte buffer at output dword 32. Both volumetric programs subsequently bound that exact buffer range at input dword 16. Inspection of their compiled host programs shows that each Wave32 group addresses two pixels per lane in an 8 × 8 half-resolution tile. One program reads forward through the packed coordinate list and the other indexes backward from a supplied count. Each also has a path that can explicitly write zero alpha at output dword 152. A capture of list membership plus the before/after alpha around each writer would distinguish a missing tile from an actual zero write.

We also tested replacing the failed full-shading result with the current scene color sampled at the same pixel, scaled by the shader's exposure terms. The replacement requires positive finite water depth and final exposure, an unusable accumulated weight and exact black in all three normal output channels. At the stationary helm the following counts use 24 original and 12 treatment dispatches from the same replay. The comparison excludes the first and last four scene ticks of each 32-tick phase.

| Region | Original black / positive-depth samples | Current-frame fallback |
| --- | ---: | ---: |
| Whole 1,184 × 480 crop | 466,667 / 12,767,573 (3.66%) | 203,633 / 6,383,182 (3.19%) |
| Fixed open-water strip | 28,070 / 3,110,400 (0.90%) | 868 / 1,555,200 (0.056%) |
| Original 495 coordinates | 396 / 11,880 (3.33%) | 378 / 5,940 (6.36%) |

The fixed strip improved, while the small original coordinate set had a higher black fraction during the treatment phases. These windows sample different frames. The fallback can display an underlying scene surface. Reconstructing the intended water still depends on the half-resolution inputs.

A fifth replay repeated the throttle comparison after three successive helm images matched the reference. Throttle began at scene tick 719, near the end of the original washout phase. The boat accelerated toward the bridge and developed twin wakes. The measurement excluded the first three seconds of throttle and the transition ticks of every phase. The earlier moving replay had provided source-alpha evidence but had missed the treatment window.

| Region under throttle | Original, 15 dispatches | Current-frame fallback, 12 dispatches |
| --- | ---: | ---: |
| Whole crop | 149,135 / 7,585,155 (1.97%) | 113,051 / 6,375,509 (1.77%) |
| Fixed open-water strip | 2,926 / 1,943,996 (0.15%) | 0 / 1,555,200 |
| Original 495 coordinates | 495 / 7,425 (6.67%) | 0 / 5,940 |

The moving repeat caught another 41,948 failed-weight observations. Every one had four zero alphas, four positive interpolation factors and four finite positive source depths. None of the 2,432,317 positive-weight controls had four zero alphas. The one moving treatment window covered different positions along the route from the original windows. The whole-crop improvement was modest, visible black marks persisted in the scene, and the original coordinates gave mixed results across the stationary and moving replays. Both additional runs ended deliberately and verified the protected save. The current-frame treatment remains experimental.
