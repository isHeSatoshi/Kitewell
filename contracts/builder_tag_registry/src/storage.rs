//! Storage layer.
//!
//! Each builder's tags are one `Vec<String>` in **persistent** storage, keyed
//! by `(TAGS, builder)`. Persistent rather than instance storage because a tag
//! set belongs to a builder and must outlive any single ledger in which the
//! contract happens to be touched; instance storage would tie every builder's
//! tags to one shared TTL.
//!
//! Storing the set per builder — rather than one global map of
//! `(builder, tag) -> ()` — is what makes the per-builder cap cheap to enforce
//! and `list_tags` a single read. The cost is a linear scan inside a builder's
//! own set, which is bounded by [`crate::MAX_TAGS_PER_BUILDER`] and therefore
//! small by construction.

use soroban_sdk::{symbol_short, Address, Env, String, Symbol, Vec};

const TAGS: Symbol = symbol_short!("TAGS");

/// TTL policy for tag sets, in ledgers. Every write extends the entry, so an
/// actively maintained builder never loses their tags to eviction.
const TTL_THRESHOLD: u32 = 1_000;
const TTL_EXTEND_TO: u32 = 5_000;

fn key(builder: &Address) -> (Symbol, Address) {
    (TAGS, builder.clone())
}

/// Read a builder's tags, defaulting to empty for an unknown builder.
pub fn get(env: &Env, builder: &Address) -> Vec<String> {
    env.storage()
        .persistent()
        .get(&key(builder))
        .unwrap_or_else(|| Vec::new(env))
}

/// Write a builder's tags and extend the entry's TTL.
pub fn set(env: &Env, builder: &Address, tags: &Vec<String>) {
    let k = key(builder);
    env.storage().persistent().set(&k, tags);
    env.storage()
        .persistent()
        .extend_ttl(&k, TTL_THRESHOLD, TTL_EXTEND_TO);
}

/// Index of `tag` in the builder's set, or `None`.
pub fn position(env: &Env, builder: &Address, tag: &String) -> Option<u32> {
    let tags = get(env, builder);
    let mut i = 0u32;
    while i < tags.len() {
        if &tags.get_unchecked(i) == tag {
            return Some(i);
        }
        i += 1;
    }
    None
}
