//! Happy path for the 32-byte commit-reveal primitive.
//!
//! The two calls that make the primitive work — `commit`, then `reveal` — plus
//! the event an indexer sees for each.

use byte32_commit_reveal::{Byte32CommitReveal, Byte32CommitRevealClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _},
    vec, Address, BytesN, Env, IntoVal, Val, Vec,
};

/// A preimage whose bytes are all `seed`, so tests can name one cheaply.
fn preimage(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

/// The one event a successful `commit` must publish: topics
/// `("committed", committer)`, data the commitment.
fn committed_event(
    env: &Env,
    contract_id: &Address,
    committer: &Address,
    commitment: &BytesN<32>,
) -> (Address, Vec<Val>, Val) {
    (
        contract_id.clone(),
        vec![
            env,
            symbol_short!("committed").into_val(env),
            committer.clone().into_val(env),
        ],
        commitment.clone().into_val(env),
    )
}

/// The one event a successful `reveal` must publish: topics
/// `("revealed", committer)`, data the commitment that was satisfied.
fn revealed_event(
    env: &Env,
    contract_id: &Address,
    committer: &Address,
    commitment: &BytesN<32>,
) -> (Address, Vec<Val>, Val) {
    (
        contract_id.clone(),
        vec![
            env,
            symbol_short!("revealed").into_val(env),
            committer.clone().into_val(env),
        ],
        commitment.clone().into_val(env),
    )
}

#[test]
fn commit_then_reveal_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let committer = Address::generate(&env);
    let secret = preimage(&env, 7);
    let commitment = client.hash(&secret);

    client.commit(&committer, &commitment);
    assert!(client.has_commitment(&committer));
    assert_eq!(client.get_commitment(&committer), Some(commitment.clone()));

    client.reveal(&committer, &secret);
    assert!(!client.has_commitment(&committer));
    assert_eq!(client.get_commitment(&committer), None);
}

/// The hashing is pinned to a known SHA-256 vector, so a later change to the
/// algorithm — or to how the 32 bytes are fed to it — cannot pass unnoticed.
#[test]
fn hash_is_sha256_of_the_preimage() {
    let env = Env::default();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    // sha256(0x00 * 32)
    let expected = BytesN::from_array(
        &env,
        &[
            0x66, 0x68, 0x7a, 0xad, 0xf8, 0x62, 0xbd, 0x77, 0x6c, 0x8f, 0xc1, 0x8b, 0x8e, 0x9f,
            0x8e, 0x20, 0x08, 0x97, 0x14, 0x85, 0x6e, 0xe2, 0x33, 0xb3, 0x90, 0x2a, 0x59, 0x1d,
            0x0d, 0x5f, 0x29, 0x25,
        ],
    );

    assert_eq!(client.hash(&preimage(&env, 0)), expected);
}

/// The commitment a committer publishes must be exactly what `reveal` checks,
/// so `hash` and `reveal` cannot drift apart.
#[test]
fn hash_output_is_accepted_by_reveal() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let committer = Address::generate(&env);
    let secret = preimage(&env, 255);

    client.commit(&committer, &client.hash(&secret));
    client.reveal(&committer, &secret);

    assert!(!client.has_commitment(&committer));
}

#[test]
fn commit_publishes_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let committer = Address::generate(&env);
    let commitment = preimage(&env, 1);

    client.commit(&committer, &commitment);

    assert_eq!(
        env.events().all(),
        vec![
            &env,
            committed_event(&env, &contract_id, &committer, &commitment)
        ]
    );
}

#[test]
fn reveal_publishes_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let committer = Address::generate(&env);
    let secret = preimage(&env, 2);
    let commitment = client.hash(&secret);

    client.commit(&committer, &commitment);
    client.reveal(&committer, &secret);

    assert_eq!(
        env.events().all(),
        vec![
            &env,
            revealed_event(&env, &contract_id, &committer, &commitment)
        ]
    );
}

/// The read methods are pure: they must not publish anything, so an indexer
/// following this contract never sees an event without a state change behind
/// it.
#[test]
fn reads_publish_nothing() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let committer = Address::generate(&env);
    let commitment = preimage(&env, 3);

    client.commit(&committer, &commitment);

    client.get_commitment(&committer);
    assert_eq!(env.events().all(), vec![&env]);

    client.has_commitment(&committer);
    assert_eq!(env.events().all(), vec![&env]);

    client.hash(&preimage(&env, 4));
    assert_eq!(env.events().all(), vec![&env]);
}
