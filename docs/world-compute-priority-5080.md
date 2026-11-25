# Native operations and RTX 5080 timings for world compute shaders

The RTX 4060 Ti report in NVIDIA fork issue 10 measured `6d7fe7745ab9c6d8` and `cfd83392ec9ae408` as its two largest indirect compute costs. Its follow-up asks whether these programs do ray traversal or screen-space work. [Original GPU profile](https://github.com/fxceless/victory-fork/issues/10)

Both exact program hashes bound and received host pipelines in the local RTX 5080 prison runs at NVIDIA fork `310c6eb` and the `7e7bb18` diagnostic branch with previously reported blend and descriptor fixes. Device was an RTX 5080 with driver 617.14 and Windows 11 Pro 25H2 build 26200.9974. I matched both hashes and decoded their native instruction streams offline using `victory shader-info --disasm` at `8dd30ab`.

| Program | Guest wave and workgroup | Decoded instruction rows | BVH ray-intersection instructions |
| --- | --- | ---: | ---: |
| `6d7fe7745ab9c6d8` | Wave32, 32 × 1 × 1 | 9,416 | 7 |
| `cfd83392ec9ae408` | Wave32, 8 × 4 × 1 | 250 | 0 |
| `5cf00dc39042edaf` | Wave32, 4 × 8 × 1 | 6,177 | 0 |

The first program uses `image_bvh64_intersect_ray`. The second has image and formatted-buffer operations, and its container metadata identifies a probe gather/copy pass. The third program's metadata identifies checkerboard shadow-mask generation. Its decoded instructions include image sampling, image stores and lane permutations. The instruction counts are static.

## GPU timestamps in the prison

In a separate run of `8dd30ab` on the same RTX 5080, I enabled `VICTORY_GPU_TIMING=1`, which serializes submissions and times individual commands. The run used the existing warmed shader cache. The title was offered a 3840 × 2160 display mode, with no shader-loop-limit override. I verified the prison scene, panned the camera twice, and sampled 12 complete host reporting windows across about 61 seconds. The runtime logged no device removal and I ended the capture deliberately. Resolution scaling still needs a separate measurement.

| Compute program | Prison timestamp results |
| --- | --- |
| `5cf00dc39042edaf` | Highest timed command group in all 12 windows. 12.30–13.91 ms per frame, about one dispatch per frame. Longest dispatch 15.30 ms. |
| `cb6adcc6da2613ff` | Present in all 12 windows. 2.02–2.89 ms per frame. Longest dispatch 5.68 ms. |
| `6d7fe7745ab9c6d8` | Bound and logged 0.36 and 0.73 ms per frame in two opening-sequence windows. No top-40 row in the verified prison windows. |
| `cfd83392ec9ae408` | Bound. No top-40 row in the verified prison windows. |

The runtime prints the 40 highest timed groups per window. A missing row leaves the shader's cost unresolved for that window. The RTX 4060 Ti report used a different revision, scene mix and shader loop limit. [Source for timing and report limits](https://github.com/fxceless/victory-fork/blob/8dd30ab472697c47b141c273c2a00f8ce1c00520/crates/victory-gpu/src/copystat.rs#L1232-L1263)

OpenAI Codex (GPT-6) matched and inspected the programs and drafted this report.
