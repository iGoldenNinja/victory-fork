# ZTRX LDS and integer blend checks on the RTX 5080

Tested ZTRX PR 36 at `1402bf7b821fc4ca9c28073debed28e8ee65fb0d` on Windows 11 Pro build 26200, Ryzen 9 5900X, 32 GB RAM, RTX 5080 16 GB, NVIDIA driver 617.14 and DXC/DXIL validator 1.9.2609.5. The worktree was clean and built with MSVC in the `diag` profile.

## LDS flag case

I used the same four matched wave64 inputs from the earlier [ZTRX PR 30 comparison](https://github.com/Zettrox/ztrx/pull/30#issuecomment-7152711800). Tests ran in separate processes with the same binary and `VICTORY_HOST_MAX_WAVE=32`.

| `VICTORY_LDS_FLAGS` | Validated compute modules | D3D12 compute PSOs created |
| --- | ---: | ---: |
| `0` | 4 of 4 | 0 of 4 |
| `1` | 4 of 4 | 1 of 4 |

The one change was `2cc4c197ee29e291`, which returned an incorrect-parameter error with the flag off and created a pipeline with it on. The other three remained refused. Both focused shader fixtures passed. The fixed-value flag region produced wave32. The two-value region retained wave64. The sampled game program was checked through pipeline creation. Its numerical execution and frame-rate impact remain unmeasured.

```powershell
$env:VICTORY_HOST_MAX_WAVE = '32'
$env:VICTORY_LDS_FLAGS = '0' # Run again with '1' in a new process.
victory translate --jobs 1 --validate --pipelines --list $inputs
```

## Integer blend observations

The three selected `pipeline::host` tests passed, including the two blend-state fixtures. The exact mesh/pixel pair in PR 36, `f5e50eb2108c425b` and `ce3f0432a08f1d35`, logged an `0x80070057` host mesh pipeline refusal on both prior RTX 5080 title runs, on NVIDIA base `310c6eb` and NVIDIA PR 688 `7e7bb18` with staging on. The signature is present in both runs, so this finding alone cannot explain the visual difference between them. PR 36's fixed blend behavior in the live title remains unverified on this machine.

[ZTRX PR 36](https://github.com/Zettrox/ztrx/pull/36) · [Earlier 5080 base and readback test](https://github.com/iGoldenNinja/victory-fork/blob/91f4337cd49c7a2dd81e2bb40495bcad794df945/docs/ssd-readback-pr688-5080.md)

Testing and reporting assisted by OpenAI Codex.
