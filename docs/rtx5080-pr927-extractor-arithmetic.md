# Ordinary format loads feeding low-half coordinate arithmetic

Three ordinary format loads through an `R16_UINT` view produced the expected coordinate values for every finite half encoding on the RTX 5080 and WARP. We tested this because the facial extractor reads X through float-typed DXIL, then interprets its low 16 bits as a half. The PR author reported colored squares on the opening waitress’s skin. [Existing facial-output measurements](rtx5080-pr927-facial-output.md) · [Author’s report](https://github.com/fxceless/victory-fork/pull/927#issuecomment-7164653246)

## Numeric fixture

The fixture assembles three GFX10 `buffer_load_format_x` instructions against one descriptor at table dword 16. Each thread reads three adjacent 16-bit elements. Three `v_fma_mix_f32` instructions consume the low halves with multiplier 1 and coordinate offsets of 2, −1 and 0.5. The host reads back the FP32 result bits. The synthetic root table is parameter 1. The observed game binding was parameter 0.

The CPU reference for `R16_UINT` zero-extends the stored element, decodes its low half and adds the coordinate. For `R16_FLOAT`, it first widens the stored binary16 number to FP32, takes the low 16 bits of that FP32 representation and decodes those bits as a half before the addition. For example, source `0x3c00` plus coordinate 2 produces 3 on the unsigned view and 2 on the float view. A third program uses the PR’s explicit `buffer_load_format_d16_x` integer-view path as a matched control.

The test covers 63,488 finite binary16 patterns at each of three input positions, including both signed zeros and every finite subnormal. It compares the resulting coordinate sums. The nonzero addends can erase the distinction between signed zeros. NaNs, infinities, vertex processing and the final skin pixels are outside this fixture.

## Measured results

On 2026-11-25, both the NVIDIA GeForce RTX 5080 with driver 617.14 and Microsoft WARP returned the expected FP32 bits for every coordinate. The host ran Windows 11 Pro 25H2 build 26200.9974. The table uses the unmodified production translator from [PR 927 head `befaf3b`](https://github.com/fxceless/victory-fork/pull/927/commits/befaf3b5aa91966d2569985eeba808d4d8017026), with the synthetic fixture added.

| Guest load and host view | RTX 5080 correct / compared | WARP correct / compared |
| --- | ---: | ---: |
| Ordinary format X, `R16_FLOAT` | 190,464 / 190,464 | 190,464 / 190,464 |
| Ordinary format X, `R16_UINT` | 190,464 / 190,464 | 190,464 / 190,464 |
| Explicit D16 format X, `R16_UINT` | 190,464 / 190,464 | 190,464 / 190,464 |

The same fixture on this repository’s `977b8df` facial diagnostics branch also matched all 190,464 expected coordinate sums per case on WARP.

The ordinary float-annotated load returned the intended unsigned-view arithmetic in this isolated three-load sequence. The CPU reference values for the float and unsigned views differ in 190,454 of 190,464 coordinate sums. Treating a valid float view as unsigned would change those results. These measurements narrow the search to differences between this synthetic dispatch and the affected draw, including its inputs, descriptor lifetime and downstream processing. A game-level fix still needs a matched scene and evidence of where values first diverge.

On a Windows SM 6.6 system, run `cargo test --profile diag --locked -j 1 -p victory-shader --lib format_x_low_half_arithmetic_on_ -- --nocapture --test-threads=1`. To reproduce the upstream revision in the table, apply this commit’s `valu_tests.rs` and `valu_tests/format_x_low_half.rs` changes to `befaf3b` before running the command. Each test prints its adapter name and comparison totals. An unavailable adapter prints a skip message.

## Separate observation on the gameplay baseline

On `92a72df` with the facial change off, the RTX 5080 rendered multicolored speckling on the exposed side of a uniformed officer’s head and neck in the outdoor crowd after the arrest. Surrounding hair and uniform had clear boundaries. A different indoor shot of a similarly dressed woman showed smoothly shaded cheek and neck skin. The lighting, angle and distance differ between these shots.

In the outdoor frame, 3,567 of 7,680 horizontal and vertical pixel pairs four pixels apart differed by at least 12 levels of green minus the mean of red and blue in an interior 64 × 64 pixel skin region. The 95th percentile of the absolute difference was 32.5 on the 8-bit screenshot scale. Same-size regions of the nearby hair, uniform and bystander’s head each had zero pairs above that threshold and a 95th percentile of 1.0. The indoor cheek and below-ear regions also had zero pairs above the threshold. These numbers describe that frame’s color variation. A matched scene across builds is still needed to assess whether the PR changes this symptom. The author's waitress shot is a different character and scene.
