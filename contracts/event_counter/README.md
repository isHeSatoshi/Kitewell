# event_counter

A **generic per-key counter that announces every change**, for Soroban.

Each key gets its own independent `u64`, and every increment publishes an event
carrying the key as a topic and the new value in the data. That makes the
contract usable as a piece of off-chain infrastructure — a consumer follows the
log rather than polling storage — while still holding the count on-chain, so two
readers cannot disagree about it.

## Methods

| Method | Description |
|--------|-------------|
| `increment(caller, key)` | Auth-gated; adds one, returns the new value |
| `increment_by(caller, key, amount)` | Auth-gated; adds `amount`, returns the new value |
| `get(key)` | Permissionless; the current value, or `0` if never counted |
| `reset(caller, key)` | Auth-gated; clears the counter back to zero |

## Behaviour notes

**Every change is attributable.** The mutations call `caller.require_auth()`
before touching storage and publish the caller alongside the value, so the log
says not just that the count went up but who moved it. A permissionless counter
would be cheaper, but a number nobody signed for is worth very little once two
parties disagree about what it means.

**Counters are per key.** Advancing one key never moves another, and `reset`
clears only the key it names. Keys are compared whole, so one key is never a
prefix match for another.

**Keys are symbols.** A key is a `Symbol`, not a string — cheap and finite, like
a metric name or an epoch label, and the cheapest thing Soroban can put in an
event topic, which matters because the topic is what a consumer filters on. Keys
are not limited to nine characters: `Symbol::new` accepts anything up to the
symbol limit, and only the topic pays for the length.

**Overflow is refused, never silent.** `increment` and `increment_by` use
checked addition and return `CounterOverflow` (`1`) rather than wrapping or
saturating, leaving the counter exactly as it was. This matters more here than
in most contracts, because `increment_by` lets a caller jump most of the way to
`u64::MAX` in a single call — so the boundary is reachable in practice, and the
tests exercise it. Any amount that would cross the boundary is refused, not just
one that lands exactly on it.

**A zero increment is allowed.** It changes nothing and still announces, because
the contract has no way to know whether a caller's announcement is meaningful.

**`reset` is strict.** Clearing a key that has never been counted returns
`NoCounter` (`2`), so a reset that did nothing cannot be mistaken for one that
did. A cleared key is then indistinguishable from a never-counted one: the
entry is *removed* rather than written as zero, so storage keeps "never counted"
and "counted back to zero" apart while `get` presents both as `0`.

**No key cap, no enumeration.** The contract does not bound the number of keys
and does not let anyone list them. A counter that could be enumerated would need
an index, and an index would make every increment more expensive than the count
is worth. Consumers are expected to know the keys they care about, which is how
a metrics-style counter is used anyway.

**Storage layout.** One `u64` per key in *persistent* storage under
`(COUNT, key)`. Persistent rather than instance storage because counters are
independent of each other; tying them to one shared instance entry would let a
busy contract evict the counters of keys nobody has touched recently. Every
write extends the entry to 5000 ledgers.

## Errors

| Code | Variant | Raised by |
|------|---------|-----------|
| 1 | `CounterOverflow` | `increment` / `increment_by` when the result would exceed `u64::MAX` |
| 2 | `NoCounter` | `reset` for a key that has never been incremented |

Errors are returned as `Result<_, Error>` rather than panicked, so the variants
stay in the contract spec and the generated `try_*` wrappers surface them as
`Err(Ok(Error::CounterOverflow))`.

## Events

One event per successful change, published only on success. The key is a topic,
so a consumer can filter the log to one counter without decoding the data.

| Event | Topics | Data | Published by |
|-------|--------|------|--------------|
| `increment` | `("increment", key)` | `(caller, value)` | every successful `increment` / `increment_by` |
| `reset` | `("reset", key)` | `caller` | every successful `reset` |

`increment` carries the new value, so a consumer following the log never has to
re-read storage to know where the counter stands. `reset` carries no value
because the new value is always zero.

## Tests

The suite is split by intent:

| File | Covers |
|------|--------|
| `tests/happy_path.rs` | increment → read → reset, and event emission |
| `tests/edge_cases.rs` | key independence, long keys, prefix keys, the value maximum, shared counters |
| `tests/failures.rs` | both `Error` variants, auth failures, and that rejected calls publish nothing |
| `tests/counter_semantics.rs` | the key as a filterable topic, attribution, the log tracking storage, no silent wrap |

```bash
cargo test --manifest-path contracts/event_counter/Cargo.toml
```

Lints:

```bash
cargo clippy --manifest-path contracts/event_counter/Cargo.toml --all-targets -- -D warnings
```

## Standalone crate

This crate is deliberately **not** a member of the `contracts/` workspace. It
declares its own `[workspace]` root in `Cargo.toml`, so it builds and tests
independently and `contracts/Cargo.toml` is left untouched. It is not wired into
`contracts/kitewell` — that integration is a separate, later issue.
