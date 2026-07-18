# JOURNAL

## 2026-07-18
- Introduced the four-file gameplay system: `CAMPAIGN.md`, `QUEST.md`, `PROGRESS.md`, and `JOURNAL.md`.
- Current code state: claim blocks, storage, and reload logic are in place; the binary entry point is still empty.
- Current focus: phase 1, the node shell and CLI startup path.
- Next likely jump: wire the existing claim/storage logic into a usable command-line entry point before touching networking.
- CLI scaffolding was added with `clap`, plus basic storage selection between memory and SQLite.
- Still missing: proper subcommands, structured error handling, and a cleaner startup flow.
- Test suite now shows the post-refactor fallout from moving `BlockStorage` behind `Box<dyn BlockStorage>`.
- The stale tests are the ones that still assume `ClaimChain<T>` or directly inspect concrete storage internals.
- Patched the stale chain tests to use boxed storage and shared test-only state, then verified `cargo test` passes.
