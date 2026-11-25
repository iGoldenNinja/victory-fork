# XCL sparse-memory import on an RTX 5080

Tested the clean head of XCL PR 2090, `5e71371dffbdc8b9f58aa8b58493803e522ed070`, in MSVC release mode. Windows 11 Pro build 26200, Ryzen 9 5900X, 32 GB RAM, GeForce RTX 5080 16 GB, driver 617.14, DXC and validator 1.9.2609.5. XCL's real D3D12.X backend identified the RTX 5080.

- The direct-layout test passed. It checked three separately committed sparse windows and their write-watch regions.
- The single-window placement test passed with the full-size write-watched allocation. D3D12 accepted 64 KiB-aligned overlapping buffer placements at their requested GPU offsets and rejected an unaligned placement.
- I ran `gpu_uma_import_check_at_init` and `gpu_sparse_vram_descriptors_buffers_and_xmem` in separate processes with `XCL_SPARSE_DIRECT=0` and `XCL_SPARSE_DIRECT=1`. All four runs passed with no skips. The enabled runs explicitly logged direct access for all three sparse windows. The disabled runs logged tiled buffers in video memory.

The end-to-end compute fixture reads a descriptor and input buffer in sparse window 0, uses a constant from the coherent window and reads back the resulting GPU output. It verifies expected values for all checked lanes, then modifies the descriptor and input and verifies the second dispatch. This establishes initial import and the tested data path on this adapter. Title performance and lifetime behavior during decommit and fallback were outside this experiment.

To reproduce each mode, set `XCL_ADAPTER=5080`, `XCL_REQUIRE_GPU=1` and set `XCL_SPARSE_DIRECT` to `0` or `1`. Run the fixtures in separate processes.

```powershell
cargo test --locked --release -p xcl --lib gpu_uma_import_check_at_init -- --nocapture --test-threads=1
cargo test --locked --release -p xcl --lib gpu_sparse_vram_descriptors_buffers_and_xmem -- --nocapture --test-threads=1
```

[XCL PR 2090](https://github.com/xcl-dev/xcl/pull/2090) · [Separate decommit report](https://github.com/xcl-dev/xcl/pull/2090#issuecomment-7152248885) · [Separate fallback report](https://github.com/xcl-dev/xcl/pull/2090#issuecomment-7152592306)

Testing and reporting assisted by OpenAI Codex.
