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

## Five-minute comparison on NVIDIA fork c96b4f2

Same computer, SATA SSD, Series X, and DRED. This second run reused the cache warmed by the first run and followed a similar movement path in the same prison room. The character was more complete, more of the room rendered, and the interaction tutorial was visible. Lighting and geometry defects remained.

PresentMon captured 1,085 displayed application frames over 300.069 s. Average 3.612 FPS, P99 664.398 ms, worst interval 1,017.174 ms, and mean GPU busy time 195.05 ms per displayed interval. During several room samples the parallel copy-back thread reported approximately 180–215 ms per frame. These timing fields overlap. Both five-minute captures missed the 30 FPS gameplay target.

The four earlier wave64 cases also reproduced rejection in an offline host compute PSO creation check on this revision. All four compiled and passed DXIL validation. None created a PSO. Both title runs reached player control with no D3D12 device-removal message in the logs.

## Five-minute comparison on NVIDIA fork `310c6eb`

On the same RTX 5080, driver 617.14, Windows 11 Pro 26200, SATA SSD and Series X profile, `310c6eb6c16822ab95076aad61fce19bba86bb98` reached the prison with working camera and movement through the existing diagnostics replay. DRED was enabled. The run used a warmed cache and five timed forward and backward movements in the same room. Camera angles and draw counts differed from the earlier run.

PresentMon captured 1,264 application frames over 300.105 seconds, averaging **4.209 FPS** with **417.475 ms P99** and **523.282 ms worst**. Mean GPU busy time was 182.73 ms per displayed interval. For comparison, `c96b4f2` averaged 3.612 FPS with 664.398 ms P99. Across 54 runtime timing samples per run, the copy-back thread's median reported time was 175.25 ms per frame on `310c6eb` and 180.65 ms on `c96b4f2`. GPU and copy-back time overlap. The camera path and cache history prevent assigning the change to a specific commit or pass.

Strong lighting and pixel artifacts remained visible. DRED did not report device removal, and the run ended deliberately after measurement. Title-process PCM was present. Audio synchronization, normal game speed and story progression beyond this room remain unmeasured. The opening sequence still refused the four previously sampled compute PSOs. [ZTRX barrier comparison on the same four inputs](https://github.com/iGoldenNinja/victory-fork/blob/f31b723185efcfac8fb9bebf7a6e0463ec581850/docs/ztrx-wave32-barriers-5080.md)
