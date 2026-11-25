# Native operations in two expensive world compute shaders

The RTX 4060 Ti report in NVIDIA fork issue 10 measured `6d7fe7745ab9c6d8` and `cfd83392ec9ae408` as its two largest indirect compute costs. Its follow-up asks whether these programs do ray traversal or screen-space work. [Original GPU profile](https://github.com/fxceless/victory-fork/issues/10)

Both exact program hashes bound and received host pipelines in the local RTX 5080 prison runs at NVIDIA fork `310c6eb` and the `7e7bb18` diagnostic branch with previously reported blend and descriptor fixes. Device was an RTX 5080 with driver 617.14 and Windows 11 Pro 25H2 build 26200.9974. I matched both hashes and decoded their native instruction streams offline using `victory shader-info --disasm` at `8dd30ab`.

| Program | Guest wave and workgroup | Decoded instruction rows | BVH ray-intersection instructions |
| --- | --- | ---: | ---: |
| `6d7fe7745ab9c6d8` | Wave32, 32 × 1 × 1 | 9,416 | 7 |
| `cfd83392ec9ae408` | Wave32, 8 × 4 × 1 | 250 | 0 |

The first program uses `image_bvh64_intersect_ray`. The second has image and formatted buffer operations, and its exact role in the frame remains open. These are static offline counts. Per-pipeline RTX 5080 timings and the dependence on resolution remain to be measured.

OpenAI Codex (GPT-6) matched and inspected the programs and drafted this report.
