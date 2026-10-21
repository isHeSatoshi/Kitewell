//! Contract errors.
//!
//! A `#[contracterror]` enum returned as `Result<_, Error>`, so the variants
//! stay in the contract spec and the generated `try_*` wrappers surface them as
//! `Err(Ok(Error::AlreadyAllowed))` rather than as an opaque host panic.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// `init` has already run. The admin is set once and cannot be replaced.
    AlreadyInit = 1,
    /// The caller is not the stored admin, or `init` has not run yet.
    NotAdmin = 2,
    /// The entry was empty. An empty string would make `contains("")` true for
    /// every caller, which is never what an allowlist means.
    EmptyEntry = 3,
    /// The entry is longer than [`crate::MAX_ENTRY_LEN`] bytes.
    EntryTooLong = 4,
    /// The entry is already on the list. Reported rather than silently ignored,
    /// so a caller cannot believe an add did something it did not.
    AlreadyAllowed = 5,
    /// The entry is not on the list, so there is nothing to remove.
    NotAllowed = 6,
    /// The list already holds [`crate::MAX_ENTRIES`] entries. The bound exists
    /// to keep the linear scan in `contains` cheap.
    AllowlistFull = 7,
}
