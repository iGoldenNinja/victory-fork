//! Correlate a title present with its host DXGI call on the native QPC timeline.
#[derive(Clone, Copy)]
pub(crate) struct Trace(u64);

impl Trace {
    pub(crate) fn begin() -> Self {
        #[cfg(all(windows, feature = "diagnostics"))]
        {
            use std::sync::{
                OnceLock,
                atomic::{AtomicU64, Ordering},
            };
            static ENABLED: OnceLock<bool> = OnceLock::new();
            static SEQUENCE: AtomicU64 = AtomicU64::new(1);
            if *ENABLED.get_or_init(|| {
                if !std::env::var_os("VICTORY_PRESENT_QPC").is_some_and(|v| v == "1") {
                    return false;
                }
                let mut hz = 0;
                // SAFETY: QPC frequency is written to the provided local variable.
                let ok = unsafe {
                    windows_sys::Win32::System::Performance::QueryPerformanceFrequency(&mut hz)
                };
                if ok != 0 && hz > 0 {
                    tracing::info!(target: "victory_present_qpc", qpc_hz = hz, "clock");
                    true
                } else {
                    tracing::warn!("present trace disabled: QPC frequency is unavailable");
                    false
                }
            }) {
                let seq = SEQUENCE.fetch_add(1, Ordering::Relaxed);
                tracing::info!(target: "victory_present_qpc", seq, qpc = qpc(), "guest_begin");
                return Self(seq);
            }
        }
        Self(0)
    }

    pub(crate) fn guest_end(self, frame: u64, hr: i32, deferred: bool) {
        if self.0 != 0 {
            tracing::info!(target: "victory_present_qpc", seq = self.0, frame, hr, deferred, qpc = qpc(), "guest_end");
        }
    }

    #[cfg(windows)]
    pub(crate) fn skip(self, frame: u64, reason: &'static str) {
        if self.0 != 0 {
            tracing::info!(target: "victory_present_qpc", seq = self.0, frame, reason, qpc = qpc(), "host_skip");
        }
    }

    #[cfg(windows)]
    pub(crate) fn physical(
        self,
        frame: u64,
        swap_chain: usize,
        call: impl FnOnce() -> windows::core::HRESULT,
    ) -> windows::core::HRESULT {
        if self.0 == 0 {
            return call();
        }
        // Keep an entry if DXGI never returns. Exclude the cost of logging it from the call duration.
        tracing::info!(target: "victory_present_qpc", seq = self.0, frame, swap_chain, qpc = qpc(), "dxgi_enter");
        let before = qpc();
        let hr = call();
        let after = qpc();
        tracing::info!(target: "victory_present_qpc", seq = self.0, frame, swap_chain,
            before, after, hr = hr.0, "dxgi_present");
        hr
    }
}

fn qpc() -> i64 {
    #[cfg(windows)]
    {
        let mut ticks = 0;
        // SAFETY: Windows writes the QPC reading to the provided local variable.
        unsafe { windows_sys::Win32::System::Performance::QueryPerformanceCounter(&mut ticks) };
        ticks
    }
    #[cfg(not(windows))]
    {
        0
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use windows::core::HRESULT;

    #[test]
    fn present_qpc_keeps_every_host_result_and_calls_once() {
        use std::cell::Cell;
        // Includes a positive DXGI status and an error, which a Result<()> would lose.
        for raw in [0, 0x087a0001, 0x887a0005u32 as i32] {
            for seq in [0, 7] {
                let calls = Cell::new(0);
                let hr = Trace(seq).physical(81, 0x1234, || {
                    calls.set(calls.get() + 1);
                    HRESULT(raw)
                });
                assert_eq!(hr.0, raw);
                assert_eq!(calls.get(), 1);
            }
        }
    }

    #[test]
    #[ignore = "requires a Windows desktop and the D3D12 WARP adapter"]
    fn present_qpc_on_hidden_warp_swapchain() -> windows::core::Result<()> {
        use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_12_0;
        use windows::Win32::Graphics::Direct3D12::*;
        use windows::Win32::Graphics::Dxgi::Common::*;
        use windows::Win32::Graphics::Dxgi::*;
        use windows::Win32::UI::WindowsAndMessaging::*;
        use windows::core::{Interface, w};

        // SAFETY: all D3D12/DXGI descriptors are local, devices outlive their
        // queues and swap chains, and this thread owns the hidden test window.
        unsafe {
            let adapter = crate::host::warp_adapter()?;
            let mut device: Option<ID3D12Device> = None;
            D3D12CreateDevice(&adapter, D3D_FEATURE_LEVEL_12_0, &mut device)?;
            let queue: ID3D12CommandQueue = device
                .unwrap()
                .CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC::default())?;
            let factory: IDXGIFactory2 = CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0))?;
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!("present QPC fixture"),
                WS_POPUP,
                0,
                0,
                32,
                32,
                None,
                None,
                None,
                None,
            )?;
            let desc = DXGI_SWAP_CHAIN_DESC1 {
                Width: 32,
                Height: 32,
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                BufferCount: 2,
                SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
                ..Default::default()
            };
            let shown = (|| -> windows::core::Result<HRESULT> {
                let chain: IDXGISwapChain3 = factory
                    .CreateSwapChainForHwnd(&queue, hwnd, &desc, None, None)?
                    .cast()?;
                let begin = qpc();
                let result = Trace(7).physical(81, chain.as_raw() as usize, || {
                    chain.Present(0, DXGI_PRESENT(0))
                });
                let end = qpc();
                assert!(begin > 0 && end >= begin);
                eprintln!(
                    "hidden WARP physical present hr=0x{:08x} elapsed_qpc_ticks={}",
                    result.0,
                    end - begin
                );
                Ok(result)
            })();
            DestroyWindow(hwnd)?;
            shown?.ok()?;
        }
        Ok(())
    }
}
