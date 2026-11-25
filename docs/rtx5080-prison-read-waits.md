# Title-memory read waits in the first prison room on RTX 5080

The first blocking read was a 2,048-byte buffer copy for **78.54% of measured title-read wait time**. Its source changed across 197 of 218 checks. Upload fills of 480 bytes accounted for another 20.48% and their source bytes were identical across all 293 checks.

| First blocking read | Requested bytes | Timed waits | Wait time | Share of read wait | Source changed / checked | Destination changed / checked |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Buffer copy | 2,048 | 218 | 29.651087 s | 78.54% | 197 / 218 | 0 / 218 |
| Upload fill | 480 | 293 | 7.729521 s | 20.48% | 0 / 293 | — |
| Buffer copy | 20,480 | 143 | 0.369907 s | 0.98% | 0 / 143 | 0 / 143 |

The 64 complete host reporting intervals covered 73.523 seconds and recorded 37.750515 seconds of title-read waits. The 480-byte group represented 10.51% of that wall time. This bounds the wait charged to that group on this run. A settle can serve several reads and overlap other work, so a frame-time improvement would require a separate controlled test. During world entry the existing safeguard caught a 2,048-byte speculative read whose bytes changed and restored waiting.

## Native run

On Windows 11 Pro build 26200.9974, Ryzen 9 5900X, RTX 5080 16 GB, NVIDIA 617.14 and SATA SSD, the Series X diagnostic run reached the idle prison window. The title process recorded 3D and copy activity on the NVIDIA adapter. Two consecutive views matched the existing prison reference at 4,758 and 5,018 draws per frame. The replay stayed idle during measurement. The controller ended the run after capture with no logged device removal.

PresentMon recorded **223 display intervals in 75.133 seconds, 2.968 FPS**, with P99 405.738 ms, maximum 423.473 ms and mean GPU-busy time 210.48 ms. Twelve host timing reports had median values of 5,054 draws per frame and 193.8 ms copying back. A separate one-job CPU build ran during startup. Fourteen checks at approximately five-second intervals detected no Cargo or Rust compiler process in the marked window. No other native GPU test ran concurrently.

## Reproducing the counters

1. Build revision `cbcdedab74a6ceafa070f3e8ba046d61db317a14` with `cargo build --locked --profile diag -p victory --features diagnostics`. The probe applies to Windows diagnostics builds. The local run also used the established host replay and read-only archive shims.
2. Set `VICTORY_READ_WAIT_STATS=1` and `VICTORY_LOG=info`. Keep relay and wait switches at their defaults. The tested runtime reported `VICTORY_RELAY=2` and had DRED enabled.
3. Reach a repeatable idle world view, mark its start and end, and sum `wait_us` from complete reporting intervals. The `title read wait attribution` rows provide read kind, requested bytes, timed intervals and successful source/destination comparisons. Each relay wait is charged to the first blocker. A byte comparison can arrive in the next interval. Leaving the switch unset disables the extra counters.

The existing RTX 5070 Ti thread discusses aggregate title-read wait time. A separate PR investigates moving constant-buffer fills onto the GPU. The measurements above use the RTX 5080 source revision and configuration given here. [Frame-time discussion](https://github.com/fxceless/victory-fork/issues/696) · [Constant-buffer fill work](https://github.com/fxceless/victory-fork/pull/522)
