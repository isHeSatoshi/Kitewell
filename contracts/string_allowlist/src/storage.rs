//! Storage layer.
//!
//! The allowlist is a `Vec<String>` held in **instance** storage. That is a
//! deliberate trade: `contains` is a linear scan, which is the right call for a
//! list that is explicitly small and bounded (see [`crate::MAX_ENTRIES`]),
//! because it keeps the whole list in one entry and makes `list` and `count`
//! free. A per-entry persistent layout would give O(1) `contains` but would
//! need a second structure to enumerate, which is more moving parts than a
//! bounded list of names warrants.
//!
//! The admin address is also instance storage: it is read on every mutating
//! call and is never expected to be large.

use soroban_sdk::{symbol_short, Address, Env, String, Symbol, Vec};

const ADMIN: Symbol = symbol_short!("ADMIN");
const ALLOW: Symbol = symbol_short!("ALLOW");

/// TTL policy for instance storage, in ledgers. Every write extends it, so an
/// actively used allowlist never expires.
const TTL_THRESHOLD: u32 = 1_000;
const TTL_EXTEND_TO: u32 = 5_000;

pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&ADMIN)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&ADMIN, admin);
    env.storage()
        .instance()
        .extend_ttl(TTL_THRESHOLD, TTL_EXTEND_TO);
}

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&ADMIN)
}

/// Read the whole list, defaulting to empty on a fresh contract.
pub fn get(env: &Env) -> Vec<String> {
    env.storage().instance().get(&ALLOW).unwrap_or_else(|| Vec::new(env))
}

pub fn set(env: &Env, list: &Vec<String>) {
    env.storage().instance().set(&ALLOW, list);
    env.storage()
        .instance()
        .extend_ttl(TTL_THRESHOLD, TTL_EXTEND_TO);
}

/// Index of `entry` in the list, or `None`.
pub fn position(env: &Env, entry: &String) -> Option<u32> {
    let list = get(env);
    let mut i = 0u32;
    while i < list.len() {
        if &list.get_unchecked(i) == entry {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Reject a caller that is not the stored admin.
///
/// A missing admin and a wrong admin collapse to the same `NotAdmin`, so a
/// caller cannot use the error to probe whether the contract has been
/// initialised.
pub fn require_admin(env: &Env, caller: &Address) -> Result<(), crate::Error> {
    match get_admin(env) {
        Some(admin) if &admin == caller => Ok(()),
        _ => Err(crate::Error::NotAdmin),
    }
}
