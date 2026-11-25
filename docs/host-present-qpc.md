# Match guest presents to DXGI and PresentMon

Set `VICTORY_PRESENT_QPC=1` in a Windows diagnostics build. The `victory_present_qpc` log target records one clock-frequency line and the following events. Each guest call receives a unique `seq`, carried through the asynchronous queue when used.

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
