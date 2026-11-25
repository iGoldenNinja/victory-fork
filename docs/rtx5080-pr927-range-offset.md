# PR 927 descriptor-range tests on RTX 5080

PR 927 translates 16-bit typed buffer operations using unsigned views that preserve the channel bits. tippythetaidum implemented the translation. avt0m4t48 identified a displaced SRV whose emitted metadata still named its range start. The upstream shader correction at `befaf3b` passes our native readback. A separate test found that the host could still allocate a fresh descriptor table too short to contain the reported slot. [Original PR](https://github.com/fxceless/victory-fork/pull/927) · [Reviewer's reproduction](https://github.com/fxceless/victory-fork/pull/927#issuecomment-7163487358) · [Upstream correction](https://github.com/fxceless/victory-fork/commit/befaf3b5aa91966d2569985eeba808d4d8017026)

## Translated shader readback

All measurements used synthetic shaders and data on Windows 11 Pro build 26200, NVIDIA GeForce RTX 5080 driver 617.14, and Microsoft WARP. The shader fixture moves four half words per thread to a typed UAV. Its test host starts with FLOAT descriptors and applies the translator's reported UINT substitutions.

| Production emitter | SRV / UAV byte offsets | Reported dwords in root 1 | Matching half words on RTX 5080 | On WARP |
|---|---:|---|---:|---:|
| Original PR `5dc9e63` | 0 / 32 | 0, 8 | 128/128 | 128/128 |
| Original PR `5dc9e63` | 32 / 64 | 0, 16 | 1/128 | 1/128 |
| Updated PR `befaf3b` | 0 / 32 | 0, 8 | 128/128 | 128/128 |
| Updated PR `befaf3b` | 32 / 64 | 8, 16 | 128/128 | 128/128 |

The updated emitter also passed the author's static- and runtime-offset metadata test. The author reports that all 231 d16 accesses in the 65 title programs examined use their declared range starts. The displaced-range examples establish rendering correctness for synthetic inputs. Their effect on in-game faces or frame rate remains unverified. [Author's test and title audit](https://github.com/fxceless/victory-fork/pull/927#issuecomment-7164043623)

## Production table binding

The binder fixture supplies the known half offset directly. Only slot 0 is declared as an SRV range start. A separate one-thread HLSL shader reads slot 0 as FLOAT and slot 2 as UINT through the heap returned by `State::tables`. It uses the normal placed-buffer descriptor records and host buffer refresh.

On `befaf3b` the real allocator reserved **one slot** for the three-slot table on both adapters. The test measured the reservation and closed the command list before dispatching. avt0m4t48 also reproduced the short reservation with a two-slot table on WARP and identified how the selected-slot list affects its size. The follow-up includes accessed elements past the range start in table sizing and buffer refresh. [Reviewer's host-side report and probe](https://github.com/fxceless/victory-fork/pull/927#issuecomment-7164227485)

| Displaced source descriptor | Host binding after correction | Slot 0 read as FLOAT | Slot 2 read as UINT |
|---|---|---|---|
| RGBA16 FLOAT | Three-slot fresh table, reused on the second bind | 1, 2, −1, −2 | `7e01, 03ff, fc01, 8000` |
| RGBA16 UINT | Original four-slot heap, displaced buffer refreshed in place | 1, 2, −1, −2 | `7e01, 03ff, fc01, 8000` |

Both adapters returned those values at `efd753e`. Its bounds checks also rejected an offset within the final descriptor and an offset past the heap before reading either record. The shader and binding tests use different shaders so each can identify which component caused a failure.

## Reproduce

Use the repository's Windows build setup. At the fixture-only commit `ffaff33`, the shader readback passes and the fresh-table allocation check fails with one slot reserved and three expected. Run the same commands on the corrected branch tip. The shader test automatically tries the default hardware device and WARP. The binder runs once per process and uses WARP when the diagnostics setting is enabled.

```powershell
cargo test --profile diag --locked -j 2 -p victory-shader --lib emit::valu_tests::halves_ -- --nocapture --test-threads=1
cargo test --profile diag --locked -j 2 -p victory-shader --lib half_places_are_the_descriptors_the_program_loads
cargo test --profile diag --locked -j 2 -p victory-gpu --features diagnostics --lib half_table_ -- --nocapture --test-threads=1
$env:VICTORY_D3D12_WARP = '1'
cargo test --profile diag --locked -j 2 -p victory-gpu --features diagnostics --lib half_table_ -- --nocapture --test-threads=1
Remove-Item Env:VICTORY_D3D12_WARP
```
