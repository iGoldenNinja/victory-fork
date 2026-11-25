# Two input bindings of the prison lighting resolve on RTX 5080

Two earlier compute passes bound writable views of textures that the reflective-caustics temporal resolve (`34fb51b8a0312e82`) subsequently bound for reading. Both matches appeared in all five sampled dispatches during a 49-second hold at the prison-window movement tutorial. The probe compared the actual host textures in descriptor slots reserved for the command list.

| Resolve SRV (root, dword) | Earlier compute pass | Earlier UAV (root, dword) | Matched dispatches |
| --- | --- | --- | --- |
| 1, 16 | Tiled lighting material-mask generation, `f0c91b4d2578f5bf` | 1, 40 | 5/5 |
| 1, 56 | Indirect diffuse lighting, screen-space bent-cone disocclusion dilation, `3811b0734e389cd6` | 1, 8 | 5/5 |

Each dispatch reported 23 SRVs. Of the 115 observations, 10 matched the two earlier UAVs, 65 identified host textures with no earlier tracked UAV in that frame, and 40 were untracked. Of those 40, 30 descriptors lacked a matching resource object and 10 referred to a buffer or texture without a host copy. Earlier in the launch, frame 2176 also matched deferred tiled water-transmittance optical-depth processing (`a929109552ce1e84`, UAV 1, 24) to SRV 0, 448. That match occurred before the prison measurement.

For all five dispatches, the recorded frame, probe ordinal, submission and list matched exactly one readback of the original output UAV (1, 176). Eight rows of the 2560 × 1440 R32_UINT texture contained 7,092, 6,644, 5,907, 5,953 and 5,545 nonzero packed RGB samples at frames 2272, 2304, 2336, 2368 and 2400. Together that is 31,141 of 102,400 sampled texels (30.41%). The earlier measurement on the same machine found 14,180 of 40,960 at this location. [Previous PR 917 measurement](https://github.com/iGoldenNinja/victory-fork/blob/bd68e02b9f74f7a2c7795f552d19c5204679edb0/docs/rtx5080-original-light-resolve.md)

The native Series X run used diagnostics revision `fefc2e24f92c8e119f19a0f07b37220e11a7bcbf` plus local archive and replay support. Hardware was an RTX 5080 Astral 16 GB, driver 617.14, Ryzen 9 5900X, 32 GB RAM, and Windows 11 Pro 25H2 build 26200.9974. Both lighting probes were enabled. The ray builder and ambient override were off. The controller ended the run after the matched hold.

The trace covers completed direct compute commands and identifies shared texture objects. Actual shader stores, overlapping subresources, input values and ray contributions remain unverified. The two repeatable bindings provide specific inputs to sample in a follow-up on the dark cell.
