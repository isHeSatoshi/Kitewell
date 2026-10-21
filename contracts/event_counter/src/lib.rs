//! A generic per-key counter that announces every change.
//!
//! Each key gets its own independent `u64`, and every increment publishes an
//! event carrying the key as a topic and the new value in the data. That makes
//! the contract usable as a piece of off-chain infrastructure — a consumer
//! follows the log rather than polling storage — while still holding the count
//! on-chain so two readers cannot disagree about it.
//!
//! ## Why the caller is in the event
//!
//! The increment is auth-gated and the caller is published alongside the value,
//! so a counter is *attributable*: the log says not just that the count went up,
//! but who moved it. A permissionless counter would be cheaper, but a number
//! nobody signed for is worth very little once two parties disagree about what
//! it means.
//!
//! ## Keys are symbols
//!
//! A key is a short `Symbol`, not a string. Keys are meant to be cheap and
//! finite — a metric name, an epoch label — and a `Symbol` is the cheapest
//! thing Soroban can put in an event topic, which matters because the topic is
//! what a consumer filters on.
//!
//! ## What this contract does not do
//!
//! It does not cap the number of keys, and it does not let anyone enumerate
//! them. A counter contract that could be enumerated would need an index, and an
//! index would make every increment more expensive than the count is worth.
//! Consumers are expected to know the keys they care about, which is how a
//! metrics-style counter is used anyway.
#![no_std]

mod errors;
mod events;
mod storage;

pub use errors::Error;
pub use events::{Increment, Reset};

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

/// A generic per-key counter that emits an event on every change.
#[contract]
pub struct EventCounter;

#[contractimpl]
impl EventCounter {
    /// Add one to the counter for `key`. Requires the caller's signature.
    ///
    /// Returns the new value. Equivalent to `increment_by(caller, key, 1)`.
    ///
    /// Fails with `CounterOverflow` if the counter is already `u64::MAX`.
    /// Publishes `increment` with the key, the caller and the new value.
    pub fn increment(env: Env, caller: Address, key: Symbol) -> Result<u64, Error> {
        Self::increment_by(env, caller, key, 1)
    }

    /// Add `amount` to the counter for `key`. Requires the caller's signature.
    ///
    /// Returns the new value. A key that has never been incremented starts from
    /// zero, so the first call on a fresh key yields exactly `amount`.
    ///
    /// The overflow check is deliberate rather than incidental: a counter that
    /// wrapped or saturated would silently corrupt a number other contracts may
    /// be reading, and `u64::MAX` is reachable in practice precisely because
    /// this function lets a caller jump most of the way there in one call.
    /// On overflow nothing is written — the counter keeps its previous value.
    ///
    /// Publishes `increment` with the key, the caller and the new value.
    pub fn increment_by(
        env: Env,
        caller: Address,
        key: Symbol,
        amount: u64,
    ) -> Result<u64, Error> {
        caller.require_auth();

        let current = storage::get(&env, &key).unwrap_or(0);
        let value = current.checked_add(amount).ok_or(Error::CounterOverflow)?;

        storage::set(&env, &key, value);
        Increment {
            key,
            caller,
            value,
        }
        .publish(&env);

        Ok(value)
    }

    /// Read the counter for `key`, or `0` if it has never been incremented.
    ///
    /// Permissionless: a counter that could not be read without a signature
    /// would be useless to the consumers that need it.
    pub fn get(env: Env, key: Symbol) -> u64 {
        storage::get(&env, &key).unwrap_or(0)
    }

    /// Clear the counter for `key` back to zero. Requires the caller's
    /// signature.
    ///
    /// Fails with `NoCounter` for a key that has never been incremented, so a
    /// reset that did nothing cannot be mistaken for one that did. The next
    /// increment on the key starts from one again.
    ///
    /// Publishes `reset` with the key and the caller.
    pub fn reset(env: Env, caller: Address, key: Symbol) -> Result<(), Error> {
        caller.require_auth();

        // Reject a reset that has nothing to clear, then drop the entry rather
        // than writing a zero, so "never counted" and "counted back to zero"
        // stay distinguishable in storage.
        storage::get(&env, &key).ok_or(Error::NoCounter)?;
        storage::remove(&env, &key);

        Reset { key, caller }.publish(&env);

        Ok(())
    }
}
