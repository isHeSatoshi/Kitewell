//! Contract events.
//!
//! One event per state transition, published only on success, so an indexer can
//! mirror every builder's tag set from the log alone. The builder is a topic so
//! a consumer can filter to one builder without decoding the data.

use soroban_sdk::{contractevent, Address, String};

/// Emitted by every successful `add_tag`.
///
/// Topics: `("tagged", builder)`, data: `(tag, count)` — the resulting number
/// of tags the builder holds, so a consumer can follow the set size without
/// replaying every event.
#[contractevent(topics = ["tagged"], data_format = "vec")]
pub struct Tagged {
    #[topic]
    pub builder: Address,
    pub tag: String,
    pub count: u32,
}

/// Emitted by every successful `remove_tag`.
///
/// Topics: `("untagged", builder)`, data: `(tag, count)`.
#[contractevent(topics = ["untagged"], data_format = "vec")]
pub struct Untagged {
    #[topic]
    pub builder: Address,
    pub tag: String,
    pub count: u32,
}
