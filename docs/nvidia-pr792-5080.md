# PR 792 captured sampler readback on RTX 5080

I reproduced both synthetic sampler fixtures on an RTX 5080 with NVIDIA driver 617.14, Windows 11 Pro 25H2 build 26200.9974, Ryzen 9 5900X and 32 GB RAM. The two MSVC Rust 1.98.1 diagnostic-profile test builds were base `9376fd60c0ba0e76a74c39b49b1407dbac0a64c7` with only the PR's test module and HLSL fixture added, and clean PR head `77938865411a2fcc856d7945fb59161db75fc080`. [PR and original fixture](https://github.com/fxceless/victory-fork/pull/792)

| D3D12 hardware build | Installed descriptor assertion | Fenced shader output changed / clamp / decoy |
| --- | --- | --- |
| Parent, test files only | Failed | `[17, 231, 61]` |
| PR 792 head | Passed | `[231, 231, 61]` |

The candidate recovered the expected output from the copied descriptor. The stable clamp and decoy outputs agreed across both builds. This test reached fenced D3D12 dispatch and readback using synthetic resources. Gameplay and frame timing were outside this test. A true WARP run requires a build with the `diagnostics` feature, so only the default hardware adapter is counted here.

OpenAI Codex (GPT-6) ran the tests and drafted the report.
