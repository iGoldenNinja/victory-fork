# VIctory PR 2176 on an RTX 5080

I tested PR 2176 at `3feadc6f71e3d16f5fd54242e406c37fc856ec48` on Windows 11 Pro 25H2 build 26200.9974, NVIDIA GeForce RTX 5080 16 GB, driver 617.14, Ryzen 9 5900X and MSVC Rust 1.98.1. This adapter reports a D3D12 wave lane range of 32–32. The test harness calls the PR's `set_host_wave32` directly and uses the PR's translator and D3D12 test device. [PR and code](https://github.com/Marvisak/VIctory/pull/2176)

## Pipeline creation

I used the same four compute inputs as the [earlier same-machine offline comparison](https://github.com/iGoldenNinja/victory-fork/blob/3cc1cc4e19736703430d7c668959a94a464667a5/docs/xcl-wave64-5080.md). The PR revision translated and validated three inputs in each mode. With host Wave32 lowering disabled, all three PSOs returned `E_INVALIDARG`. With it enabled, all three PSOs were created. The fourth input reaches a separate descriptor translation limit at native offset `0x4d0` in both modes. The matched programs were exercised through PSO creation. The output comparison below uses a purpose-built synthetic program.

| Host Wave32 mode | Inputs tested | DXIL validated | Compute PSOs created |
| --- | ---: | ---: | ---: |
| Off | 4 | 3 | 0 |
| On | 4 | 3 | 3 |

## Corrected RDNA2 lane tests

My original `ds_bpermute_b32` fixture expected a value from the entire 64-lane wave. On RDNA2 this instruction selects within each 32-lane half. The original hardware output, `1000` for lanes 0–31 and `1032` for lanes 32–63, matches that rule. The fixture passes on the RTX 5080 with the corrected expectation. [AMD RDNA2 ISA, section 10.4.4](https://www.amd.com/content/dam/amd/en/documents/radeon-tech-docs/instruction-set-architectures/rdna2-shader-instruction-set-architecture.pdf) · [Native AMD checks in XCL PR 1022](https://github.com/xcl-dev/xcl/pull/1022)

I independently reproduced Dathinel's `v_readlane_b32` result on the RTX 5080. This instruction selects a lane from the full 64-lane guest wave. Each lane supplied its group thread ID, and the synthetic program requested lanes 5 and 32 in separate dispatches. DXIL validated, and both dispatches completed and returned the following values. [AMD RDNA2 ISA, `V_READLANE_B32`](https://docs.amd.com/api/khub/documents/Et~wpu9g~Ffl7d9q0QZ~Og/content) · [Original RTX 3050 Laptop test](https://github.com/Marvisak/VIctory/pull/2176#issuecomment-7155645168)

| Requested guest lane | Expected in all 64 outputs | Observed lanes 0–31 | Observed lanes 32–63 |
| --- | ---: | ---: | ---: |
| 5 | 5 | 5 | 37 |
| 32 | 32 | 0 | 32 |

The fixture patch applies to the pinned PR revision. With the DXIL validator available, run `cargo test --profile diag -p victory-shader --lib wave64_host32_ -- --nocapture --test-threads=1` on a 32-lane D3D12 adapter. The corrected permute fixture passes and the full-wave read fixtures fail their assertions on this revision. [Synthetic fixture patch](https://github.com/iGoldenNinja/victory-fork/blob/3e01f314dda0e96fbf4b09472b585e185a06eb9f/docs/victory-pr2176-synthetic-cross-half.patch)

OpenAI Codex (GPT-6) built the test harness, ran the tests, and drafted this report.
