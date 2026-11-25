# VIctory PR 2176 on an RTX 5080

I tested PR 2176 at `3feadc6f71e3d16f5fd54242e406c37fc856ec48` on Windows 11 Pro 25H2 build 26200.9974, NVIDIA GeForce RTX 5080 16 GB, driver 617.14, Ryzen 9 5900X and MSVC Rust 1.98.1. This adapter reports a D3D12 wave lane range of 32–32. The test harness calls the PR's `set_host_wave32` directly and uses the PR's translator and D3D12 test device. [PR and code](https://github.com/Marvisak/VIctory/pull/2176)

## Pipeline creation

I used the same four compute inputs as the [earlier same-machine offline comparison](https://github.com/iGoldenNinja/victory-fork/blob/3cc1cc4e19736703430d7c668959a94a464667a5/docs/xcl-wave64-5080.md). The PR revision translated and validated three inputs in each mode. With host Wave32 lowering disabled, all three PSOs returned `E_INVALIDARG`. With it enabled, all three PSOs were created. The fourth input reaches a separate descriptor translation limit at native offset `0x4d0` in both modes. The matched programs were exercised through PSO creation. The output comparison below uses a purpose-built synthetic program.

| Host Wave32 mode | Inputs tested | DXIL validated | Compute PSOs created |
| --- | ---: | ---: | ---: |
| Off | 4 | 3 | 0 |
| On | 4 | 3 | 3 |

## Cross-half readback

The synthetic compute fixture has one 64-thread guest wave. Thread `i` supplies `1000 + i` to `ds_bpermute_b32`. Every thread reads the value addressed by byte offset `32 * 4` and stores it in its own UAV output slot. Guest Wave64 semantics require all 64 outputs to equal `1032`.

With host Wave32 lowering enabled, the translated DXIL validated, created a compute pipeline and executed. Lanes 0–31 all wrote `1000`, and lanes 32–63 all wrote `1032`. This agrees with the patch's masking of the requested index to five bits and subsequent use of the current host wave. The fixture reproduced the wrong output on hardware and failed its guest-semantics assertion.

OpenAI Codex (GPT-6) built the test harness, ran the tests, and drafted this report.
