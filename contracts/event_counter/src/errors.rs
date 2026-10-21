//! Contract errors.
//!
//! A `#[contracterror]` enum returned as `Result<_, Error>`, so the variants
//! stay in the contract spec and the generated `try_*` wrappers surface them as
//! `Err(Ok(Error::CounterOverflow))` rather than as an opaque host panic.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// The increment would take the counter past `u64::MAX`. The counter is
    /// left exactly as it was — a saturated or wrapped value would silently
    /// corrupt a number that other contracts may be reading.
    CounterOverflow = 1,
    /// `reset` was called for a key that has never been incremented, so there
    /// is no counter to clear.
    NoCounter = 2,
}
