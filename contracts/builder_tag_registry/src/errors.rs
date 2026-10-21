//! Contract errors.
//!
//! A `#[contracterror]` enum returned as `Result<_, Error>`, so the variants
//! stay in the contract spec and the generated `try_*` wrappers surface them as
//! `Err(Ok(Error::TooManyTags))` rather than as an opaque host panic.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// The tag was empty. An empty tag would be present on every builder,
    /// which makes it useless as a label.
    EmptyTag = 1,
    /// The tag is longer than [`crate::MAX_TAG_LEN`] bytes.
    TagTooLong = 2,
    /// The builder already holds [`crate::MAX_TAGS_PER_BUILDER`] tags. The cap
    /// is what keeps a builder's tag list bounded in both size and lookup cost.
    TooManyTags = 3,
    /// The builder already carries this tag. Reported rather than silently
    /// ignored, so a caller cannot believe an add did something it did not.
    TagAlreadyPresent = 4,
    /// The builder does not carry this tag, so there is nothing to remove.
    TagNotPresent = 5,
}
