# Original lighting resolve in the prison on RTX 5080

The reflective-caustics temporal resolve produced a mixture of zero, very dim and nonzero values in the prison cell. The original texture had measurable RGB values before any ambient-light substitution.

The diagnostic targeted compute program `34fb51b8a0312e82` after a direct dispatch, using the texture UAV at root table 1, dword 176 identified by [PR 917](https://github.com/fxceless/victory-fork/pull/917). It sampled eight evenly spaced rows of the original 2560 × 1440 R32_UINT texture on the first matching dispatch and every 64 thereafter. It restores the original resource state and reads the result when the host list completes. The run used the early-depth, split-dispatch and paired-wave candidate with the ray-tracing builder disabled. The program translated, created a host pipeline, bound and dispatched. The source revision was `92a72df7b310a4e816ce07a155be1420a6537672`, plus local archive and replay support.

The machine was an RTX 5080 Astral 16 GB with driver 617.14, Ryzen 9 5900X, 32 GB RAM and Windows 11 Pro 25H2 build 26200.9974. The Series X run reached the prison-window movement tutorial. We matched the camera against the previous prison run at a 2542 × 1396 client size, then held it there for 48 seconds. Five samples completed over the full launch. Two completed within the marked window.

| Recorded frame | Sampled texels | Zero RGB | Nonzero RGB |
| --- | ---: | ---: | ---: |
| 2,643 | 20,480 | 12,686 | 7,794 |
| 2,707 | 20,480 | 14,094 | 6,386 |

Using PR 917's RGB9E5 interpretation, 14,180 of 40,960 sampled observations had nonzero RGB (34.62%). Of those, 8,432 had a shared exponent from zero through three. With nine-bit channels, even the largest component at exponent three is below 0.000244 in decoded linear units. These are repeated samples of eight rows, with no per-pixel correspondence to the final displayed walls. The later image still showed the dark room and lit window. [Microsoft's RGB9E5 decoder](https://github.com/microsoft/DirectXMath/blob/main/Inc/DirectXPackedVector.inl)

Four focused tests passed, including a Windows WARP fixture that copies from a synthetic R32_UINT texture, checks the packed values in eight sampled rows and verifies that all original texels survive. The live run ended deliberately without a logged device removal.

This result rules out a uniformly zero output for this dispatch at those two frames. Other probe programs, the correctness of these values, upstream ray data, material inputs and downstream lighting remain open. The run applied no ambient override. A matched measurement of the original output with working ray inputs would help distinguish where the darkness enters.
