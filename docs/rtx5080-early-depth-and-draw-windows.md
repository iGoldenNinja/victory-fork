# Early depth and indirect draw windows on RTX 5080

The test machine used an RTX 5080 16 GB with driver 617.14, Windows 11 Pro 25H2 build 26200.9974, a Ryzen 9 5900X, and 32 GB RAM. The baseline `bbc35d060d2e131f7da814f2ed63b29348afd25d` combines NVIDIA `a253706` with split indirect dispatch and paired Wave64 compute. The candidate `74becd8b03e372e44b326e1e419819757f277440` adds the early-depth change from PR 907 at `6f358fc6d10d9a49da4e1de0ccae882123c40208` and a correction to the driver watcher. Both binaries used the same local archive and input support. [Early-depth PR](https://github.com/fxceless/victory-fork/pull/907) · [Driver-watcher fix](https://github.com/fxceless/victory-fork/pull/897#issuecomment-7162654669)

## Early depth

I matched ten alpha prepass programs from the baseline's live pipeline logs. All ten have `DB_SHADER_CONTROL=0x1450`. Both binaries emitted 10 of 10, validated their DXIL and created all ten offline D3D12 test pipelines on the RTX 5080. The native early-depth regression passed on the candidate. All ten programs built and bound during the candidate's successful launch.

Both Series X runs reached a controllable cell. At 2542 × 1396 before camera movement, the candidate showed the wall courses, wall decoration, and more of the character's clothing. The prominent dark band across the shoulders in the baseline was reduced in the candidate. In the same 250 × 120 left-wall region, mean encoded luma on the 0–255 scale was 7.39 in the baseline and 44.87 on the candidate. The sky region seen through the window was 185.45 and 196.80 respectively. Draw windows were off in both captures. These were separate launches with their own shader-cache history and naturally different idle animation poses. Intermediate depth and motion targets remain to be checked.

The candidate's first launch ended during world entry after a 1 MiB host allocation failed. Minimum sampled system commit headroom was about 0.01 GiB. The same executable passed that point on an isolated retry with its already used shader cache and a fresh copy of the starting profile.

## Draw windows

In the successful launch, a diagnostics-only marker turned `VICTORY_INDIRECT_WINDOW` on and then off again, with the driver watcher enabled. `VICTORY_COPY_PLAIN` was off. Each setting had a 30-second displayed segment. The client stayed at 2542 × 1396. The captured viewpoint changed between phases and a performance overlay appeared in the enabled capture. The draw-count change limits the frame-rate comparison, even within one process.

| Setting | Displayed FPS | P99 interval | Mean PresentMon GPU busy | Median draws per frame |
| --- | ---: | ---: | ---: | ---: |
| Off before | 2.985 | 393.94 ms | 214.42 ms | 4,949 |
| On | 3.446 | 629.10 ms | 178.53 ms | 5,079 |
| Off after | 3.813 | 358.66 ms | 203.69 ms | 3,847 |

In periodic host reports with 4,800–5,300 draws per frame, median reported GPU time was 179.45 ms across six initial off samples and 78.40 ms across five on samples. The final off segment was outside that draw range. The enabled setting used less measured GPU time in the overlapping draw range, while the display tail remained slow. A repeat with the same viewpoint and overlay state is required to establish the FPS effect. No driver event or device removal was logged, and the run ended deliberately. [Upstream workload correction](https://github.com/fxceless/victory-fork/commit/0b0074e0b2765e574c622e9a3de566f20f04135b)

Testing and reporting assisted by OpenAI Codex.
