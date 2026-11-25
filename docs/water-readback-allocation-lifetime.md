# Guest allocation lifetime for water readback

The Series X boat run used `e4c18ec`. The later guard at `b97bb825e6d2f7dda59c7c7acc95501b97e2cb00` has synthetic Windows test coverage. The boat outcome on the guarded revision remains unverified.

Geckomacabre implemented the original texture readback. In fork PR 999, cxy0072019-sketch added allocation-lifetime checks at `04df435`. That change keeps the native texture alive and discards pending guest writes after resource destruction, same-base texture replacement or overlapping mapping changes. It also declares the copyback destinations as pending writes to the submission scheduler. [Original implementation](https://github.com/Geckomacabre/vx-layer/commit/eee5667d) · [Upstream guard](https://github.com/fxceless/victory-fork/commit/04df4350876fa759a7e7e134c8e68a64cbb9e2f7)

The local guard records the eligible owner when the resource is created. Creating **any resource at the same guest base**, including a buffer, retires that owner. Even if the old texture remains alive and a later list asks to record it, the old owner can no longer acquire a readback ticket. PR 999 checks the tickets already outstanding when a replacement texture is created. Its separate scheduler change remains outside the local guard.

A runtime-reported overlapping remap invalidates prior tickets while allowing the surviving owner to receive a fresh ticket on a later shader write. A last guest release also invalidates pending work. The landing path holds the mapping read guard and allocation lock across the final guest-memory access. The local tests cover adjacent, empty and partial remaps, destruction and address reuse, and replacement by an ineligible resource. Aliases at other virtual addresses need a corresponding remap notification to be detected. Guest-side aliasing outside those notifications and the guard’s in-game performance remain unmeasured.

## Validation

Windows diagnostics build, one test thread. Run `cargo test --offline --locked --profile diag -j 1 -p victory-gpu --features diagnostics --lib -- --test-threads=1`. All four water tests passed. The full suite reported **194 passed, one failed, six ignored**. The WARP indirect-draw budget test failed the same assertion in isolated runs on both `e4c18ec` and `b97bb82`. Five ignored tests are opt-in measurements. The sixth writes a Windows Application event. Two queue tests returned early with the normal adapter and both passed when rerun with `VICTORY_D3D12_WARP=1`, `VICTORY_CARD_BUFFERS_MIN_MB=1`, `VICTORY_QUEUE2=1` and `VICTORY_RELAY=2`.
