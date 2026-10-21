//! Storage layer.
//!
//! One `u64` per key, in **persistent** storage under `(COUNT, key)`. Persistent
//! rather than instance storage because counters are independent of each other:
//! tying them to one shared instance entry would let a busy contract evict the
//! counters of keys nobody has touched recently.
//!
//! A key that has never been incremented is absent rather than zero, so the
//! contract can tell "never counted" from "counted back to zero". Reads present
//! both as `0`, which is what a caller wants; only `reset` cares about the
//! difference, and it reports `NoCounter`.

use soroban_sdk::{symbol_short, Env, Symbol};

const COUNT: Symbol = symbol_short!("COUNT");

/// TTL policy for counters, in ledgers. Every write extends the entry, so an
/// actively incremented counter never expires.
const TTL_THRESHOLD: u32 = 1_000;
const TTL_EXTEND_TO: u32 = 5_000;

fn storage_key(key: &Symbol) -> (Symbol, Symbol) {
    (COUNT, key.clone())
}

/// Read a counter, or `None` if the key has never been incremented.
pub fn get(env: &Env, key: &Symbol) -> Option<u64> {
    env.storage().persistent().get(&storage_key(key))
}

/// Write a counter and extend its TTL.
pub fn set(env: &Env, key: &Symbol, value: u64) {
    let k = storage_key(key);
    env.storage().persistent().set(&k, &value);
    env.storage()
        .persistent()
        .extend_ttl(&k, TTL_THRESHOLD, TTL_EXTEND_TO);
}

/// Drop a counter entirely.
pub fn remove(env: &Env, key: &Symbol) {
    env.storage().persistent().remove(&storage_key(key));
}
