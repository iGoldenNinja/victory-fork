# Paired Wave64 compute pipelines in play on RTX 5080

Tested an RTX 5080 16 GB, driver 617.14, Windows 11 Pro 25H2 build 26200.9974, Ryzen 9 5900X, and 32 GB RAM. The offline pass used NVIDIA PR 854 at `0751197e744c92d392a65d02fe40973f566843e2` and its pinned `f85101b6095e8cf3cecf8f0647cc7f5428daa177` base. [PR 854](https://github.com/fxceless/victory-fork/pull/854)

## D3D12 pipeline creation

Seven compute shaders that previously failed host pipeline creation were translated and validated in each arm. Pairing produced a Wave32 host program for each and resolved all seven `E_INVALIDARG` refusals. The other PR changes were present in both PR arms.

| Revision and setting | DXIL validated | Host pipelines created |
| --- | ---: | ---: |
| Pinned base `f85101b` | 7 of 7 | 0 of 7 |
| PR 854 with `VICTORY_WAVE_PAIRS=0` | 7 of 7 | 0 of 7 |
| PR 854 with paired lowering enabled | 7 of 7 | 7 of 7 |

## Series X replay

The live build combined PR 854 with the PR 834 dispatch split and its diagnostics in `0ec6631dc1d7d51632c04a9621b1579cdcb29b8c`, using the same local archive and input support as the earlier split replays. The screen-space tracing tile generator `2cc4c197ee29e291` created a host pipeline and was bound in play. The diagnostics recorded no compute PSO refusals. The one recorded mesh PSO refusal was `f5e50eb2108c425b` / `ce3f0432a08f1d35`, the integer-target blending case addressed by PR 51. [Earlier split replay](nvidia-pr834-live-dispatch-5080.md) · [PR 51](https://github.com/fxceless/victory-fork/pull/51)

The player entered the prison, responded to the camera and movement inputs, and completed the 90-second route. GPU argument readback accounted for all the requested diffuse-lighting tile groups in 17 of 17 samples during that interval. The largest request sampled in the launch was 115,200 groups, represented by 65,535 and 49,665.

In the captures, the large lower band was absent. The interior remained dark, and irregular edges and bands were still visible on the character. The ray-tracing structure builder was off. Its stages accounted for a median of 341 intentionally skipped dispatches per frame in 16 periodic reports during the route. This test did not isolate the effect of the paired shaders on those remaining artifacts.

The first recorded displayed frame was 4.17 seconds after the route marker. There were 89.45 seconds of display data from that frame to the last captured frame. A predeclared 90-second display segment therefore did not qualify. The first full 75-second segment from that frame measured 2.791 displayed FPS and 793.80 ms P99. The window dimensions changed between the initial prison and route captures. These measurements describe this build and session. A paired-lowering performance comparison needs matched capture dimensions and a complete timing window. The run ended deliberately with no logged device removal.

Testing and reporting assisted by OpenAI Codex.
