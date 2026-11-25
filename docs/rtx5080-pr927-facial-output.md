# PR 927 restores a changing facial-network output on RTX 5080

Two native Series X profile runs reached the prison on Windows 11 Pro 25H2 build 26200.9974, GeForce RTX 5080 Astral 16 GB, NVIDIA driver 617.14, Ryzen 9 5900X and 32 GB RAM. We measured the 1,064,960-byte R16_UINT buffer read by the blend-shape extractor, at root table 0, dword 16. This independently confirms the network-output change reported by tippythetaidum on another RTX 5080 configuration. [Original PR and author's results](https://github.com/fxceless/victory-fork/pull/927)

| Measurement | Legacy translation | PR 927 translation |
| --- | ---: | ---: |
| Whole-run nonzero readbacks | 0 / 76 | 69 / 69 |
| Changed contents on successive comparable reads of the same view | 0 / 75 | 59 / 68 |
| Nonzero readbacks in the first 15 seconds at the matched prison window | 0 / 12 | 13 / 13 |
| Nonzero bytes per read in those 15 seconds | 0 | 382,454–384,847 |

Both arms used the same diagnostic executable from `b055481`, with `VICTORY_FACE_PROBE=1` and `VICTORY_D16_TYPED_BITS=0` or `1`. It includes the typed-buffer changes from `5dc9e63` on `92a72df`. Each arm used a separate copy of the same saved profile and driver pipeline cache. We pretranslated the same 3,077 shaders for each mode. Three representative title shaders validated offline in the baseline, control and treatment. Disabling the change matched the original translator byte for byte in all three. Enabling it changed the SiLU and vector-matrix producers and preserved the downstream extractor. [Diagnostic source](https://github.com/iGoldenNinja/victory-fork/commit/b055481d513b3f9fddbc9c3c74db45c8e9613e4d)

The diagnostic sampled one in every 32 extractor dispatches. It read whole views up to 4 MiB after dispatch, collected results after GPU completion, and compared contents only when the same resource and binding recurred. All observed readbacks mapped successfully. The separate 2,809,960-byte downstream UAV already contained changing data in the control. A nonzero value in that UAV alone would be a poor test of the patch.

### Opening-cutscene skin remains under investigation

After this comparison, tippythetaidum reported colored squares on the opening waitress's skin with the change enabled and clean skin with it disabled on an RTX 5080. We checked our saved translation of the extractor. Three ordinary `buffer_load_format_x` instructions all read root table 0, dword 16. The captured DXIL declares the handle as a four-component float buffer and emits three `bufferLoad.f32` calls. For each call it reinterprets the first component's bits as an integer and decodes the low half as a half float. Both local GPU runs recorded an R16_UINT view at that exact binding. This confirms the mismatch the author suspected. Its effect on those particular pixels still needs a matched visual test. Treat this branch as experimental. [Author's visual comparison and hypothesis](https://github.com/fxceless/victory-fork/pull/927#issuecomment-7164653246)

Both prison observation windows began at the same camera. In the treatment the camera subsequently turned toward the bunks between 15 and 25 seconds. Across the first 45 seconds the displayed rates were 2.61 and 2.23 FPS, with P99 frame times of 529 and 870 ms. Those diagnostic windows include different views. Both runs ended deliberately. The captured close-ups did not establish lip sync or normal animation speed.

An independent local scan found that all 425 typed 16-bit access sites in the 24 network producer programs observed binding on the earlier RTX 5080 run used the first descriptor in their declared range. All 1,156 sites in the larger 65-program family had the same property. The synthetic interior-range case has separate hardware and software-renderer coverage. tippythetaidum corrected its shader offset in `befaf3b`, and avt0m4t48 documented the remaining fresh-table size case. [Upstream correction](https://github.com/fxceless/victory-fork/pull/927#issuecomment-7164043623) · [Host-table discussion](https://github.com/fxceless/victory-fork/pull/927#issuecomment-7164227485) · [RTX 5080 and WARP table tests](https://github.com/iGoldenNinja/victory-fork/blob/094ed30187a6c75c06343d3cf3fbf80b5b2e1288/docs/rtx5080-pr927-range-offset.md)
