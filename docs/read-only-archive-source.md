# Optional read-only archive source for the Windows host

This experiment lets a private package mount read large, immutable archives directly from a separate local folder. It was built against commit `4e17478b9c27bb530643e5b0e16a767cf4419e08` of the NVIDIA fork. [Base revision](https://github.com/fxceless/victory-fork/commit/4e17478b9c27bb530643e5b0e16a767cf4419e08)

Set `VICTORY_ARCHIVE_READ_ROOT` to an absolute local content directory before starting the host. When the guest has a mounted G drive, the host’s `CreateFileW` wrapper consults this directory for eligible `.rpf` and `.ttpf` reads. This covers guest calls to that wrapper and DirectStorage factory opens. A successful direct open emits the info-level event `direct read-only source archive`.

The guest must request exactly `GENERIC_READ` with `OPEN_EXISTING`, permit read sharing, and supply the default security and template arguments. The source handle permits read sharing only. The wrapper accepts default flags, `FILE_ATTRIBUTE_NORMAL`, sequential or random access, overlapped I/O and unbuffered I/O. Source files must be regular files with safe ASCII path components. V2 accepts a literal current-directory component inside a path. Parent components, empty components, alternate streams, reserved names and reparse points fall back to the normal package path.

Writes, metadata lookups, DLL loads and other file APIs continue to use the normal mount. Use this option for immutable archives. If a private copy is edited while the source archive still exists, a subsequent eligible read will return the source copy.

## Verification

The Windows fixture gives the source and private folders different byte strings. It checks plain and dotted archive reads, private lookup and write behavior, a competing writer's sharing violation, and fallback for a real NTFS junction pointing outside the source. It also checks parent and repeated separator handling. The fixture creates its own files and runs with the option both disabled and enabled.

```powershell
cargo test --release --locked -p victory-hle --lib modules::files:: -- --test-threads=1
```

All seven focused tests passed on Windows x64. An earlier V1 live run recorded 24 successful direct archive opens in one startup sample. A V2 startup sample recorded 92 successful direct opens across 46 distinct dotted-path table archives. These observations verify file routing. A controlled boot-time or frame-rate measurement is pending.

### First player screen on an RTX 5080

On Windows 11 build 26200 with a Ryzen 9 5900X, 32 GB RAM and NVIDIA 617.14, the Series X diagnostics build reached a visible local player prompt after about 189 seconds with the source archives on a SATA hard drive. The host presented to a 3840 × 2160 SDR surface. The title's PID accumulated 3D and copy-engine time on the RTX 5080. An external PID-filtered presentation trace measured 30.05 displayed FPS over 60.034 seconds on that idle screen, with a 35.315 ms 99th percentile and a worst interval of 41.160 ms. Its audio session remained silent over the same interval. Gameplay, responsiveness to controls and game-speed correctness need separate measurements.

The first 24- and 36-second thread samples placed all 24 DirectStorage workers in synchronous file reads. At 65 and 115 seconds the storage workers were waiting for work, while an ordinary reader remained in Windows ReadFile. These observations establish the boot path and account-screen timing on this machine. A same-storage, same-cache control is needed to measure how much time the host change itself saves.

[Upstream test discussion](https://github.com/Marvisak/VIctory/issues/49#issuecomment-7148741489)

AI-assisted implementation and testing with OpenAI Codex (GPT-6).
