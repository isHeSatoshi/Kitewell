//! A deduplicated, capped registry of tags per builder address.
//!
//! Attaches a small set of short labels to a builder address — `audit`,
//! `soroban`, `testnet` — with `add_tag`, `remove_tag`, and permissionless
//! reads. The set is deduplicated and capped, so it cannot grow into a
//! free-for-all that costs more to read than it returns.
//!
//! The contract is keyed by the builder address and stores the tags **on the
//! builder**, not the builders under the tag. That is the direction that makes
//! the cap cheap to enforce, because "how many tags does this builder hold" is
//! the length of one entry rather than a count across a global index.
//!
//! ## Auth
//!
//! Both mutations are gated on `builder.require_auth()`. A tag set is a claim
//! about a builder, so only that builder may change it — otherwise anyone could
//! label anyone, and the tags would be worth nothing.
//!
//! ## What this contract does not do
//!
//! It does not check that a builder is registered anywhere. This is a tag
//! store, not a membership check: if a caller wants tags to mean something only
//! for known builders, that gate belongs in the contract that knows the
//! registry. Adding it here would couple this primitive to one registry's
//! notion of who counts.
//!
//! It also does not validate tag *vocabulary*. Length and non-emptiness are
//! enforced because they are structural; whether `"audit"` is a real tag is a
//! policy question for whoever reads them.
#![no_std]

mod errors;
mod events;
mod storage;

pub use errors::Error;
pub use events::{Tagged, Untagged};

use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

/// Maximum length of a tag, in bytes.
pub const MAX_TAG_LEN: u32 = 32;

/// Maximum number of tags one builder may hold.
///
/// The cap is what keeps a builder's tag list bounded: at 8 tags of at most 32
/// bytes, `list_tags` returns at most 256 bytes and `has_tag` scans at most
/// eight entries.
pub const MAX_TAGS_PER_BUILDER: u32 = 8;

/// A deduplicated per-builder tag registry.
#[contract]
pub struct BuilderTagRegistry;

#[contractimpl]
impl BuilderTagRegistry {
    /// Attach `tag` to `builder`. Requires the builder's signature.
    ///
    /// The tag must be non-empty, at most [`MAX_TAG_LEN`] bytes, and not
    /// already present — a duplicate is reported as `TagAlreadyPresent` rather
    /// than silently ignored, so a caller is never left believing an add did
    /// something it did not. Fails with `TooManyTags` once the builder holds
    /// [`MAX_TAGS_PER_BUILDER`] tags.
    ///
    /// Publishes `tagged` with the tag and the resulting count.
    pub fn add_tag(env: Env, builder: Address, tag: String) -> Result<(), Error> {
        builder.require_auth();

        if tag.is_empty() {
            return Err(Error::EmptyTag);
        }
        if tag.len() > MAX_TAG_LEN {
            return Err(Error::TagTooLong);
        }
        if storage::position(&env, &builder, &tag).is_some() {
            return Err(Error::TagAlreadyPresent);
        }

        let mut tags = storage::get(&env, &builder);
        if tags.len() >= MAX_TAGS_PER_BUILDER {
            return Err(Error::TooManyTags);
        }
        tags.push_back(tag.clone());
        storage::set(&env, &builder, &tags);

        Tagged {
            builder,
            tag,
            count: tags.len(),
        }
        .publish(&env);

        Ok(())
    }

    /// Detach `tag` from `builder`. Requires the builder's signature.
    ///
    /// Fails with `TagNotPresent` when the builder does not carry the tag, so a
    /// removal that did nothing cannot be mistaken for one that did. Removing a
    /// tag frees exactly one slot against the cap.
    ///
    /// Publishes `untagged` with the tag and the resulting count.
    pub fn remove_tag(env: Env, builder: Address, tag: String) -> Result<(), Error> {
        builder.require_auth();

        let index = storage::position(&env, &builder, &tag).ok_or(Error::TagNotPresent)?;

        let old = storage::get(&env, &builder);
        let mut tags = Vec::new(&env);
        let mut i = 0u32;
        while i < old.len() {
            if i != index {
                tags.push_back(old.get_unchecked(i));
            }
            i += 1;
        }
        storage::set(&env, &builder, &tags);

        Untagged {
            builder,
            tag,
            count: tags.len(),
        }
        .publish(&env);

        Ok(())
    }

    /// Every tag on `builder`, in the order they were added.
    ///
    /// Permissionless: tags are public labels, and a reader that needed a
    /// signature to see them could not use them.
    pub fn list_tags(env: Env, builder: Address) -> Vec<String> {
        storage::get(&env, &builder)
    }

    /// How many tags `builder` holds.
    pub fn count_tags(env: Env, builder: Address) -> u32 {
        storage::get(&env, &builder).len()
    }

    /// Whether `builder` carries `tag`.
    pub fn has_tag(env: Env, builder: Address, tag: String) -> bool {
        storage::position(&env, &builder, &tag).is_some()
    }
}
