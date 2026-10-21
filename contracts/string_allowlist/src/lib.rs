//! An admin-managed allowlist of short strings.
//!
//! A small, bounded set of names that an administrator maintains — lab names,
//! permitted asset codes, approved labels — with `add`, `remove` and a
//! permissionless `contains` check that any other contract or client can call.
//!
//! The list is deliberately bounded on both axes: entries have a maximum
//! length and the list has a maximum size. A set with no bound is a set whose
//! lookup cost and storage footprint grow without limit, and neither is
//! acceptable for a primitive that other contracts are meant to depend on.
//!
//! ## What this contract does not do
//!
//! There is **one** admin, set once by `init`, and no way to change it. A
//! transferable admin is a separate concern with its own design questions
//! (two-step handover, key rotation, revocation), and folding a half-answer
//! into this primitive would be worse than leaving it out.
//!
//! It also does not normalise entries. `"Lab"` and `"lab"` are two different
//! entries, because deciding whether that is true is a policy question — and
//! a contract that silently lowercased an asset code would be a bug, not a
//! convenience.
#![no_std]

mod errors;
mod events;
mod storage;

pub use errors::Error;
pub use events::{Added, Removed};

use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

/// Maximum length of an allowlist entry, in bytes.
pub const MAX_ENTRY_LEN: u32 = 64;

/// Maximum number of entries the list will hold.
///
/// The bound is what keeps the linear scan in `contains` cheap: at 100 entries
/// of at most 64 bytes, a lookup touches at most 6.4 KB.
pub const MAX_ENTRIES: u32 = 100;

/// An admin-managed allowlist of short strings.
#[contract]
pub struct StringAllowlist;

#[contractimpl]
impl StringAllowlist {
    /// One-time initialiser that sets the admin address.
    ///
    /// `admin` must sign the call. Without that, `init` would be first-caller
    /// wins: anyone watching the deploy could call `init` first and take
    /// ownership of the list before the intended admin got there. Requiring the
    /// signature makes the initialiser safe to run in a separate transaction
    /// from the deploy, because only the address that is about to become admin
    /// can run it.
    ///
    /// Fails with `AlreadyInit` if it has already run; the admin cannot be
    /// changed afterwards.
    pub fn init(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();

        if storage::has_admin(&env) {
            return Err(Error::AlreadyInit);
        }
        storage::set_admin(&env, &admin);
        Ok(())
    }

    /// Read the current admin, if `init` has run.
    pub fn get_admin(env: Env) -> Option<Address> {
        storage::get_admin(&env)
    }

    /// Add `entry` to the allowlist. Admin only.
    ///
    /// The entry must be non-empty and at most [`MAX_ENTRY_LEN`] bytes, and
    /// must not already be present — a duplicate is reported as
    /// `AlreadyAllowed` rather than silently ignored, so a caller is never left
    /// believing an add did something it did not.
    ///
    /// Publishes `added` with the entry and the resulting count.
    pub fn add(env: Env, admin: Address, entry: String) -> Result<(), Error> {
        admin.require_auth();
        storage::require_admin(&env, &admin)?;

        if entry.is_empty() {
            return Err(Error::EmptyEntry);
        }
        if entry.len() > MAX_ENTRY_LEN {
            return Err(Error::EntryTooLong);
        }
        if storage::position(&env, &entry).is_some() {
            return Err(Error::AlreadyAllowed);
        }

        let mut list = storage::get(&env);
        if list.len() >= MAX_ENTRIES {
            return Err(Error::AllowlistFull);
        }
        list.push_back(entry.clone());
        storage::set(&env, &list);

        Added {
            entry,
            count: list.len(),
        }
        .publish(&env);

        Ok(())
    }

    /// Remove `entry` from the allowlist. Admin only.
    ///
    /// Fails with `NotAllowed` when the entry is not present, so a removal that
    /// did nothing cannot be mistaken for one that did. Publishes `removed`
    /// with the entry and the resulting count.
    pub fn remove(env: Env, admin: Address, entry: String) -> Result<(), Error> {
        admin.require_auth();
        storage::require_admin(&env, &admin)?;

        let index = storage::position(&env, &entry).ok_or(Error::NotAllowed)?;

        let old = storage::get(&env);
        let mut list = Vec::new(&env);
        let mut i = 0u32;
        while i < old.len() {
            if i != index {
                list.push_back(old.get_unchecked(i));
            }
            i += 1;
        }
        storage::set(&env, &list);

        Removed {
            entry,
            count: list.len(),
        }
        .publish(&env);

        Ok(())
    }

    /// Whether `entry` is currently allowed.
    ///
    /// Permissionless and free of auth: an allowlist that could not be read
    /// without a signature would be useless to the callers that need it most.
    pub fn contains(env: Env, entry: String) -> bool {
        storage::position(&env, &entry).is_some()
    }

    /// Number of entries currently on the list.
    pub fn count(env: Env) -> u32 {
        storage::get(&env).len()
    }

    /// Every entry currently on the list, in insertion order.
    ///
    /// Insertion order is preserved across removals: removing an entry closes
    /// the gap rather than swapping the last element into it, so the list a
    /// client reads back is always in the order the admin built it.
    pub fn list(env: Env) -> Vec<String> {
        storage::get(&env)
    }
}
