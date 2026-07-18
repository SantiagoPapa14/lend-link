# QUEST

## Completed Quest: Rebuild The Claim Chain From Storage

Goal:
- Make the claim chain survive a restart by rebuilding in-memory state from `BlockStorage::load()`.

What already exists:
- `BlockStorage` now exposes `load()`.
- `MemoryStorage` and `SqliteStorage` can return saved claim blocks.
- Storage round-trip tests already verify that saved claims can be read back.

What remains:
- First mission: make `SqliteStorage` accept an explicit database path.
- Then add restart tests against a temp SQLite file so reload behavior can be verified cleanly.
- Reconstruct `genesis`, `links`, `children`, and orphan tracking from loaded blocks.
- Keep tip selection behavior unchanged after reload.
- Current reload tests pass for genesis and a linear chain, but fail when an orphan is loaded before its parent.

Hints:
- Start from the data already persisted in storage.
- Rebuild in parent-first order if needed.
- Use `MemoryStorage` for ordinary tests.
- Use a temp SQLite file for restart tests.

Status:
- Completed. `SqliteStorage::init_with_path` exists, storage load tests are present, and `ClaimChain::index_from_storage()` rebuilds genesis, links, children, and orphan queues in the reload tests.
