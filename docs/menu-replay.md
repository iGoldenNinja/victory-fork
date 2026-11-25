# Replaying a Windows GameInput menu trial

The Windows diagnostics build can read button presses and stick movements from a local text file as a trial runs. This lets the operator see each screen before choosing the next input. The host reports the inputs through its emulated GameInput keyboard and controller. It reads the file on a background thread and starts each hold on its first input poll. Leave `VICTORY_INPUT_REPLAY` unset for an ordinary run.

## Run a trial

Build and test from the repository root in PowerShell.

```powershell
cargo test --locked --profile diag -p victory-hle --features diagnostics --lib 'modules::gameinput::' -- --test-threads=1
cargo build --locked --profile diag -p victory --features diagnostics
```

Create a fresh file on a local drive before starting each trial. In the launch shell, the following commands create an empty file and print the name to use in the second shell. Start the title with your usual arguments and configuration using `target\diag\victory.exe` in the launch shell.

```powershell
$env:VICTORY_INPUT_REPLAY = [System.IO.Path]::GetTempFileName()
$env:VICTORY_INPUT_REPLAY
```

In the second shell, run these commands when the player-selection prompt appears.

```powershell
$replay = Read-Host 'Paste the file name printed by the launch shell'
Add-Content -LiteralPath $replay -Value 'PadA 200' -Encoding Ascii
```

Append one line per input. Each line takes a key and a duration from 25 to 60,000 milliseconds. Wait for the screen to respond before sending the next menu action. A complete newline ends an event. Commands read together can overlap. The reader checks about every 40 ms and reads up to 512 bytes at a time. The existing timed `VICTORY_KEYS` setting can also be used.

| Input | Names |
| --- | --- |
| Face buttons | `PadA`, `PadB`, `PadX`, `PadY` |
| D-pad | `Up`, `Down`, `Left`, `Right` |
| Menu, View, bumpers, triggers, stick clicks | `Menu`, `View`, `LB`, `RB`, `LT`, `RT`, `LS`, `RS` |
| Move, look | `W`/`A`/`S`/`D`, `LookUp`/`LookLeft`/`LookDown`/`LookRight` |

Use `PadA` for the controller's A button. The single letter `A` moves the left stick left.

## Measured menu route

We tested the replay on Windows 11, Ryzen 9 5900X, RTX 5080 16 GB, NVIDIA 617.14, Series X profile, title build 1.0.2.1. This run used the host's read-only archive route to the SATA HDD. The replay source here matches the run. Thirteen focused GameInput tests cover this build, including three new replay fixtures for appended partial lines, held inputs, and both sticks.

1. At the visible player prompt, press `PadA 200`. At Navigation Experience, `Right 160` changed the minimap choice and `Left 160` restored Dynamic.
2. Press `PadA 180` separately on each screen to continue through Navigation Experience, UI size, subtitles, and brightness. At **Finish setup**, `Menu 180` opened the full Settings interface.
3. Press `RB 160` to reach Controller. `Right 140` changed horizontal look sensitivity from 7 to 8. `Left 140` restored 7. Individual `RB 140` presses opened Gameplay, Display, UI, Camera, and Audio.
4. `PadB 180` returned to **Finish setup**. `PadA 180` continued past setup.

Each window below spans 60.034 s. PresentMon identified every displayed frame as an application frame. A marked action is a timestamped replay command within the corresponding measurement window. The windows cover interactive menus.

| Window | Marked actions | Display intervals | Displayed FPS | P99 interval | Longest interval |
| --- | ---: | ---: | ---: | ---: | ---: |
| Setup | 7 | 1,800 | 29.9831 | 35.3831 ms | 82.3172 ms |
| Full Settings | 9 | 1,801 | 29.9997 | 35.3912 ms | 35.6378 ms |

Process-tree loopback PCM contained a click after each minimap change. Both three-second windows peaked at −22.9 dBFS. The simultaneous session meter reported zero.

[Earlier player-selection measurement](player-input-trace.md) · [Upstream measurement thread](https://github.com/Marvisak/VIctory/issues/49#issuecomment-7148741489)

AI-assisted implementation, testing, and write-up with OpenAI Codex (GPT-6).
