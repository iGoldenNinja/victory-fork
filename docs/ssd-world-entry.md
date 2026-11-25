# RTX 5080 SSD startup and compute pipeline rejection

Windows 11 Pro 26200, Ryzen 9 5900X, 32 GB, NVIDIA RTX 5080 16 GB, driver 617.14. Series X diagnostics, pinned NVIDIA fork base `4e17478b9c27bb530643e5b0e16a767cf4419e08` with the existing read-only package route and diagnostics input replay. DRED was enabled.

The first capture, about 15 seconds after launch, already showed the local player prompt. After controller A, the title rendered illustrated loading artwork, the prologue title card and the opening credits. The startup run used a SATA SSD and an existing local profile. An earlier SATA HDD run captured the player prompt at about 113 seconds with different cache and profile conditions.

Four distinct compute pipelines reported `CreateComputePipelineState` refusal with `0x80070057`. Each logged BLAKE3 prefix matched exactly one compute container in the offline archive. On the same pinned translator, all four emitted DXIL and validated with DXIL validator 1.9.2609.5. The offline translation set `VICTORY_HOST_MAX_WAVE=32`. DXC disassembly of each module reports `WaveSize=64`. This verifies the requested wave size for this four-program sample.

The observed checkpoint was the opening credits. Controllable gameplay and the device-removal failure in VIctory #71 still require separate evidence. No game files, captures or compiled shaders are attached.

[VIctory issue #71](https://github.com/Marvisak/VIctory/issues/71) · [Tester discussion](https://github.com/Marvisak/VIctory/issues/49)

AI-assisted testing and analysis with OpenAI Codex.