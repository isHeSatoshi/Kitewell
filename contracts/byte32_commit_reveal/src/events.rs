//! Contract events.
//!
//! One event per state transition, published only on success, so an indexer can
//! follow the whole lifecycle of a commitment from the log alone. A call that
//! returns an error publishes nothing.

use soroban_sdk::{contractevent, Address, BytesN};

/// Emitted by every successful `commit`.
///
/// Topics: `("committed", committer)`, data: the commitment. A committer can
/// only hold one commitment at a time, so a sequence of these events for one
/// address is always commit → (reveal | cancel) → commit …
#[contractevent(topics = ["committed"], data_format = "single-value")]
pub struct Committed {
    #[topic]
    pub committer: Address,
    pub commitment: BytesN<32>,
}

/// Emitted by every successful `reveal`.
///
/// Topics: `("revealed", committer)`, data: the commitment that was satisfied.
/// The commitment itself is cleared before this is published, so the event is
/// the only remaining record of which commitment the preimage opened.
#[contractevent(topics = ["revealed"], data_format = "single-value")]
pub struct Revealed {
    #[topic]
    pub committer: Address,
    pub commitment: BytesN<32>,
}

/// Emitted by every successful `cancel`.
///
/// Topics: `("cancelled", committer)`, data: the commitment that was withdrawn.
#[contractevent(topics = ["cancelled"], data_format = "single-value")]
pub struct Cancelled {
    #[topic]
    pub committer: Address,
    pub commitment: BytesN<32>,
}
