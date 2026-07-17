# QUEST

## Active Quest: Rebuild The Claim Chain From Storage

Goal:
- Make the claim chain survive a restart by rebuilding in-memory state from `BlockStorage::load()`.

What already exists:
- `BlockStorage` now exposes `load()`.
- `MemoryStorage` and `SqliteStorage` can return saved claim blocks.
- Storage round-trip tests already verify that saved claims can be read back.

What remains:
- Add failing tests that describe how a chain should look after loading.
- Reconstruct `genesis`, `links`, `children`, and orphan tracking from loaded blocks.
- Keep tip selection behavior unchanged after reload.

Hints:
- Start from the data already persisted in storage.
- Rebuild in parent-first order if needed.
- Use tests to define what should happen with genesis, a simple linear chain, and an orphan that later becomes resolvable.
