# ZTRX divergent barrier test on the RTX 5080

Windows 11 Pro build 26200, Ryzen 9 5900X, 32 GB RAM, GeForce RTX 5080 16 GB, driver 617.14 (`32.0.16.1714`), and DXC/DXIL validator 1.9.2609.5. Both ZTRX revisions were built in the `diag` profile with MSVC and clean checkouts.

## Synthetic dispatch

ZTRX PR 30 at `52e06e5c18b78d37bf6b2e559b3c234e7211a105` passed all three tests selected by `automatic_barrier`. The hardware test confirmed D3D12 wave max 32 and actually dispatched with 1, 10, 32, 33, and 64 active lanes. Each dispatch returned the expected value of 19. The two other tests exercised barrier mode selection and emitted DXIL flags for both single- and multi-console-wave groups.

```powershell
cargo test --profile diag --locked -p victory-shader --lib automatic_barrier -- --nocapture --test-threads=1
```

The synthetic program checks completion and a UAV readback. Correct numerical results for game shader reads and writes between the two halves of an emulated wave remain unmeasured.

## Four previously rejected compute inputs

| ZTRX source | Compute modules validated | D3D12 compute PSOs created |
| --- | ---: | ---: |
| Base `90b529f878a470fec9bf07b7be316bc769a12232` | 4 of 4 | 0 of 4 |
| PR 30 `52e06e5c18b78d37bf6b2e559b3c234e7211a105` | 4 of 4 | 0 of 4 |

All four rejections were incorrect-parameter errors on both revisions. The same four inputs were used in the previous XCL split-wave64 comparison. The console thread groups were 64, 64, 128, and 256. PR 30's automatic mode only removes the divergent title barrier's thread-group wait for eligible single-console-wave groups. The two larger groups are outside this rule. The offline title sample ran through validation and PSO creation only.

```powershell
$env:VICTORY_HOST_MAX_WAVE = '32'
$env:VICTORY_GROUP_SYNC = $null
$env:VICTORY_WAVE_WHY = '1'
victory translate --jobs 1 --validate --pipelines --list $inputs
```

[ZTRX PR 30](https://github.com/Zettrox/ztrx/pull/30) · [Same-machine XCL split-wave64 comparison](https://github.com/iGoldenNinja/victory-fork/blob/fb252dfa345c491efad433ecd52df44b1a9b3dfb/docs/xcl-wave64-5080.md)

Testing and reporting assisted by OpenAI Codex.
