# Tracking the controller A press on the RTX 5080

This branch adds tracing at three points in the Windows keyboard-to-controller path. With `VICTORY_WINDOW_PAD_TRACE=1`, the host records title-window A/X key messages, changes to the emulated controller's A/X bits, and the state returned by `GetGamepadState`. A is bit 4. X is bit 16. With a focused title window, a brief mapped button tap is retained for 50 ms. Focus loss or a physical controller clears pending taps. The 10 focused GameInput tests passed on Windows x64.

On a Ryzen 9 5900X and RTX 5080 (driver 617.14), the Series X diagnostics build reached the local PLAYER screen on a SATA hard drive. We started it with the fork's existing `VICTORY_KEYS=245:Enter` test setting. The first captured player prompt was 112.6 seconds after recorder attachment. The first input poll was within a second of launch. At the scheduled time the emulated pad reported A down. The title read A about 17 ms later and read its release 199 ms after the down reading. The first captured visual change was a row of colored stars within one second of the game reading A. Animated illustrated frames followed, then the Navigation Experience minimap setting appeared.

The successful press came from the built-in 200 ms diagnostics replay. Physical keyboard delivery was inconclusive because the window-control tool could not reliably activate the game. The tap-retention logic is covered by its own tests. This run used the same read-only archive router as the previous 60-second idle-player-screen measurement. Cache conditions differed between launches.

[Previous storage measurement](read-only-archive-source.md) · [Upstream RTX 5080 report](https://github.com/Marvisak/VIctory/issues/49#issuecomment-7148741489) · [Independent RTX 3090 report and Settings route](https://github.com/Marvisak/VIctory/issues/49#issuecomment-7118117128)

AI-assisted implementation and testing with OpenAI Codex (GPT-6).
