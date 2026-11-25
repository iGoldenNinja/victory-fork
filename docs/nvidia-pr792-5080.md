# PR 792 captured sampler readback on RTX 5080

I reproduced both synthetic sampler fixtures on an RTX 5080 with NVIDIA driver 617.14, Windows 11 Pro 25H2 build 26200.9974, Ryzen 9 5900X and 32 GB RAM. The two MSVC Rust 1.98.1 diagnostic-profile test builds were base `9376fd60c0ba0e76a74c39b49b1407dbac0a64c7` with only the PR's test module and HLSL fixture added, and clean PR head `77938865411a2fcc856d7945fb59161db75fc080`. [PR and original fixture](https://github.com/fxceless/victory-fork/pull/792)

| D3D12 hardware build | Installed descriptor assertion | Fenced shader output changed / clamp / decoy |
| --- | --- | --- |
| Parent, test files only | Failed | `[17, 231, 61]` |
| PR 792 head | Passed | `[231, 231, 61]` |

The candidate recovered the expected output from the copied descriptor. The stable clamp and decoy outputs agreed across both builds. This test reached fenced D3D12 dispatch and readback using synthetic resources. Gameplay and frame timing were outside this test. A true WARP run requires a build with the `diagnostics` feature, so only the default hardware adapter is counted here.

## Gameplay replay

I then used the same RTX 5080 and Windows setup to compare parent `9376fd6` against PR head `7793886` in the first controllable prison room. Both diagnostics builds used the same local archive and input support, the Series X profile, default readback and separate copies of the same 3,067 warm shader and host pipeline cache files. The parent data comes from the earlier readback-off run. The first pan matched within 23 ms of player selection and subsequent actions matched within 22 ms relative to the first pan.

| 90-second gameplay measurement | Parent | PR 792 |
| --- | ---: | ---: |
| Displayed application frames | 322 | 320 |
| Displayed application FPS | 3.56 | 3.54 |
| P99 displayed frame time | 470.38 ms | 493.93 ms |
| Worst displayed frame time | 658.55 ms | 646.78 ms |
| Mean GPU busy per displayed frame | 202.79 ms | 202.45 ms |
| Median reported draw count | 5,770 | 5,292 |

Both runs reached the room, accepted camera and movement inputs, and logged the same eight unique D3D12 pipeline refusals (`E_INVALIDARG`, seven compute pipelines and one mesh pipeline). Neither run logged device removal. At the matched post-retreat view, the sharp lighting boundary across the room and stepped character edges were visible on both builds.

This is one run per build. The synthetic sampler result remains independently reproducible. The gameplay replay did not instrument whether the title hit the descriptor mismatch, and it showed no clear improvement in the visible artifacts or frame rate.

OpenAI Codex (GPT-6) ran the tests and drafted the report.
