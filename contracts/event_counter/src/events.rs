//! Contract events.
//!
//! The counter key is a topic, so a consumer can filter the log to one key
//! without decoding the data — which is the point of a counter that announces
//! itself. The caller is in the data, so an increment is attributable.

use soroban_sdk::{contractevent, Address, Symbol};

/// Emitted by every successful `increment` and `increment_by`.
///
/// Topics: `("increment", key)`, data: `(caller, value)` — the new value after
/// the increment, so a consumer following the log never has to re-read storage
/// to know where the counter stands.
#[contractevent(topics = ["increment"], data_format = "vec")]
pub struct Increment {
    #[topic]
    pub key: Symbol,
    pub caller: Address,
    pub value: u64,
}

/// Emitted by every successful `reset`.
///
/// Topics: `("reset", key)`, data: the caller. No value is carried because the
/// new value is always zero.
#[contractevent(topics = ["reset"], data_format = "single-value")]
pub struct Reset {
    #[topic]
    pub key: Symbol,
    pub caller: Address,
}
