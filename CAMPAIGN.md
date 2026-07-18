# CAMPAIGN

## Vision
Lend-Link should grow from a learning project into a small but complete distributed ledger node:
- a usable CLI
- claim and debt block handling
- local persistence and reload
- peer discovery and gossip
- indexing and graph simplification
- tests that keep the whole system honest

## Current State
- Claim blocks exist and are validated.
- Claim-chain storage and reload work.
- The binary entry point is still empty.
- There is no CLI parsing yet.
- There is no networking layer yet.
- The debt side of the ledger is not implemented yet.
- The next practical milestone is the app shell and CLI.

## Campaign Roadmap

### Phase 1: App Shell
Goal: turn `main.rs` into a real node entry point.
- Add CLI parsing.
- Add subcommands or flags for node startup, genesis creation, and inspection.
- Load configuration from flags and sensible defaults.
- Wire storage initialization into the startup path.
- Add a clean exit path and error reporting.

### Phase 2: Claim Node Workflow
Goal: make the claim chain usable through the CLI.
- Create or load the claim chain on startup.
- Expose commands to add and inspect claims.
- Keep persistence and reload behavior working through the CLI.
- Add integration tests for the command flow.

### Phase 3: Networking Basics
Goal: let nodes talk to each other.
- Add a minimal peer connection model.
- Support peer discovery from seed nodes.
- Support block gossip to known peers.
- Add message validation before accepting remote data.
- Add tests for basic peer exchange and malformed input.

### Phase 4: Debt Block Foundations
Goal: add the debt-side data model.
- Define `DebtBlock`.
- Validate copied claim signatures and recipient signatures.
- Add debt block hashing and tests.
- Keep this separate from network transport until stable.

### Phase 5: Debt Chain And Indexing
Goal: make debts behave like a real second tree.
- Add debt-chain storage and reload.
- Index debts into a graph.
- Preserve forks and orphan handling on the debt side.
- Add tests for branching and restart cases.

### Phase 6: Simplification And Settlement
Goal: turn the ledger into something useful for users.
- Build complete and simplified debt graphs.
- Implement netting between the same pair of users.
- Implement transitive simplification.
- Add settlement flow and tests.

### Phase 7: Hardening
Goal: reduce regressions before calling the project complete.
- Expand edge-case coverage.
- Add end-to-end tests for startup, reload, and gossip.
- Review error handling and validation boundaries.
- Clean up the CLI and documentation.

## Rules Of The Road
- Prefer small, testable milestones.
- Keep each phase shippable before moving on.
- Favor learning value over premature abstraction.
- If networking starts getting large, split it into a dedicated mini-campaign.

## Completion Criteria
The campaign is complete when:
- a node can start from the CLI
- it can load and persist chains
- it can exchange blocks with peers
- both claim and debt chains work
- the chain can be rebuilt after restart
- the tests cover the main failure modes
