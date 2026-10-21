//! Edge cases: flows that are unusual but valid.
//!
//! These are the cases a maintainer is most likely to probe in review — what
//! happens to a commitment that is cancelled, whether two committers can get in
//! each other's way, and whether the contract ever second-guesses the bytes it
//! is handed.

use byte32_commit_reveal::{Byte32CommitReveal, Byte32CommitRevealClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _},
    vec, Address, BytesN, Env, IntoVal, Val, Vec,
};

fn preimage(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

fn cancelled_event(
    env: &Env,
    contract_id: &Address,
    committer: &Address,
    commitment: &BytesN<32>,
) -> (Address, Vec<Val>, Val) {
    (
        contract_id.clone(),
        vec![
            env,
            symbol_short!("cancelled").into_val(env),
            committer.clone().into_val(env),
        ],
        commitment.clone().into_val(env),
    )
}

/// `commit` refuses to overwrite, so `cancel` is the only way out of a
/// commitment. Having taken it, the committer must be free to commit again.
#[test]
fn cancel_then_commit_again_is_allowed() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let committer = Address::generate(&env);
    let first = preimage(&env, 10);
    let second = preimage(&env, 11);

    client.commit(&committer, &first);
    client.cancel(&committer);
    client.commit(&committer, &second);

    assert_eq!(client.get_commitment(&committer), Some(second));
}

#[test]
fn cancel_publishes_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let committer = Address::generate(&env);
    let commitment = preimage(&env, 12);

    client.commit(&committer, &commitment);
    client.cancel(&committer);

    assert_eq!(
        env.events().all(),
        vec![
            &env,
            cancelled_event(&env, &contract_id, &committer, &commitment)
        ]
    );
}

/// Commitments are namespaced by committer, so one address holding a commitment
/// must not affect any other address at all.
#[test]
fn two_committers_are_independent() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let alice_secret = preimage(&env, 20);
    let bob_secret = preimage(&env, 21);
    let alice_commitment = client.hash(&alice_secret);
    let bob_commitment = client.hash(&bob_secret);

    client.commit(&alice, &alice_commitment);
    client.commit(&bob, &bob_commitment);

    // Alice revealing must clear only Alice's entry.
    client.reveal(&alice, &alice_secret);

    assert!(!client.has_commitment(&alice));
    assert!(client.has_commitment(&bob));
    assert_eq!(client.get_commitment(&bob), Some(bob_commitment));
}

/// The contract treats the commitment as opaque bytes: it cannot tell whether
/// they are the digest of anything, because there is nothing to compare against
/// until the reveal. An all-zero commitment is therefore accepted rather than
/// rejected — the check that matters happens in `reveal`.
#[test]
fn commitment_bytes_are_opaque_at_commit_time() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let committer = Address::generate(&env);
    let zeroes = preimage(&env, 0);

    client.commit(&committer, &zeroes);
    assert_eq!(client.get_commitment(&committer), Some(zeroes.clone()));

    // The only preimage that opens it is one that hashes to zeroes, which the
    // contract does not pretend to know. Zeroes are not their own digest.
    assert_ne!(client.hash(&zeroes), zeroes);
}

/// A commitment is a value, not a one-shot token on the address: once it has
/// been consumed, committing the same value again is a fresh binding.
#[test]
fn the_same_commitment_can_be_made_again_after_a_reveal() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let committer = Address::generate(&env);
    let secret = preimage(&env, 30);
    let commitment = client.hash(&secret);

    client.commit(&committer, &commitment);
    client.reveal(&committer, &secret);

    client.commit(&committer, &commitment);
    assert!(client.has_commitment(&committer));

    client.reveal(&committer, &secret);
    assert!(!client.has_commitment(&committer));
}

/// An address that has never committed reads as empty rather than erroring, so
/// callers can check without a try/catch.
#[test]
fn reads_on_an_unknown_committer_are_empty() {
    let env = Env::default();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let stranger = Address::generate(&env);

    assert_eq!(client.get_commitment(&stranger), None);
    assert!(!client.has_commitment(&stranger));
}

/// Two different preimages must not collide on the same commitment, which is
/// what makes the reveal meaningful in the first place.
#[test]
fn distinct_preimages_hash_differently() {
    let env = Env::default();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    assert_ne!(
        client.hash(&preimage(&env, 40)),
        client.hash(&preimage(&env, 41))
    );
}
