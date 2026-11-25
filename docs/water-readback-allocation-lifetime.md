# Guest allocation lifetime for water readback

The original Series X boat comparison used `e4c18ec`. A separate native run tested the guarded code at `b97bb825e6d2f7dda59c7c7acc95501b97e2cb00` with the same two water planes.

Geckomacabre implemented the original texture readback. In fork PR 999, cxy0072019-sketch added allocation-lifetime checks at `04df435`. That change keeps the native texture alive and discards pending guest writes after resource destruction, same-base texture replacement or overlapping mapping changes. It also declares the copyback destinations as pending writes to the submission scheduler. [Original implementation](https://github.com/Geckomacabre/vx-layer/commit/eee5667d) · [Upstream guard](https://github.com/fxceless/victory-fork/commit/04df4350876fa759a7e7e134c8e68a64cbb9e2f7)

The local guard records the eligible owner when the resource is created. Creating **any resource at the same guest base**, including a buffer, retires that owner. Even if the old texture remains alive and a later list asks to record it, the old owner can no longer acquire a readback ticket. PR 999 checks the tickets already outstanding when a replacement texture is created. Its separate scheduler change remains outside the local guard.

A runtime-reported overlapping remap invalidates prior tickets while allowing the surviving owner to receive a fresh ticket on a later shader write. A last guest release also invalidates pending work. The landing path holds the mapping read guard and allocation lock across the final guest-memory access. The local tests cover adjacent, empty and partial remaps, destruction and address reuse, and replacement by an ineligible resource. Aliases at other virtual addresses need a corresponding remap notification to be detected. Guest-side aliasing outside those notifications and the guard’s in-game performance remain unmeasured.

## Validation

Windows diagnostics build, one test thread. Run `cargo test --offline --locked --profile diag -j 1 -p victory-gpu --features diagnostics --lib -- --test-threads=1`. All four water tests passed. The full suite reported **194 passed, one failed, six ignored**. The WARP indirect-draw budget test failed the same assertion in isolated runs on both `e4c18ec` and `b97bb82`. Five ignored tests are opt-in measurements. The sixth writes a Windows Application event. Two queue tests returned early with the normal adapter and both passed when rerun with `VICTORY_D3D12_WARP=1`, `VICTORY_CARD_BUFFERS_MIN_MB=1`, `VICTORY_QUEUE2=1` and `VICTORY_RELAY=2`.

## Guarded boat trial

One fresh Series X process ran on a native RTX 5080, driver 617.14, Windows 11 Pro 26200.9974 and Ryzen 9 5900X. The guarded and original builds used identical archive and controller overlays. The guarded process started from fresh, hash-matched copies of the same nine-file profile and 818-file cache, with `VICTORY_TEXTURE_BACK=64x64f16,128x128f36`.

The earlier disabled control showed the “Cal fell off the boat” failure reason 44.0 seconds after its first captured “Follow the plane” objective. In the guarded process the hull and crew stayed afloat throughout that interval. The first captured “Collect the package” objective appeared 52.9 seconds after “Follow the plane”, while the controller was idle. The first throttle input followed 5.0 seconds later. After throttle, successive images spanning 17.7 seconds showed the moving hull and twin wakes. A short starboard input produced a bank and a change in heading relative to the bridge. The operator ended the run with the package objective active. The original save verified against its pre-run hashes.

The completed-write log reached **1,024** for 64×64 format 16 and **256** for 128×128 format 36. These are lower bounds because each counter logs at powers of two. The stale-landing counter emits its first skip and then powers of two. It emitted no event in this run. Synthetic tests supply the evidence for stale-destination rejection. This native trial evaluated the two selectors together and collected no comparative speed measurement.
