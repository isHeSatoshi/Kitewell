# byte32_commit_reveal

A generic **32-byte commit-reveal primitive** for Soroban.

Commit-reveal lets someone bind themselves to a value without showing it yet.
The committer publishes `sha256(preimage)` — the *commitment* — and later
publishes the preimage itself. A 32-byte preimage has 2^256 possible values and
SHA-256 is preimage-resistant, so the commitment gives an observer no practical
way to recover the preimage early, while the reveal proves the committer had not
changed their mind in between.

This crate is the primitive only. It does not know what a preimage *means*, and
it is not wired into the Kitewell registry contract — the same code can back a
sealed-bid auction, a commit-then-reveal registration, or a verifiable coin
toss, because nothing here is specific to any of them.

## Methods

| Method | Description |
|--------|-------------|
| `commit(committer, commitment)` | Auth-gated; records a 32-byte commitment for `committer` |
| `reveal(committer, preimage)` | Auth-gated; checks `sha256(preimage)` against the record and clears it |
| `cancel(committer)` | Auth-gated; clears the commitment without revealing |
| `get_commitment(committer)` | Permissionless read; the outstanding commitment, or `None` |
| `has_commitment(committer)` | Permissionless read; whether a reveal is still possible |
| `hash(preimage)` | Permissionless helper; hashes exactly as `reveal` does |

### Lifecycle

```text
commit(committer, sha256(preimage))  -> commitment on record
reveal(committer, preimage)          -> commitment cleared, `revealed` event
cancel(committer)                    -> commitment cleared, `cancelled` event
```

## Behaviour notes

**One commitment per address.** `commit` is keyed by the committer's address and
refuses to overwrite an existing entry, returning `CommitmentAlreadyExists`
(`1`). A second commit cannot erase a binding already made; to change your mind,
`cancel` first, then commit again.

**A commitment is single-use.** A successful `reveal` removes the entry before
the event is published, so the same preimage cannot be replayed against it and
later returns `NoCommitment` (`2`). The same is true of `cancel`: a committer who
withdraws cannot then reveal the preimage they had already chosen. Re-committing
the same value afterwards is a *fresh* binding and works normally — the state
moved, not the input.

**A failed reveal is free.** A wrong preimage returns `PreimageMismatch` (`3`)
and leaves the commitment exactly where it was, so a typo costs nothing and
cannot be used by an observer to reset the game. `reveal` compares all 32 bytes,
so a preimage differing in a single byte is rejected.

**Auth.** `commit`, `reveal` and `cancel` all call `require_auth()` on the
committer address before touching storage. Nobody can commit on someone else's
behalf — and, just as importantly, nobody can be *conscripted* into a commitment
they did not choose. The read methods (`get_commitment`, `has_commitment`,
`hash`) are permissionless: a commitment is public by design, since publishing
it is the whole point of the commit step.

**Commitment bytes are opaque.** At commit time the contract cannot tell whether
the bytes are the digest of anything, because there is nothing to compare them
against until the reveal. It therefore accepts whatever 32 bytes it is given —
including all zeroes — and lets `reveal` be the place where the claim is tested.
`hash(preimage)` is provided so a caller can compute the conventional value
on-chain rather than reimplementing the hashing off-chain and risking a mismatch
in either the algorithm or the input encoding.

**Storage type.** The commitment is stored as `BytesN<32>`, not as an SDK
`Hash<32>`. The SDK is explicit that `Hash` is only meant to be produced by a
secure hash function inside the contract and should not be used with storage,
since bytes coming back out of storage carry no proof of how they were made.
Since the commitment arrives from outside as an argument, `BytesN<32>` is the
honest type for it.

**TTL.** Commitments live in persistent storage under the `COMMIT` key prefix and
every write extends the entry to 5000 ledgers (roughly 7 hours at 5 seconds per
ledger), which is far longer than the gap between a commit and its reveal.
Reads never extend TTL.

**No deadline.** This contract does not enforce a reveal deadline, because a
deadline only means something once there is a consequence for missing it — that
policy belongs to whatever contract uses this primitive. It also does not refund
anything: `cancel` simply drops the commitment.

## Errors

| Code | Variant | Raised by |
|------|---------|-----------|
| 1 | `CommitmentAlreadyExists` | `commit` when the committer already holds one |
| 2 | `NoCommitment` | `reveal` or `cancel` with nothing on record |
| 3 | `PreimageMismatch` | `reveal` when the digest does not match |

Errors are returned as `Result<_, Error>` rather than panicked, so the variants
stay in the contract spec and the generated `try_*` wrappers surface them as
`Err(Ok(Error::PreimageMismatch))`.

## Events

One event per successful state change, so an indexer can follow a commitment's
whole lifecycle from the log alone. A call that returns an error publishes
nothing at all.

| Event | Topics | Data | Published by |
|-------|--------|------|--------------|
| `committed` | `("committed", committer)` | `commitment` | every successful `commit` |
| `revealed` | `("revealed", committer)` | `commitment` | every successful `reveal` |
| `cancelled` | `("cancelled", committer)` | `commitment` | every successful `cancel` |

`revealed` and `cancelled` carry the commitment that was cleared, because the
storage entry is removed in the same call — the event is the only remaining
record of which commitment the preimage opened.

## Tests

The suite is split by intent:

| File | Covers |
|------|--------|
| `tests/happy_path.rs` | commit → reveal, the pinned SHA-256 vector, and event emission |
| `tests/edge_cases.rs` | cancel-then-recommit, two independent committers, opaque bytes, repeat commits |
| `tests/failures.rs` | every `Error` variant, auth failures, and that rejected calls publish nothing |
| `tests/single_use.rs` | consumption and replay resistance — the property the primitive exists for |

```bash
cargo test --manifest-path contracts/byte32_commit_reveal/Cargo.toml
```

Lints:

```bash
cargo clippy --manifest-path contracts/byte32_commit_reveal/Cargo.toml --all-targets -- -D warnings
```

## Standalone crate

This crate is deliberately **not** a member of the `contracts/` workspace. It
declares its own `[workspace]` root in `Cargo.toml`, so it builds and tests
independently and `contracts/Cargo.toml` is left untouched. It is not wired into
`contracts/kitewell` — that integration is a separate, later issue.
