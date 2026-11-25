# XCL GPU parity on the RTX 5080

Test system was a GeForce RTX 5080 16 GB on NVIDIA driver 617.14, Ryzen 9 5900X, Windows 11 Pro 25H2 build 26200.9974, Rust 1.98.1 MSVC and DXC/DXIL 1.9.2609.5. The test selected the RTX 5080 with `XCL_ADAPTER=5080` and required GPU execution with `XCL_REQUIRE_GPU=1`.

The clean baseline was [XCL PR 1778](https://github.com/xcl-dev/xcl/pull/1778) head `2e2ee557dfeb6de0f1d3c7467b16216f61564b0e`. It ran this synthetic release suite.

```text
cargo test --locked --release -p xcl --test gpu_dxil_parity -- --nocapture --test-threads=1
```

Cross-lane/DPP and MIMG passed. The LDS atomics/barrier test failed at `CreateComputePipelineState` (`0x8007000E`) on direct DXIL in split wave64. The test's DXIL validation passed before PSO creation. A fresh process reproduced the same failure. The clean test attempts direct DXIL first and stops at that failure.

A diagnostic copy ran both backends independently before asserting. Each cell describes the direct DXIL and HLSL + DXC result together.

| Fixture and mode | PR 1778 baseline | With PR 1919 shader lowering |
| --- | --- | --- |
| Cross-lane/DPP, split wave64 and wave32 | Both passed in both modes | Both passed in both modes |
| Image ops and image atomics, split wave64 and wave32 | Both passed in both modes | Both passed in both modes |
| LDS atomics/barrier, split wave64 | Both refused PSO creation with `0x8007000E` | Both passed, including output comparison |
| LDS atomics/barrier, wave32 | Both refused PSO creation with `0x8007000E` | Both passed, including output comparison |

For the candidate, I applied only `gpu/dxil/lower_mem.rs` and `gpu/recomp/emit.rs` from [XCL PR 1919](https://github.com/xcl-dev/xcl/pull/1919) head `a081eb52220dbc746b63b9e1eec04ba7481ef59f` onto the PR 1778 tree. Those changes implement the single-atomic LDS subtract lowering. Both baseline and candidate used the same diagnostic fixture and assertions. The candidate passed all three tests, with zero failures, ignores or GPU skips. The LDS test passed again in three fresh processes, each covering both backends in both wave modes. Both LDS backends declared a single 32 KiB groupshared array before and after the change.

This comparison covers the synthetic fixture on the two supported wave modes. Retail shader behavior, gameplay timing and the current XCL main branch remain unverified by this test.

OpenAI Codex (GPT-6) ran the tests, added local logging to compare the backends and drafted this report.
