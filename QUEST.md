# QUEST

## Active Quest: Raise The Node Shell

Goal:
- Turn `main.rs` into a real launcher for the project by adding a basic CLI and startup flow.

What already exists:
- Claim blocks, claim-chain storage, and reload logic.
- A working persistence layer for claim data.
- A campaign roadmap in `CAMPAIGN.md`.

What remains:
- Add CLI argument parsing.
- Add a command or flag for starting a node.
- Add a command or flag for initializing or loading storage.
- Wire the existing claim-chain/storage pieces into startup.
- Make startup failures readable instead of silent.

Hints:
- Keep the first pass simple: one binary, a few flags, and one startup path.
- Do not build networking yet unless the CLI shape forces it.
- Focus on clear ownership of configuration before expanding features.
- Think of this as building the inn at the edge of the hold before the roads.

Status:
- Active.
