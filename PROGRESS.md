# PROGRESS

## Completed Quests

### Quest 1: Claim Block Foundations
- Defined the `ClaimBlock` structure for claim-only work.
- Implemented claim hashing so the stored hash field does not affect the computed hash.
- Added signing and signature verification for claim issuance.
- Added tests for hash stability, tampering detection, and missing-signature rejection.

### Quest 2: Claim Chain Core
- Introduced `ClaimChain` as the in-memory claim tree.
- Added genesis handling and parent linkage rules.
- Added depth tracking for chain links.
- Added tip selection with longest-path selection and lexicographic tie-breaking.
- Added tests for genesis, valid child insertion, invalid parent handling, and tip resolution.

### Quest 3: Forks And Orphans
- Preserved forks instead of overwriting branches.
- Added orphan tracking indexed by missing parent hash.
- Added recursive orphan resolution when a parent arrives.
- Added tests for linear orphan chains, sibling orphans, and forked orphan resolution.

### Quest 4: Persistence Hook
- Added a minimal SQLite save path for accepted claims.
- Stored validated claim blocks in a local database.
- Wired persistence into claim insertion so accepted claims are recorded.
