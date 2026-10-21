//! Storage layer.
//!
//! Commitments live in *persistent* storage, keyed by `(COMMIT, committer)`.
//! Persistent rather than instance storage because a commitment's whole value
//! is that it survives until it is revealed: if the entry were evicted between
//! the two calls, the committer would lose their binding and, worse, an
//! observer could no longer tell whether a commitment had ever been made.
//!
//! The entry is stored as a plain [`BytesN<32>`] rather than an SDK `Hash<32>`.
//! `Hash` is only meant to be produced by a secure hash function inside the
//! contract, and the SDK notes it should not be used with storage — the bytes
//! coming back out of storage carry no proof of how they were made. Since the
//! commitment arrives from outside as an argument, `BytesN<32>` is the honest
//! type for it.

use soroban_sdk::{symbol_short, Address, BytesN, Env, Symbol};

/// Storage key prefix, so the entry cannot collide with anything else this
/// contract might store later.
const COMMITMENT: Symbol = symbol_short!("COMMIT");

/// TTL policy for commitment entries, in ledgers.
///
/// A commitment is meant to be short-lived — it exists only until the preimage
/// is published — but it must outlive the gap between the two calls. Every
/// write extends the entry to `TTL_EXTEND_TO` ledgers, so a commitment stays
/// live for roughly 5000 ledgers (~7 hours at 5s per ledger) after the most
/// recent write. Reads never extend TTL.
const TTL_THRESHOLD: u32 = 1_000;
const TTL_EXTEND_TO: u32 = 5_000;

fn key(committer: &Address) -> (Symbol, Address) {
    (COMMITMENT, committer.clone())
}

/// Read the outstanding commitment for `committer`, if any.
pub fn get(env: &Env, committer: &Address) -> Option<BytesN<32>> {
    env.storage().persistent().get(&key(committer))
}

/// Whether `committer` currently holds a commitment.
pub fn has(env: &Env, committer: &Address) -> bool {
    env.storage().persistent().has(&key(committer))
}

/// Write a commitment and extend its TTL.
///
/// The caller is responsible for rejecting an overwrite; this function is the
/// raw write.
pub fn set(env: &Env, committer: &Address, commitment: &BytesN<32>) {
    let k = key(committer);
    env.storage().persistent().set(&k, commitment);
    env.storage()
        .persistent()
        .extend_ttl(&k, TTL_THRESHOLD, TTL_EXTEND_TO);
}

/// Drop the commitment for `committer`.
pub fn remove(env: &Env, committer: &Address) {
    env.storage().persistent().remove(&key(committer));
}
