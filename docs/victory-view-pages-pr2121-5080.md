# VIctory descriptor paging on the RTX 5080

Tested on NVIDIA GeForce RTX 5080 16 GB, driver 617.14, Ryzen 9 5900X, Windows 11 Pro 25H2 build 26200.9974, Rust 1.98.1 MSVC, DXIL 1.9.2609.5, with the diagnostics profile. `VICTORY_VIEW_PAGES` and `VICTORY_WARP` were unset for all runs.

Clean baseline was VIctory `940d32f69b22265690a0b5a15d6ded572ba33baf`. Clean candidate was PR 2121 head `442632413ce4e8b081b8c4ae4174a51a3682e7f4`.

| Targeted fixture | Baseline | PR 2121 |
| --- | --- | --- |
| Dynamic slot lifetime | Failed the physical heap-size assertion | Passed |
| Page placement on first write and exhaustion | Added by PR | Passed |
| Translated shader read at virtual slot 1,099,777 | Added by PR | Passed on hardware and WARP |
| Buffer read from a non-buffer view returns zero | Outside this comparison | Passed |

The baseline checked that its physical descriptor count was at least `HEAP_VIEWS`. The candidate covers that case through its page table, and the translated shader fixture independently compared a high virtual slot with a direct low slot. The main baseline command was `cargo test --profile diag --locked --offline -p victory-gpu --lib -- dynamic_slots_are_held_until_released --nocapture --test-threads=1`. On the PR head I also ran `pages_are_placed_once_on_first_write` in `victory-gpu --lib`, and `a_heap_index_in_a_paged_view_heap_reads_where_its_page_is` plus `a_heap_buffer_read_where_no_buffer_view_is_reads_zeros` in `victory-shader --lib` with the same profile. The PR passed all four selected fixtures with zero failures or ignored tests.

These checks cover the synthetic descriptor mapping. Title rendering, gameplay timing and page demand on the RTX 5080 remain unverified for this PR. [VIctory PR 2121](https://github.com/Marvisak/VIctory/pull/2121)

OpenAI Codex (GPT-6) ran the tests and drafted this report.
