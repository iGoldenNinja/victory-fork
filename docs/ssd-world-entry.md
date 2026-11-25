# RTX 5080 SSD startup and compute pipeline rejection

Windows 11 Pro 26200, Ryzen 9 5900X, 32 GB, NVIDIA RTX 5080 16 GB, driver 617.14. Series X diagnostics, pinned NVIDIA fork base `4e17478b9c27bb530643e5b0e16a767cf4419e08` with the existing read-only package route and diagnostics input replay. DRED was enabled.

The first capture, about 15 seconds after launch, already showed the local player prompt. After controller A, the title rendered illustrated loading artwork, the prologue title card and the opening credits. The startup run used a SATA SSD and an existing local profile. An earlier SATA HDD run captured the player prompt at about 113 seconds with different cache and profile conditions.

Four distinct compute pipelines reported `CreateComputePipelineState` refusal with `0x80070057`. Each logged BLAKE3 prefix matched exactly one compute container in the offline archive. On the same pinned translator, all four emitted DXIL and validated with DXIL validator 1.9.2609.5. The offline translation set `VICTORY_HOST_MAX_WAVE=32`. DXC disassembly of each module reports `WaveSize=64`. This verifies the requested wave size for this four-program sample.

The observed checkpoint was the opening credits. Controllable gameplay and the device-removal failure in VIctory #71 still require separate evidence. No game files, captures or compiled shaders are attached.

[VIctory issue #71](https://github.com/Marvisak/VIctory/issues/71) · [Tester discussion](https://github.com/Marvisak/VIctory/issues/49)

AI-assisted testing and analysis with OpenAI Codex.

## Controlled gameplay on the pinned build

In the prison room, recorded controller input turned the camera both ways and changed the distance to the desk and lamp during forward and backward movement. Substantial character, room and lighting defects were visible. The five-minute run, with four separate marked movement inputs, recorded 737 displayed application frames over 300.016 s. It averaged 2.453 FPS, with 740.854 ms P99 frame time and a worst interval of 981.953 ms. The mean measured GPU busy time was 147.23 ms per displayed interval. The exact-PID trace showed no generated frames. Title-process loopback contained real audio during gameplay. A passing real-time animation and audio-sync test is still pending.

The frame-time result covers the pinned SSD build. Separately, the earlier wave64 evidence covers four opening-sequence pipeline rejections.