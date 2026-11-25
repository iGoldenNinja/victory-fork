# XCL split-wave64 on an RTX 5080

Tested on Windows 11 Pro build 26200, Ryzen 9 5900X, 32 GB RAM, NVIDIA GeForce RTX 5080, driver 617.14, and DXC/DXIL 1.9.2609.5. XCL's adapter report confirmed D3D12 wave lanes 32–32. XCL main `cbc296ab65a2661a61e3e2da109134bcd5319f60` was built in MSVC release mode.

## Four previously rejected compute pipelines

Each of the four local inputs matched one compute PSO previously rejected by VIctory with `0x80070057`. Their console thread groups were 64, 64, 128 and 256 threads. VIctory's diagnostics recorded an LDS read and write in a lane-conditional region for all four.

| XCL treatment | Emitted wave size | PSOs created | PSOs refused |
|---|---|---:|---:|
| Default HLSL path, split explicitly off | 64 | 0 | 4 |
| Default HLSL path, split explicitly on | 32 | 4 | 0 |
| Direct DXIL path, split on | 32 | 4 | 0 |

The HLSL runs used separate empty XCL DXIL caches. Both compiled all four inputs, and neither loaded a driver blob from XCL's disk cache. DXIL disassembly confirmed the HLSL paths emitted four `WaveSize=64` and four `WaveSize=32` modules respectively. The direct backend validated all four and created the same four host PSOs. Its per-lane mode fell back to the per-instruction form on these inputs. These tests created PSOs without dispatching game shaders.

Separately, six upstream synthetic split-wave64 GPU tests and `console_wave64_compute_through_the_pso_hook_dispatch_and_read_back` ran successfully on the RTX 5080. Those tests execute and read back synthetic programs. Game shader numerical output and any improvement to gameplay FPS remain unverified.

Commands for a local directory that contains only the four private compute inputs:

```powershell
$env:XCL_ADAPTER = '5080'
xcl shaders $inputs --precompile --only cs --pipelines --split 0
xcl shaders $inputs --precompile --only cs --pipelines --split 1
xcl shaders $inputs --direct --sample 0 --validate --threads 1 --json
```

## Retail graphics and title startup

The retail GPU suite executed all seven tests with the same input archive. Six passed. `retail_es_gs_voxelize_geometry_mode` failed the dominant-axis geometry projection check with the same values reported on driver 617.42 in the XCL GPU thread. The other tester reported a pass with driver 621.24. This system ran 617.14. [Local GPU results in the XCL thread](https://github.com/xcl-dev/xcl/issues/42#issuecomment-7152366749)

On a clean profile with the stock HLSL runtime, a 300-second title run through a local read-through package view produced black pixels in all 11 internal back-buffer snapshots. A launcher-only variant that pointed to the ordinary SATA SSD package directory reached the visible Player 1 screen in two runs. It booted a verified copy of the executable inside a private title data directory. With the existing XCL `XCL_PRESS_A_AT=72` controller test, the title registered A, rendered the animated intro, and reached the first Navigation Experience setup screen. The runs shared the XCL shader cache. The result establishes a startup path through the first setup screen. Gameplay, keyboard control, and performance in a playable scene remain unverified in XCL.

[Source revision](https://github.com/xcl-dev/xcl/commit/cbc296ab65a2661a61e3e2da109134bcd5319f60) · [XCL wave64 design](https://github.com/xcl-dev/xcl/blob/cbc296ab65a2661a61e3e2da109134bcd5319f60/docs/wave64-on-wave32.md) · [Related VIctory report](https://github.com/Marvisak/VIctory/issues/71#issuecomment-7151627014)

AI-assisted testing and reporting with OpenAI Codex.
