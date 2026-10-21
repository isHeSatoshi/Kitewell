# string_allowlist

An **admin-managed allowlist of short strings** for Soroban.

A small, bounded set of names that an administrator maintains — lab names,
permitted asset codes, approved labels — with `add`, `remove` and a
permissionless `contains` check that any other contract or client can call.

The list is deliberately bounded on both axes: entries have a maximum length
and the list has a maximum size. A set with no bound is a set whose lookup cost
and storage footprint grow without limit, and neither is acceptable for a
primitive other contracts are meant to depend on.

## Methods

| Method | Description |
|--------|-------------|
| `init(admin)` | One-time initialiser; stores the admin address |
| `get_admin()` | Returns the current admin, if initialised |
| `add(admin, entry)` | Auth-gated, admin-only; adds `entry` to the list |
| `remove(admin, entry)` | Auth-gated, admin-only; removes `entry` from the list |
| `contains(entry)` | Permissionless; whether `entry` is currently allowed |
| `count()` | Permissionless; number of entries on the list |
| `list()` | Permissionless; every entry, in insertion order |

## Behaviour notes

**One admin, set once, and `init` is signed.** `init(admin)` requires
`admin`'s signature before it stores anything, so the initialiser cannot be
front-run: an onlooker who sees the deploy cannot call `init` first and take the
list. That also makes `init` safe to run in its own transaction after the
deploy. It cannot run twice (`AlreadyInit`, `1`), and there is no way to change
the admin afterwards. A transferable admin is a separate concern with its own
design questions (two-step handover, key rotation, revocation), and folding a
half-answer into this primitive would be worse than leaving it out.

**The admin check collapses two cases.** A caller that is not the stored admin
and a caller on a contract that has never been initialised both get `NotAdmin`
(`2`). That is deliberate: the error cannot be used to probe whether the
contract has been initialised.

**Add is strict.** An entry must be non-empty (`EmptyEntry`, `3`), at most
`MAX_ENTRY_LEN` bytes (`EntryTooLong`, `4`), and not already present
(`AlreadyAllowed`, `5`). A duplicate is *reported* rather than silently ignored,
so a caller is never left believing an add did something it did not. An empty
entry is refused because it would make `contains("")` true for every caller,
which is never what an allowlist means.

**Remove is strict.** Removing an entry that is not present returns
`NotAllowed` (`6`), so a removal that did nothing cannot be mistaken for one
that did.

**Order is preserved.** Removing an entry closes the gap rather than swapping
the last element into it, so `list()` always returns the entries in the order
the admin built them.

**No normalisation.** `"Lab"` and `"lab"` are two different entries, and
whitespace is not trimmed. Deciding whether case matters is a policy question —
and a contract that silently lowercased an asset code would be a bug, not a
convenience — so the contract compares the bytes it is given.

**Reads are open, writes are not.** `contains`, `count`, `list` and `get_admin`
take no auth. An allowlist that could not be read without a signature would be
useless to the callers that need it most.

**Storage layout.** The list is a `Vec<String>` in *instance* storage and the
admin sits beside it. `contains` is therefore a linear scan, which is the right
call for a list that is explicitly small and bounded: at 100 entries of at most
64 bytes, a lookup touches at most 6.4 KB, and `list` and `count` come for free.
A per-entry persistent layout would give O(1) lookup but would need a second
structure to enumerate, which is more moving parts than a bounded list of names
warrants. Every write extends the instance TTL to 5000 ledgers.

## Constants

| Constant | Value | Meaning |
|----------|-------|---------|
| `MAX_ENTRY_LEN` | 64 | Maximum entry length in bytes |
| `MAX_ENTRIES` | 100 | Maximum number of entries; further adds return `AllowlistFull` (`7`) |

## Errors

| Code | Variant | Raised by |
|------|---------|-----------|
| 1 | `AlreadyInit` | `init` when it has already run |
| 2 | `NotAdmin` | `add` / `remove` from a non-admin, or before `init` |
| 3 | `EmptyEntry` | `add` with a zero-length entry |
| 4 | `EntryTooLong` | `add` with an entry over `MAX_ENTRY_LEN` |
| 5 | `AlreadyAllowed` | `add` for an entry already on the list |
| 6 | `NotAllowed` | `remove` for an entry not on the list |
| 7 | `AllowlistFull` | `add` when the list already holds `MAX_ENTRIES` |

Errors are returned as `Result<_, Error>` rather than panicked, so the variants
stay in the contract spec and the generated `try_*` wrappers surface them as
`Err(Ok(Error::NotAllowed))`.

## Events

One event per successful mutation, published only on success, so an indexer can
mirror the list from the log alone. Each carries the resulting count, so a
consumer can follow the size of the list without replaying every event.

| Event | Topics | Data | Published by |
|-------|--------|------|--------------|
| `added` | `("added",)` | `(entry, count)` | every successful `add` |
| `removed` | `("removed",)` | `(entry, count)` | every successful `remove` |

## Tests

The suite is split by intent:

| File | Covers |
|------|--------|
| `tests/happy_path.rs` | init → add → contains → remove, and event emission |
| `tests/edge_cases.rs` | insertion order across removals, the length boundary, case and whitespace sensitivity, re-adding |
| `tests/failures.rs` | every `Error` variant, auth failures, and that rejected calls publish nothing |
| `tests/allowlist_semantics.rs` | membership as the authority, removal leaving no trace, reads open while writes are closed |

```bash
cargo test --manifest-path contracts/string_allowlist/Cargo.toml
```

Lints:

```bash
cargo clippy --manifest-path contracts/string_allowlist/Cargo.toml --all-targets -- -D warnings
```

## Standalone crate

This crate is deliberately **not** a member of the `contracts/` workspace. It
declares its own `[workspace]` root in `Cargo.toml`, so it builds and tests
independently and `contracts/Cargo.toml` is left untouched. It is not wired into
`contracts/kitewell` — that integration is a separate, later issue.
