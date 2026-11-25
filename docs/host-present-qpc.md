# Match guest presents to DXGI and PresentMon

The trace is off by default. Build with diagnostics on Windows and set the switch in the PowerShell session that launches the instrumented process. Keep the `victory_present_qpc` log target at info level.

```powershell
cargo build --locked --profile diag -p victory --features diagnostics
$env:VICTORY_PRESENT_QPC = '1'
```

The `victory_present_qpc` log target records one clock-frequency line and the following events. Each guest call receives a unique `seq`, carried through the asynchronous queue when used.

| Event | Meaning |
|---|---|
| `clock` | `qpc_hz` is the native performance counter frequency in ticks per second. |
| `guest_begin` | The application entered the console queue's PresentX call. |
| `host_skip` | The renderer could not issue a DXGI present. `reason` identifies the queue or host precondition. |
| `dxgi_enter` | The host reached the DXGI call site. An entry can survive a call that never returns. |
| `dxgi_present` | The actual `IDXGISwapChain::Present(0, 0)` returned. `before` and `after` bracket only the method call, excluding event logging. `hr` is the complete signed 32-bit HRESULT, including positive DXGI status values. |
| `guest_end` | The title-facing call returned its existing `hr`. `deferred=true` means that guest return and later physical presentation can be out of order. |

Convert each absolute counter reading to milliseconds with `ticks * 1000 / qpc_hz`, retaining precision. Match `seq` across application and host events. Match the host `swap_chain` identity and timestamp to the same process in a PresentMon QPC capture. Convert the decimal chain value to hex when comparing it with CSV output. PresentMon's displayed timestamp is `CPUStartQPCTime + DisplayLatency` when both fields are present. A guest frame number, successful DXGI call, and displayed scanout each describe a different stage. Check actual HRESULTs and the counts at each stage before interpreting a missing row. Trace logging itself costs time, so use a dedicated attribution run.

The Windows unit tests exercise plane prechecks and HRESULT propagation. The optional WARP smoke test creates a hidden 32×32 flip-model swap chain and issues one physical present.

```powershell
cargo test --profile diag -p victory-gpu --features diagnostics --lib present_qpc
cargo test --profile diag -p victory-gpu --features diagnostics --lib present_qpc_on_hidden_warp_swapchain -- --ignored --nocapture
```

## Synthetic validation

We tested source revision `b35c8dad961b8776d68d69b6f6bdfb19e35641ea` on Windows 11 Pro build 26200.9974 on November 25, 2026. Both CPU tests above passed. The first exercised the queue's missing-plane and missing-resource checks. The other verified that an instrumented present calls its target exactly once and preserves `S_OK`, the positive `DXGI_STATUS_OCCLUDED`, and the failure `DXGI_ERROR_DEVICE_REMOVED`, with tracing enabled and disabled.

The explicitly selected WARP test also passed. It created a hidden 32×32 Win32 window and a D3D12 flip-model swap chain on Microsoft Basic Render Driver. One actual `Present(0, 0)` returned `S_OK`. A separate synthetic log check on the same host reported a 10,000,000 Hz QPC clock, retained the positive status and a separate skipped-call reason, and confirmed zero trace output with the switch off. The GPU crate passed `cargo check` with default and diagnostic features.

No measured title run has used this diagnostic. Its effect on game frame times and the cause of any in-game display gap remain unmeasured.
