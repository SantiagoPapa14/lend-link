# Lend-Link
Lend-Link is a decentralized distributed ledger to register and simplify debts between a group of people.
The main idea is that each node represents a user, where both messages between nodes and transactions are signed.
The chain consists of two main subchains, a chain of claims and a chain of debts.

## Accounts
A pair of keys is generated using {TO BE DEFINED} algorithm. 
- The public key, encoded as hex, represents the account address.
- The private key is used to sign messages, claims and debts.

## Chains
Since the idea is for this algorithm to be deployed privately to manage debts between a private group, the blocks
will NOT contain more than one claim or debt, making it more atomic and easy to handle rejections.

There are two independent chains that run in parallel — a **claim chain** and a **debt chain** — each forming its own tree with its own genesis block. The only cross-chain reference is the `claim_hash` field on debt blocks, which links a debt back to the claim it originated from.

### Claims
Claims represent an unverified debt that user A claims to user B, or vice versa.
A claim is created and signed by ONE of the two users. The fields are as follows:
```json
{
    "previous_hash": string,
    "hash": string,
    "issuer": 'borrower' | 'lender',
    "lender": string,
    "borrower": string,
    "amount": int,
    "issued_at": timestamp,
    "issuer_signature": string
}
```

### Debts
Debts represent that a claim has been verified between two users. When a node receives a claim issued to it, it can approve it and create a new debt block.
If it ignores it, it will be abandoned in the chain and never be turned/verified into a debt.

Note that EACH debt has a ONE-TO-ONE relationship with a claim, yet claims might not have a debt.

The fields are as follows:
```json
{
    "previous_hash": string,
    "claim_hash": string,               [claim hash]
    "hash": string,
    "issuer": 'borrower' | 'lender',    [copy claim]
    "lender": string,                   [copy claim]
    "borrower": string,                 [copy claim]
    "amount": int,                      [copy claim]
    "issued_at": timestamp,             [copy claim]
    "lender_signature": string,         [if issuer is lender: copied from claim's `issuer_signature`; verified against the claim hash]
    "borrower_signature": string        [if issuer is borrower: copied from claim's `issuer_signature`; verified against the claim hash]
}
```

The debt block carries two signatures:
- **The claim's original signature** (`issuer_signature`) is copied into the field matching the issuer's role, and is verified against the **claim hash** (via `claim_hash`).
- **The recipient's new signature** signs the **debt block's hash**. If the issuer was the borrower, the lender signs the debt hash for `lender_signature`, and vice versa.

### Consensus
Since both claims and debts require signatures from the users, the validity of each block is ONLY tied to
the signatures being valid. This makes it impossible to create debt between two users if they don't have
the private key of the other account. They can only issue claims that will never be approved. This ensures
both chains are ALWAYS valid.

Blocks form a **tree** — each block references a single parent via `previous_hash`, but multiple blocks
may share the same parent, creating branches. No block is ever discarded.

Each tree independently follows the **longest path from its own genesis** to determine the tip for attaching new blocks. Shorter branches are
preserved in the tree and may become the longest if extended in the future. When two branches are equal
in length, the one with the smallest block hash (lexicographic) is chosen as the tip.

#### Acceptance
Conditions for a block to be valid:
- In case of a claim, the issuer signature must be valid.
- In case of a debt, both signatures must be valid.
- The `previous_hash` must reference a block that exists in the corresponding tree (claim tree for claim blocks, debt tree for debt blocks).

#### Rejection
If the signatures are invalid, the block is ignored and the node is considered byzantine.

#### Orphan Blocks
If the `previous_hash` is unknown, the block is an orphan. The verifier requests the missing chain
from the proposer and appends the received blocks to the local tree. The orphan is resolved once
all its ancestors are known.

#### Branching
When a node receives a block that points to a known parent but is not at the tip of the
longest path, the block is appended as a fork. Both branches are kept. Nodes should attach
new blocks to the tip of the longest known path to minimize forks.

Debt blocks from all branches contribute to the debt graph — once a debt is signed, it is
permanent regardless of which branch it belongs to. The tree structure only affects the order
in which claims are presented to nodes for approval.

### Genesis
Each chain has its own genesis block, created by the first user. Both are broadcast to the peers when the `--genesis` flag is set.

**Claim chain genesis:**
```json
{
    "previous_hash": null,
    "hash": string,
    "issuer": 'genesis',
    "lender": {creator_address},
    "borrower": {creator_address},
    "amount": null,
    "issued_at": timestamp,
    "issuer_signature": {creator_signature}
}
```

**Debt chain genesis:**
```json
{
    "previous_hash": null,
    "hash": string,
    "issuer": 'genesis',
    "lender": {creator_address},
    "borrower": {creator_address},
    "amount": null,
    "issued_at": timestamp,
    "lender_signature": {creator_signature},
    "borrower_signature": {creator_signature}
}
```

When instantiating a node, by default it will ask for a copy of both chains from the peers.

### Peer Discovery
A node connects to **seed nodes** — a set of known addresses composed of long-running servers and random active nodes from previous sessions.

To discover peers:
1. The node asks each seed for the nodes they know.
2. It connects to each newly discovered node and asks them for their peers in turn.
3. This continues until a minimum number of peers is reached.

Once connected, all blocks are broadcast to known peers. There is no central coordinator — the network is a gossip overlay where every node forwards every block it receives.

#### Debt simplification & Indexing
When a node processes the chain, it creates a graph of nodes (accounts) and edges (debts) to represent the debts between the users.
It must compute ALL debts in all branches, since all debts are accepted by both users.

Both the **complete** (unmodified) and **simplified** graphs are kept. Users can consult either view when deciding what to pay, since any claim they submit must still be verified by the recipient.

The simplified graph is computed by:
- Netting debts between the same two users. There is only one amount to be paid between each pair of users, which can be 0/null (no edge).
- Simplifying transitive debts. If user A owes money to user B and B owes money to C, then user A owes money to C.

This system is why we separate between claims and debts, since claims are unverified and debts are verified. Simplifying claims in a transitive way
could allow one issuer to absorb the whole system's debt and ruin the chain.

Lastly, to settle a debt, a new claim is created in the opposite direction. Then when the graph is simplified, the net is zero.

