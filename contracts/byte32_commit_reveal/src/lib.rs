//! A generic 32-byte commit-reveal primitive.
//!
//! Commit-reveal lets someone bind themselves to a value without showing it
//! yet. The committer publishes `sha256(preimage)` — the *commitment* — and
//! later publishes the preimage itself. Because a 32-byte preimage has 2^256
//! possible values and SHA-256 is preimage-resistant, the commitment gives an
//! observer no practical way to recover the preimage early, while the reveal
//! proves the committer had not changed their mind in between.
//!
//! This contract is the primitive only. It does not know what the preimage
//! means and it is not wired into the Kitewell registry: the same code can back
//! a sealed-bid auction, a commit-then-reveal registration, or a fair coin
//! toss, because nothing here is specific to any of them.
//!
//! ## Lifecycle
//!
//! A commitment is keyed by the committer's address and is **single-use**:
//!
//! ```text
//! commit(committer, sha256(preimage))  -> commitment on record
//! reveal(committer, preimage)          -> commitment cleared, event published
//! cancel(committer)                    -> commitment cleared, no reveal
//! ```
//!
//! Three rules follow from that, and each is covered by a test:
//!
//! * A committer holds **at most one** commitment at a time. A second `commit`
//!   is rejected rather than overwriting, so nobody can erase a binding they
//!   already made.
//! * A commitment is **consumed** by a successful reveal. Replaying the same
//!   preimage afterwards fails with `NoCommitment`, so a reveal cannot be used
//!   twice.
//! * A **failed** reveal leaves the commitment in place. Guessing wrong costs
//!   the committer nothing, and does not let an observer reset the game.
//!
//! ## What this contract does not do
//!
//! It does not enforce a reveal deadline, because a deadline only means
//! something once there is a consequence for missing it — that policy belongs
//! to whatever contract uses this primitive. It also does not refund anything:
//! `cancel` simply drops the commitment.
#![no_std]

mod errors;
mod events;
mod storage;

pub use errors::Error;
pub use events::{Cancelled, Committed, Revealed};

use soroban_sdk::{contract, contractimpl, Address, Bytes, BytesN, Env};

/// A generic 32-byte commit-reveal primitive.
#[contract]
pub struct Byte32CommitReveal;

#[contractimpl]
impl Byte32CommitReveal {
    /// Publish a 32-byte commitment for `committer`.
    ///
    /// `committer` must sign the call, so nobody can commit on someone else's
    /// behalf — and, just as importantly, nobody can be *conscripted* into a
    /// commitment they did not choose.
    ///
    /// The commitment is whatever 32 bytes the caller supplies; this contract
    /// does not require it to be `sha256` of anything in particular, because it
    /// cannot verify that until the reveal. [`Self::hash`] is provided so a
    /// caller can compute the conventional value on-chain.
    ///
    /// Fails with `CommitmentAlreadyExists` if `committer` already holds one.
    pub fn commit(env: Env, committer: Address, commitment: BytesN<32>) -> Result<(), Error> {
        committer.require_auth();

        if storage::has(&env, &committer) {
            return Err(Error::CommitmentAlreadyExists);
        }

        storage::set(&env, &committer, &commitment);
        Committed {
            committer,
            commitment,
        }
        .publish(&env);

        Ok(())
    }

    /// Reveal the preimage behind `committer`'s commitment.
    ///
    /// Hashes `preimage` with SHA-256 and compares the digest against the
    /// stored commitment. On a match the commitment is cleared and `Revealed`
    /// is published. On a mismatch nothing changes: the commitment survives, so
    /// a wrong guess is free to retry.
    ///
    /// `committer` must sign the call, and the lookup is keyed by that address,
    /// so only the address that made a commitment can open it.
    ///
    /// Fails with `NoCommitment` if nothing is on record, or
    /// `PreimageMismatch` if the digest does not match.
    pub fn reveal(env: Env, committer: Address, preimage: BytesN<32>) -> Result<(), Error> {
        committer.require_auth();

        let stored = storage::get(&env, &committer).ok_or(Error::NoCommitment)?;
        let digest = env
            .crypto()
            .sha256(&Bytes::from_array(&env, &preimage.to_array()));

        if digest.to_bytes() != stored {
            return Err(Error::PreimageMismatch);
        }

        storage::remove(&env, &committer);
        Revealed {
            committer,
            commitment: stored,
        }
        .publish(&env);

        Ok(())
    }

    /// Withdraw an outstanding commitment without revealing it.
    ///
    /// Useful when the committer no longer wants to proceed, and necessary
    /// because `commit` refuses to overwrite: without this there would be no
    /// way to change your mind. Fails with `NoCommitment` if nothing is on
    /// record.
    pub fn cancel(env: Env, committer: Address) -> Result<(), Error> {
        committer.require_auth();

        let stored = storage::get(&env, &committer).ok_or(Error::NoCommitment)?;

        storage::remove(&env, &committer);
        Cancelled {
            committer,
            commitment: stored,
        }
        .publish(&env);

        Ok(())
    }

    /// Read the outstanding commitment for `committer`, if any.
    ///
    /// Permissionless: a commitment is public by design, since publishing it is
    /// the whole point of the commit step.
    pub fn get_commitment(env: Env, committer: Address) -> Option<BytesN<32>> {
        storage::get(&env, &committer)
    }

    /// Whether `committer` currently holds an outstanding commitment.
    ///
    /// Equivalent to `get_commitment(committer).is_some()`, and cheaper for a
    /// caller that only needs to know whether a reveal is still possible.
    pub fn has_commitment(env: Env, committer: Address) -> bool {
        storage::has(&env, &committer)
    }

    /// Hash a 32-byte preimage exactly as [`Self::reveal`] does.
    ///
    /// A convenience so a committer can obtain the value to pass to `commit`
    /// without reimplementing the hashing off-chain and risking a mismatch in
    /// either the algorithm or the input encoding. Pure: it reads nothing and
    /// publishes nothing.
    pub fn hash(env: Env, preimage: BytesN<32>) -> BytesN<32> {
        env.crypto()
            .sha256(&Bytes::from_array(&env, &preimage.to_array()))
            .to_bytes()
    }
}
