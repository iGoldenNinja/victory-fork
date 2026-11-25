# PR 823 Wave32 pipelines on the RTX 5080

I tested NVIDIA fork PR head `aaf658a40f3e512550165e489a0f0c411e3c9544` and parent `9376fd60c0ba0e76a74c39b49b1407dbac0a64c7` with an RTX 5080 16 GB, driver 617.14, Windows 11 Pro 25H2 build 26200.9974, Ryzen 9 5900X and 32 GB RAM. Both executables use the same local archive and replay support. [PR 823](https://github.com/fxceless/victory-fork/pull/823)

## Offline pipeline creation

I set `VICTORY_HOST_MAX_WAVE=32` and ran the same seven compute inputs through `translate --jobs 1 --validate --pipelines --list`. All seven emitted DXIL and validated in each test. Both the parent and candidate with `VICTORY_WAVE_WIDE=0` created zero pipelines. With `VICTORY_WAVE_WIDE=1` for this bounded offline test, the candidate created four. The three controls still failed pipeline creation with the invalid-parameter error.

| Compute program | PR 823 offline D3D12 pipeline |
| --- | --- |
| `5dc3b25986622949` | Created |
| `6f225b641c5b0708` | Created |
| `369cbe906ea72764` | Created |
| `2cc4c197ee29e291` | Created |
| `02384a60fda6f2d0` | Refused |
| `2da4beb386509b77` | Refused |
| `6b82bc7d22016cb5` | Refused |

## Gameplay replay

I started the PR in the Series X profile with the default relay, readback, ray-tracing builder and per-program wave settings. The host pipeline library loaded at the same size as the earlier parent run. The same 3,067 cache files were copied from the original seed. The PR's changed translation key produced zero shader-cache hits at startup, while the parent reference started with warm shader entries. Creation of all four compute pipelines appeared in the PR title log using the default allowlist. The same three compute controls and one mesh pipeline remained refused (`E_INVALIDARG`).

| 90-second prison replay | Earlier parent run | PR 823 |
| --- | ---: | ---: |
| Displayed application frames | 322 | 268 |
| Displayed application FPS | 3.56 | 2.97 |
| P99 displayed frame time | 470.38 ms | 552.72 ms |
| Worst displayed frame time | 658.55 ms | 593.89 ms |
| Mean GPU busy per displayed frame | 202.79 ms | 209.17 ms |
| Median reported draws per frame | 5,770 | 6,627.5 |
| Unique host pipeline refusals | 8 | 4 |

Both builds reached controllable prison and neither logged device removal. The first pan matched within 36 ms after player selection, with later inputs matching within 20 ms relative to the pan. Blocky tiles below the courtyard window remained visible in two stationary captures around 25 seconds apart on PR 823. At the comparable post-retreat view, the ceiling and walls drew in both builds and the stepped character edge remained. The PR view showed additional lit detail on the divider and floor in that capture.

The live comparison uses one run per revision, with differing startup shader-cache behavior and draw counts. It measures the full PR, which also changes page mapping. The observed FPS difference does not isolate the cost of the four compute programs.

## Same-build allowlist comparison

I reused the enabled run above and repeated the same 90-second Series X prison replay on the identical `aaf658a` executable with `VICTORY_WAVE_WIDE_ONLY=none`. Both starts used separate, byte-matched copies of the same 3,067 cache seed files and the same host pipeline library. Both had zero startup shader-cache hits. Relay, readback, the builder, and the PR's page-mapping code were fixed across the two runs. The enabled run came first.

| 90-second prison replay | Allowlist enabled | Allowlist disabled |
| --- | ---: | ---: |
| Targeted compute pipelines created | 4/4 | 0/4 |
| Unique D3D12 pipeline refusals | 4 | 8 |
| Displayed application frames | 268 | 325 |
| Displayed application FPS | 2.97 | 3.59 |
| P99 displayed frame time | 552.72 ms | 517.43 ms |
| Worst displayed frame time | 593.89 ms | 605.65 ms |
| Mean GPU busy per displayed frame | 209.17 ms | 201.10 ms |
| Median reported draws per frame | 6,627.5 | 5,610 |

With the allowlist disabled, the four targeted programs returned `E_INVALIDARG`. Both runs reached controllable prison and finished the route without a logged device removal. The first pan matched within 12 ms of player selection and later input offsets matched within 25 ms. Both runs logged 405 startup shader-cache misses and 3,058 misses at the last tally, with zero hits.

Blocky tiles below the courtyard window and stepped character edges were visible in both settings. With the allowlist enabled, the comparable post-retreat view showed more lit detail on the divider and floor. The timing and draw counts describe this replay pair. Each setting was run once, in fixed order.

