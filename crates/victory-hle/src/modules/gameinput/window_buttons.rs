//! Buttons pressed on a hooked title window between two keyboard samples.
use super::{A, FALLBACK_BUTTONS, X, lock};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging as wm;

// A 30 Hz menu that requests only the current reading can miss an 8 ms reading.
const TAP_TIME: Duration = Duration::from_millis(50);
static STATE: LazyLock<Mutex<Buttons>> = LazyLock::new(Default::default);
static TRACE: LazyLock<bool> = LazyLock::new(||
    std::env::var_os("VICTORY_WINDOW_PAD_TRACE").is_some_and(|v| v == "1"));

#[derive(Default)]
struct Buttons {
    window: usize,
    root: usize,
    held: u32,
    pending: u32,
    until: [Option<Instant>; FALLBACK_BUTTONS.len()],
    last_trace: Option<(bool, u32, u32, u32)>,
}

impl Buttons {
    fn clear(&mut self) {
        self.window = 0;
        self.root = 0;
        self.held = 0;
        self.pending = 0;
        self.until.fill(None);
    }

    fn message(&mut self, window: usize, root: usize, button: u32, down: bool, repeat: bool) {
        if self.window != window {
            self.clear();
            self.window = window;
            self.root = root;
        }
        if down {
            if self.held & button == 0 && !repeat {
                self.pending |= button;
            }
            self.held |= button;
        } else {
            self.held &= !button;
        }
    }

    fn sample(&mut self, now: Instant, foreground: usize, connected: bool) -> u32 {
        if !connected || self.root != foreground {
            self.clear();
            return 0;
        }
        let mut active = self.held;
        for ((_, button), until) in FALLBACK_BUTTONS.iter().zip(&mut self.until) {
            if self.pending & button != 0 {
                // Begin at observation, even if the UI thread handled both edges first.
                *until = Some(now + TAP_TIME);
            }
            if until.is_some_and(|end| now < end) {
                active |= button;
            } else {
                *until = None;
            }
        }
        self.pending = 0;
        active
    }
}

pub(in super::super) fn observe(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) {
    if matches!(message, wm::WM_KILLFOCUS | wm::WM_NCDESTROY)
        || (message == wm::WM_ACTIVATEAPP && wparam == 0)
    {
        let mut state = lock(&STATE);
        if state.window == hwnd as usize || (message == wm::WM_ACTIVATEAPP && wparam == 0) {
            state.clear();
        }
        return;
    }
    let down = match message {
        wm::WM_KEYDOWN => true,
        wm::WM_KEYUP => false,
        _ => return,
    };
    let Some(&(_, button)) = FALLBACK_BUTTONS.iter().find(|(vk, _)| usize::from(*vk) == wparam) else {
        return;
    };
    // SAFETY: queries on the window whose existing procedure we are running.
    let root = unsafe { wm::GetAncestor(hwnd, wm::GA_ROOT) };
    let focused = !root.is_null() && root == unsafe { wm::GetForegroundWindow() };
    let repeat = down && (lparam as usize & (1 << 30)) != 0;
    if *TRACE && button & (A | X) != 0 {
        tracing::info!(button, down, repeat, focused, "title window pad message");
    }
    if focused {
        lock(&STATE).message(hwnd as usize, root as usize, button, down, repeat);
    }
}

pub(super) fn sample(sampled: u32, connected: bool) -> u32 {
    // SAFETY: GetForegroundWindow is a read-only process-independent query.
    let foreground = unsafe { wm::GetForegroundWindow() } as usize;
    let mut state = lock(&STATE);
    let from_window = state.sample(Instant::now(), foreground, connected);
    let output = sampled | from_window;
    if *TRACE {
        let mask = A | X;
        let trace = (connected, sampled & mask, from_window & mask, if connected { output & mask } else { 0 });
        if state.last_trace != Some(trace) {
            state.last_trace = Some(trace);
            tracing::info!(stand_in_connected = connected, sampled_ax = trace.1, window_ax = trace.2,
                stand_in_ax = trace.3, "keyboard stand-in sample");
        }
    }
    output
}

pub(super) fn delivered(reading: &super::Reading, buttons: u32) {
    use std::sync::atomic::{AtomicU32, Ordering};
    static PREVIOUS: AtomicU32 = AtomicU32::new(u32::MAX);
    // SAFETY: a reading made by the hub retains the hub device for its lifetime.
    if *TRACE && unsafe { (*reading.device).index } == super::FALLBACK {
        let ax = buttons & (A | X);
        if PREVIOUS.swap(ax, Ordering::Relaxed) != ax {
            tracing::info!(ax, sequence = reading.sequence, "title read keyboard stand-in pad state");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brief_tap_between_polls_is_visible_to_a_menu() {
        let mut s = Buttons::default();
        let t = Instant::now();
        s.message(1, 1, A, true, false);
        s.message(1, 1, A, false, false);
        assert_eq!(s.sample(t, 1, true), A);
        assert_eq!(s.sample(t + Duration::from_millis(34), 1, true), A);
        assert_eq!(s.sample(t + TAP_TIME, 1, true), 0);
    }

    #[test]
    fn held_button_survives_until_release_without_repeat_pulses() {
        let mut s = Buttons::default();
        let t = Instant::now();
        s.message(1, 1, X, true, false);
        assert_eq!(s.sample(t, 1, true), X);
        s.message(1, 1, X, true, true);
        assert_eq!(s.sample(t + TAP_TIME * 3, 1, true), X);
        s.message(1, 1, X, false, false);
        assert_eq!(s.sample(t + TAP_TIME * 3, 1, true), 0);
    }

    #[test]
    fn focus_loss_or_a_real_pad_discards_held_and_pending_buttons() {
        let t = Instant::now();
        for (foreground, connected) in [(2, true), (1, false)] {
            let mut s = Buttons::default();
            s.message(1, 1, A, true, false);
            assert_eq!(s.sample(t, 1, true), A);
            s.message(1, 1, X, true, false);
            assert_eq!(s.sample(t, foreground, connected), 0);
            assert_eq!(s.sample(t, 1, true), 0);
        }
    }
}
