# builder_tag_registry

A **deduplicated, capped registry of tags per builder address** for Soroban.

Attaches a small set of short labels to a builder address — `audit`, `soroban`,
`testnet` — with `add_tag`, `remove_tag`, and permissionless reads. The set is
deduplicated and capped, so it cannot grow into a free-for-all that costs more
to read than it returns.

The contract is keyed by the builder address and stores the tags **on the
builder**, not the builders under the tag. That is the direction that makes the
cap cheap to enforce: "how many tags does this builder hold" is the length of
one entry rather than a count across a global index.

## Methods

| Method | Description |
|--------|-------------|
| `add_tag(builder, tag)` | Auth-gated; attaches `tag` to `builder` |
| `remove_tag(builder, tag)` | Auth-gated; detaches `tag` from `builder` |
| `list_tags(builder)` | Permissionless; every tag, in the order added |
| `count_tags(builder)` | Permissionless; how many tags the builder holds |
| `has_tag(builder, tag)` | Permissionless; whether the builder carries `tag` |

## Behaviour notes

**Only the builder may label themselves.** Both mutations call
`builder.require_auth()` before touching storage. A tag is a claim about a
builder, so only that builder may make it — without the signature anyone could
label anyone and the tags would be worth nothing.

**Add is strict.** A tag must be non-empty (`EmptyTag`, `1`), at most
`MAX_TAG_LEN` bytes (`TagTooLong`, `2`), and not already present
(`TagAlreadyPresent`, `4`). A duplicate is *reported* rather than silently
ignored, so a caller is never left believing an add did something it did not.
An empty tag is refused because it would be present on every builder, which
makes it useless as a label.

**Remove is strict.** Removing a tag the builder does not carry returns
`TagNotPresent` (`5`), so a removal that did nothing cannot be mistaken for one
that did. Removing a tag frees exactly one slot against the cap.

**The cap is per builder.** `MAX_TAGS_PER_BUILDER` (8) tags, and a ninth add
returns `TooManyTags` (`3`). The cap is what keeps a builder's set bounded: at 8
tags of at most 32 bytes, `list_tags` returns at most 256 bytes and `has_tag`
scans at most eight entries. Because the set is stored per builder, one address
filling up does not consume anybody else's budget.

**Order is preserved.** Removing a tag closes the gap rather than swapping the
last tag into it, so `list_tags` always returns the tags in the order the
builder added them.

**Duplicates are checked before the cap.** A builder already at the limit who
re-adds a tag they hold gets `TagAlreadyPresent`, not `TooManyTags`. Either way
nothing changes, but the error tells the caller which problem they actually
have.

**No normalisation.** `"Audit"` and `"audit"` are two different tags, because
whether they are the same label is a policy question.

**No vocabulary validation.** Length and non-emptiness are enforced because they
are structural; whether `"audit"` is a *real* tag is a question for whoever
reads them.

**No registration check.** This is a tag store, not a membership check. If a
caller wants tags to mean something only for known builders, that gate belongs
in the contract that knows the registry — adding it here would couple this
primitive to one registry's notion of who counts.

**Storage layout.** Each builder's tags are one `Vec<String>` in *persistent*
storage, keyed by `(TAGS, builder)`. Persistent rather than instance storage
because a tag set belongs to a builder and must outlive any single ledger in
which the contract happens to be touched; instance storage would tie every
builder's tags to one shared TTL. Every write extends the entry to 5000
ledgers. The cost of this layout is a linear scan inside a builder's own set,
which is bounded by the cap and therefore small by construction.

## Constants

| Constant | Value | Meaning |
|----------|-------|---------|
| `MAX_TAG_LEN` | 32 | Maximum tag length in bytes |
| `MAX_TAGS_PER_BUILDER` | 8 | Maximum tags one builder may hold |

## Errors

| Code | Variant | Raised by |
|------|---------|-----------|
| 1 | `EmptyTag` | `add_tag` with a zero-length tag |
| 2 | `TagTooLong` | `add_tag` with a tag over `MAX_TAG_LEN` |
| 3 | `TooManyTags` | `add_tag` when the builder already holds `MAX_TAGS_PER_BUILDER` |
| 4 | `TagAlreadyPresent` | `add_tag` for a tag the builder already carries |
| 5 | `TagNotPresent` | `remove_tag` for a tag the builder does not carry |

Errors are returned as `Result<_, Error>` rather than panicked, so the variants
stay in the contract spec and the generated `try_*` wrappers surface them as
`Err(Ok(Error::TooManyTags))`.

## Events

One event per successful mutation, published only on success, so an indexer can
mirror every builder's tag set from the log alone. The builder is a topic, so a
consumer can filter to one builder without decoding the data.

| Event | Topics | Data | Published by |
|-------|--------|------|--------------|
| `tagged` | `("tagged", builder)` | `(tag, count)` | every successful `add_tag` |
| `untagged` | `("untagged", builder)` | `(tag, count)` | every successful `remove_tag` |

Both names fit the SDK's nine-character short-symbol limit, so the topics cost
no more than a built-in symbol.

## Tests

The suite is split by intent:

| File | Covers |
|------|--------|
| `tests/happy_path.rs` | add → has_tag → remove, and event emission |
| `tests/edge_cases.rs` | order across removals, the length boundary, the exact cap, case sensitivity |
| `tests/failures.rs` | every `Error` variant, auth failures, and that rejected calls publish nothing |
| `tests/tag_set_semantics.rs` | per-builder namespacing, the per-builder cap, removals leaving no trace |

```bash
cargo test --manifest-path contracts/builder_tag_registry/Cargo.toml
```

Lints:

```bash
cargo clippy --manifest-path contracts/builder_tag_registry/Cargo.toml --all-targets -- -D warnings
```

## Standalone crate

This crate is deliberately **not** a member of the `contracts/` workspace. It
declares its own `[workspace]` root in `Cargo.toml`, so it builds and tests
independently and `contracts/Cargo.toml` is left untouched. It is not wired into
`contracts/kitewell` — that integration is a separate, later issue.
