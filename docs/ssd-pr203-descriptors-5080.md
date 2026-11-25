# Descriptor-only shader reads on the RTX 5080

Tested on a GeForce RTX 5080 16 GB, NVIDIA driver 617.14, Ryzen 9 5900X and Windows 11 Pro 25H2 build 26200.9974, with Rust 1.98.1 MSVC and DXIL 1.9.2609.5.

The comparison uses NVIDIA fork PR 688 (`7e7bb180625298dd418cb7ce71923140363ff02f`) plus the previously tested integer blend fix from PR 51 (`d82acefa8e3005a4293f327b4099b6c547602d9e`). The candidate adds only the descriptor-data lowering from PR 203 (`0e49a91c39c4049ab8eece84d79b8af898373c40`). Both builds have the same local archive and input helpers. The PR's interpolation and wave-sum changes were outside this experiment.

## Exact pixel shader comparison

The same two pixel programs failed in the baseline prison run and in `victory translate --jobs 1 --validate --pipelines --list` with `a descriptor read only as data`. With the isolated change, both validated and created offline D3D12 test pipelines. Both also translated and were accepted in their live mesh pipelines.

| Pixel program | Baseline offline | Candidate offline | Candidate live mesh/pixel pipeline |
| --- | --- | --- | --- |
| `7d0f14ce44746d7e` | Refused | Validated, pipeline created | Accepted with mesh `a9184e594aae6932` |
| `6b6db9f1a526daeb` | Refused | Validated, pipeline created | Accepted with mesh `4f23fd8d99482703` |

The five compute controls gave the same results in both builds. Four validated and failed D3D12 pipeline creation with an incorrect-parameter error. The fifth failed at a lane-mask exchange during translation.

## Five-minute prison replay

Both runs reached player control and followed the same camera and movement inputs on the Series X profile, with `VICTORY_RELAY=1`, `VICTORY_CARD_INLINE_STAGE=1` and DRED. The shader cache keys include the executable fingerprint. Both runs logged zero shader-cache hits during startup. The corresponding interior view after the first retreat still showed large wall and ceiling gaps with the patch applied.

| Display and host counters | PR 688 + PR 51 | With PR 203 descriptor lowering |
| --- | ---: | ---: |
| Displayed application frames | 1,371 | 1,364 |
| Presented FPS | 4.565 | 4.543 |
| P99 display interval | 435.139 ms | 435.072 ms |
| Worst display interval | 711.421 ms | 535.089 ms |
| Mean PresentMon GPU busy per interval | 139.90 ms | 140.93 ms |
| Median card-copy fetch wait | 35.20 ms | 34.20 ms |
| Median host-reported missing-pipeline drops per frame | 318 | 316 |

The candidate ended deliberately after recording and showed no DRED device removal. This is one run per build. Average FPS and P99 were nearly identical despite verified recovery of both pixel shader pipelines.

[NVIDIA fork PR 203](https://github.com/fxceless/victory-fork/pull/203) · [Previous integer-blend comparison](https://github.com/iGoldenNinja/victory-fork/blob/9942063c0e11401fceda1446ff3d4a8ef89810c6/docs/ssd-blend-pr51-5080.md)

OpenAI Codex (GPT-6) isolated and ran the comparison and drafted this report.
