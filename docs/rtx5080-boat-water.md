# RTX 5080 boat water and mission progression

Geckomacabre identified the need to copy water textures written by the GPU back to memory the title reads on the CPU. This trial uses a local port of that work, with an explicit selector for single-plane textures and ownership retained until the GPU finishes the copy. [Geckomacabre's original change](https://github.com/Geckomacabre/vx-layer/commit/eee5667d7119bc4a378214621e1b18a814ebe80f) · [Existing boat issue](https://github.com/fxceless/victory-fork/issues/863)

KaV3R reported flotation and driving on an RTX 4070 SUPER using four texture shapes. Mishkile reported flotation on an RTX 5090 using copies keyed to the water-simulation programs. Their working sets included two common outputs, which this RTX 5080 pair isolated. [KaV3R's Series X retest](https://github.com/fxceless/victory-fork/issues/863#issuecomment-7165728646) · [Mishkile's independent producer trace](https://github.com/fxceless/victory-fork/issues/863#issuecomment-7165747535)

## Test setup

Series X profile, ASUS ROG Astral RTX 5080 16 GB, NVIDIA 617.14, Ryzen 9 5900X, 32 GB RAM, Windows 11 Pro 25H2 build 26200.9974. The diagnostic renderer revision was `e4c18eca18299d8b14d10f48d217f1d548161dd4`. The executable SHA-256 was `b0f232d39948e7ee85980975cf9bfce1bd7ef7d1542201b7f584c4c73610bcb8`. It included local archive and controller-replay support. Each process started with separate writable copies of the same verified story save and warm cache. [Tested renderer change](https://github.com/iGoldenNinja/victory-fork/commit/e4c18eca18299d8b14d10f48d217f1d548161dd4)

Both CPU tests passed. One checks that selection is limited to the configured single-plane textures. The other round-trips test data with pitched rows. Native observations below cover the waterline and the mission as seen on the RTX 5080.

## Native control

With water writeback disabled, water rose above the deck while both characters were aboard (sample 044). The aircraft-follow objective and HUD appeared as the boat sat nearly submerged (051). The game then reported that the mission failed because Cal fell off the boat (056). The only replay event in this process was account selection. The failure occurred before any throttle or steering input.

## Two-plane writeback

The treatment copied only the 64 × 64 `R32G32_FLOAT` (format 16) and 128 × 128 `R16G16_UINT` (format 36) planes back to guest memory. Logs confirmed at least 2,048 and 512 completed full-plane writes respectively. The counters log powers of two. At the corresponding helm shot (050), water sat around the hull and the bank and trees were visible behind the helm. In the aft-camera HUD view the stern, both outboards and occupants stayed above the water through samples 058 and 059. The aircraft-follow objective remained active. The first throttle input came after both observations. Black rectangular patches remained visible on the surface.

After RT, a wake appeared and widened as the boat moved forward (061–062). A 4.2-second starboard input while under throttle changed the bow heading relative to the fixed bridge (063), with the boat still afloat. The HUD then advanced to the package-collection objective and showed a marker on the water ahead (065).

Later, the boat struck the mangroves (071). The player entered the water and the HUD prompted a return to the boat (072).

This control and treatment establish that the two outputs together restored flotation through the observed mission handoff on this 5080. The individual contribution of each output remains untested. The control lost its passenger before the treatment's driving interval, so there is no matched driving FPS comparison.
