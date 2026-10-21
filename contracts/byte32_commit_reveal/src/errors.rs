//! Contract errors.
//!
//! Pattern chosen: a `#[contracterror]` enum returned as `Result<_, Error>`.
//! Compared with a stringly `panic!`, this keeps the variants in the contract
//! spec and lets the generated `try_*` wrappers surface them as
//! `Err(Ok(Error::PreimageMismatch))`.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// The committer already has an outstanding commitment. A commitment is
    /// never silently overwritten — the committer must `reveal` or `cancel` it
    /// first, so a second `commit` cannot erase a binding already made.
    CommitmentAlreadyExists = 1,
    /// The committer has no commitment on record, so there is nothing to
    /// reveal or cancel.
    NoCommitment = 2,
    /// The supplied preimage does not hash to the stored commitment. The
    /// commitment is left in place, so a wrong guess does not destroy it.
    PreimageMismatch = 3,
}
