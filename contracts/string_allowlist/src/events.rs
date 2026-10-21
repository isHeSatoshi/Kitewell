//! Contract events.
//!
//! One event per state transition, published only on success, so an indexer can
//! mirror the allowlist from the log alone. A call that returns an error
//! publishes nothing.

use soroban_sdk::{contractevent, String};

/// Emitted by every successful `add`.
///
/// Topics: `("added",)`, data: the entry. Carries the resulting count as well,
/// so an indexer can follow the size of the list without replaying every event.
#[contractevent(topics = ["added"], data_format = "vec")]
pub struct Added {
    pub entry: String,
    pub count: u32,
}

/// Emitted by every successful `remove`.
///
/// Topics: `("removed",)`, data: the entry and the resulting count.
#[contractevent(topics = ["removed"], data_format = "vec")]
pub struct Removed {
    pub entry: String,
    pub count: u32,
}
