# PR 648 DMA staging on RTX 5080

I tested PR 648's separate DMA revision `4d28486528d239800a79766ee6c2f9f348dca4b5`, based on `310c6eb`, on an RTX 5080 16 GB with driver 617.14, Ryzen 9 5900X, 32 GB RAM and Windows 11 Pro 25H2 build 26200.9974. This revision stages changed pieces in VRAM and downloads them on a D3D12 copy queue. [DMA change](https://github.com/fxceless/victory-fork/commit/4d28486528d239800a79766ee6c2f9f348dca4b5)

Both Series X runs used one diagnostics executable with the same local archive and replay support. I ran the default first, then set `VICTORY_CARD_FETCH_INFLIGHT=1` with its default 64 MiB staging buffer. The two starts used separate byte-matched copies of 3,067 cache seed files and loaded the same host pipeline library. The two starts compiled shader entries afresh, with zero cache hits and 3,054 misses reported during early loading in each run. The gameplay sample was a 90-second camera and movement replay in the first controllable prison room.

| 90-second gameplay measurement | Default | DMA staging |
| --- | ---: | ---: |
| Displayed application frames | 350 | 372 |
| Displayed application FPS | 3.87 | 4.12 |
| P99 displayed frame time | 423.36 ms | 464.50 ms |
| Worst displayed frame time | 1,005.45 ms | 540.95 ms |
| Mean GPU busy per displayed frame | 191.30 ms | 193.40 ms |
| Median reported legacy fetch wait per frame | 139.10 ms | 46.75 ms |
| Median reported copy-back per frame | 186.80 ms | 102.20 ms |
| Median reported draws per frame | 5,362.5 | 5,782.5 |

Over 80 gameplay report windows, the enabled run staged a median 800 pieces (49.66 MiB) per frame and waited 7.0 ms on the copy queue. A median 362.5 pieces per frame were flagged beyond their flight's staging region, with 3.0 landings per frame fetching pieces through the legacy path. The maximum reported averages were 513 pieces and 3.3 legacy landings per frame.

Both runs accepted camera and movement input. The first pan matched within 35 ms relative to player selection, and later actions matched within 14 ms relative to the first pan. The courtyard wire was visible in both. The enabled run had bright gaps in the room shell and long hair streaks in two stationary courtyard views 24 seconds apart. After the retreat, it also showed sky through the ceiling and missing parts of the wall. The default run drew the ceiling and more of the wall in the comparable view. Both runs reported the same eight distinct D3D12 pipeline refusals (`E_INVALIDARG`) with no device removal.

Displayed FPS rose 6.4% in this one fixed-order pair. P99 frame time and visible artifacts were worse with the switch on. The two runs establish the observed difference. A repeat with the order reversed would test whether the image regression follows the switch.
