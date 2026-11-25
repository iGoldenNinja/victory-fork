# VIctory PR 2241 texture atomics on RTX 5080

I tested clean PR head `e2624b3384813a444067fe9ba93dfd131473a63c` and its direct parent `ffe310eb2098e52169f4c44db5ffdabf43a72622` on Windows 11 Pro 25H2 build 26200.9974 with an RTX 5080 16 GB, NVIDIA driver 617.14, Ryzen 9 5900X and 32 GB RAM. The builds used MSVC Rust 1.98.1 and `dxil.dll` 1.9.2609.5. GitHub reports conflicts with the current target `main`. [VIctory PR 2241](https://github.com/Marvisak/VIctory/pull/2241)

## Synthetic GPU readback

The PR's atomic add and compare-swap tests both passed with hardware and WARP and no adapter skips. For min and max, I added a test-only fixture to the same head. It seeds a texel by swapping in `0x8000000000000002`, then applies the specified atomic with `0x7ffffffffffffffe`. The inputs order differently as signed and unsigned 64-bit values and in opposite order if compared one dword at a time. Both adapters returned the seeded value through the old-value readback and produced the expected final texel in all four cases. [Runnable synthetic fixture patch](https://github.com/iGoldenNinja/victory-fork/blob/a08152888c521b73d6f64b7b369f49f9705f1338/docs/fixtures/victory-pr2241-minmax.patch)

| Atomic | Expected and observed final texel |
| --- | --- |
| `smin` | `0x8000000000000002` |
| `umin` | `0x7ffffffffffffffe` |
| `smax` | `0x7ffffffffffffffe` |
| `umax` | `0x8000000000000002` |

## Matched pixel programs

I ran `translate --jobs 2 --validate --pipelines --list` on the same 169 inputs for both builds. The CLI deduplicated one input. The matched pixel programs contain a two-dword `image_atomic_umax` with `glc` off.

| Revision | DXIL emitted | Validated | D3D12 pipelines created | Remaining refusal |
| --- | ---: | ---: | ---: | --- |
| Direct parent `ffe310e` | 0 of 168 | 0 | 0 | 168 × `a 64-bit image atomic` |
| PR head `e2624b3` | 162 of 168 | 162 | 162 | 6 × `barycentrics read directly` |

All 162 emitted programs validated and created an offline D3D12 pipeline. The six remaining programs passed I/O analysis before being refused when lowering direct reads of barycentrics. Synthetic fixtures reached GPU execution and readback. The matched pixel programs reached pipeline creation, so the effects on gameplay rendering and frame rate remain unverified.

## Combined retest with PR 453

PR 453 adds DXIL `SV_Barycentrics` inputs for shaders that read those values directly. I applied the exact PR 2241 commit to clean PR 453 head `88b34a447350eeb1e747be84e4204dbb3955c58c`. The changes combined without manual edits. I rebuilt with the same toolchain and reran the six remaining programs on the same RTX 5080. All six emitted, validated and created D3D12 test pipelines. [Barycentric lowering](https://github.com/Marvisak/VIctory/pull/453)

The full retest used the same 169 inputs, with one duplicate. All **168 unique pixel programs** emitted, validated and created D3D12 test pipelines, including the 162 that had worked with PR 2241 alone. The two branches share the older `ffe310e` base. The combined build establishes offline pipeline coverage for this set. Gameplay output remains unverified.
