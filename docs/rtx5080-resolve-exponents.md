# Prison lighting resolve exponent histogram on RTX 5080

Of the **5,748** nonzero output texel observations above shared exponent 3 in the original two prison frames, **4,797 had exponent 15 or higher**. The largest occupied exponent was 17 in frame 2643 and 18 in frame 2707. The five later frames independently had 12,795 nonzero observations at exponent 15 or higher. [PR 917 author's question](https://github.com/fxceless/victory-fork/pull/917#issuecomment-7164831266)

| Nonzero texels by exponent | First two frames | Later five frames |
| --- | ---: | ---: |
| 0–3 | 8,432 | 13,864 |
| 4–11 | 643 | 3,742 |
| 12–14 | 308 | 740 |
| 15 | 835 | 2,026 |
| 16 | 3,686 | 9,919 |
| 17 | 272 | 844 |
| 18 | 4 | 6 |
| 19–31 | 0 | 0 |
| **All nonzero** | **14,180** | **31,141** |
| All sampled texels, including black | 40,960 | 102,400 |

For the original 5,748, the individual counts at exponents 4 through 14 were 59, 78, 133, 155, 154, 41, 9, 14, 35, 82 and 191. Each readback sampled eight evenly spaced rows of the 2560 × 1440 original resolve output, or 20,480 texels. In every frame the exact zero-word count equaled the count of texels with three zero mantissas. Subtracting those black words from exponent bin zero therefore recovers the exact nonzero histogram in these runs. The counts refer to repeated samples across frames.

| Diagnostic | Prison frame | Nonzero texels | At exponent ≥15 | Highest exponent |
| --- | ---: | ---: | ---: | ---: |
| `92a72df` | 2643 | 7,794 | 2,570 | 17 |
| `92a72df` | 2707 | 6,386 | 2,227 | 18 |
| `fefc2e2` | 2272 | 7,092 | 2,698 | 18 |
| `fefc2e2` | 2304 | 6,644 | 2,503 | 17 |
| `fefc2e2` | 2336 | 5,907 | 2,262 | 18 |
| `fefc2e2` | 2368 | 5,953 | 2,783 | 18 |
| `fefc2e2` | 2400 | 5,545 | 2,549 | 18 |

Under the RGB9E5 interpretation, each channel equals its 9-bit mantissa times `2^(E−24)`. A channel at exponent 3 is at most 0.000243664, and one at exponent 14 is at most 0.499023438. Only the exponent-15-and-higher observations could reach 0.6. For each frame with highest exponent 17, the maximum observed channel is bounded by 0.0078125–3.9921875. For highest exponent 18 the corresponding bounds are 0.015625–7.984375. The lower bound uses one nonzero mantissa. The recorded histograms do not retain its magnitude or individual pixel positions. They cannot establish how many channels exceeded 0.6, whether bright texels followed scene geometry, or whether a floor would preserve valid lighting. [Shared-exponent format specification](https://registry.khronos.org/OpenGL/extensions/EXT/EXT_texture_shared_exponent.txt)

Both measurements reached the Series X prison window on an RTX 5080 Astral 16 GB, driver 617.14, Ryzen 9 5900X, 32 GB RAM and Windows 11 Pro 25H2 build 26200.9974, with the ray builder and ambient override off. The 65 of 115 entries in the prior input trace were bound input textures with no earlier tracked compute UAV in that sampled frame. The histogram above measures the output at root table 1, dword 176. [Original two-frame measurement](https://github.com/iGoldenNinja/victory-fork/blob/bd68e02b9f74f7a2c7795f552d19c5204679edb0/docs/rtx5080-original-light-resolve.md) · [Five-frame measurement](https://github.com/iGoldenNinja/victory-fork/blob/cfa7e4cde248fe73652948d7b2a2d3344d2e01ac/docs/rtx5080-resolve-input-bindings.md)
