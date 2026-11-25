# Pixel quad reads at triangle edges on RTX 5080 and WARP

On November 25, 2026, both the parent and the changed emitter matched every pixel of a game-free D3D12 test. The host ran Windows 11 Pro build 26200.9974. The hardware adapter was an NVIDIA GeForce RTX 5080, driver 617.14. The software adapter was Microsoft Basic Render Driver (WARP). [Upstream change and RX 6700 XT comparison](https://github.com/Marvisak/VIctory/commit/4015a182e3999bbaa881f6831360eb257f168c82)

Each cell counts pixels with any incorrect RGBA channel among 256 target pixels. The diagonal triangle covers 136 pixels, including 24 pixels in eight quads that straddle its edge. The remaining 120 pixels are checked against a cleared background.

| Adapter | Parent, full triangle | Parent, diagonal edge | Changed, full triangle | Changed, diagonal edge |
|---|---:|---:|---:|---:|
| NVIDIA GeForce RTX 5080 | 0 | 0 | 0 | 0 |
| Microsoft Basic Render Driver | 0 | 0 | 0 | 0 |

The parent is `a1a2077a3d9ab6bf51d457c1a3f86db166554446`. The changed emitter is `4015a182e3999bbaa881f6831360eb257f168c82`. Both builds used the same fixture. The parent takes the wave-read path for this pixel program. The change uses quad-local reads at triangle edges even when there is no discard.

## What the fixture measures

A hand-authored wave32 GFX10 pixel program computes `v = (x + 2*y)²` from pixel-center coordinates. Four `v_mov_b32` DPP permutations select the horizontal neighbor, vertical neighbor, diagonal neighbor and top-left lane. The test runs the guest instruction words through the project's decoder, resource and input analysis, and DXIL emitter. It supplies the pixel input descriptor and uses PC DXC for the signature and reference shaders.

For each geometry the test compares three draws against the CPU coordinate formula. The PC shader with quad intrinsics and the PC control with four direct `WaveReadLaneAt` operations also matched all 256 pixels on both adapters. The pixel at `(0, 0)` read `[6.25, 12.25, 20.25, 2.25]` in each. This result establishes behavior for this DPP program on the tested driver. Upstream reported a failing pre-change RX 6700 XT case using console-compiled fine derivatives. This fixture exercises neither early discard nor a title shader. It gives no measurement of appearance or performance in a game.

## Repeat

Use a Windows checkout of this evidence branch. Set `VICTORY_PC_DXC` to a PC DirectX Shader Compiler executable supporting shader model 6.5 and `VICTORY_DXIL` to its matching validator DLL. The two opt-in tests select hardware and WARP and print each adapter's name. The D3D12 test host requires shader model 6.6.

```powershell
cargo test --profile diag -p victory-shader --lib adjacent_quad_pixels_on_ -- --ignored --nocapture --test-threads=1
```

To test the immediate parent with the identical fixture, run the following from the evidence branch, then repeat the command above inside the new worktree.

```powershell
git worktree add --detach ../quad-parent a1a2077a3d9ab6bf51d457c1a3f86db166554446
git -C ../quad-parent cherry-pick --no-commit test/rtx5080-pixel-quad-edges
```
