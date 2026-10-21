//! Single-use semantics — the property that makes commit-reveal safe to build on.
//!
//! A commitment that could be revealed more than once would let the same
//! commitment back two separate claims. The tests here pin the rule that a
//! commitment is *consumed*: exactly one reveal (or one cancel) ends its life,
//! and everything afterwards fails with `NoCommitment`.

use byte32_commit_reveal::{Byte32CommitReveal, Byte32CommitRevealClient, Error};
use soroban_sdk::{
    testutils::{Address as _, Events as _},
    vec, Address, BytesN, Env,
};

fn preimage(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

#[test]
fn reveal_consumes_the_commitment() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let committer = Address::generate(&env);
    let secret = preimage(&env, 1);

    client.commit(&committer, &client.hash(&secret));
    client.reveal(&committer, &secret);

    // The very same preimage, against the very same address, is now dead.
    assert_eq!(
        client.try_reveal(&committer, &secret),
        Err(Ok(Error::NoCommitment))
    );
}

/// The replay is rejected because the commitment is gone, not because the
/// preimage changed — committing the same value again makes the same preimage
/// work again, which is what proves the state, not the input, is what moved.
#[test]
fn the_same_preimage_cannot_be_replayed_against_a_consumed_commitment() {
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
        client.try_reveal(&committer, &secret),
        Err(Ok(Error::NoCommitment))
    );

    client.commit(&committer, &commitment);
    client.reveal(&committer, &secret);
    assert_eq!(
        client.try_reveal(&committer, &secret),
        Err(Ok(Error::NoCommitment))
    );
}

/// Cancelling ends the commitment too: a committer who withdraws cannot then
/// turn round and reveal the preimage they had already chosen.
#[test]
fn cancel_consumes_the_commitment() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let committer = Address::generate(&env);
    let secret = preimage(&env, 3);

    client.commit(&committer, &client.hash(&secret));
    client.cancel(&committer);

    assert_eq!(
        client.try_reveal(&committer, &secret),
        Err(Ok(Error::NoCommitment))
    );
    assert_eq!(
        client.try_cancel(&committer),
        Err(Ok(Error::NoCommitment))
    );
}

/// A commitment is bound to the address that made it. Another address cannot
/// open it — and, just as importantly, cannot destroy it by trying: the
/// rightful committer must still be able to reveal afterwards.
#[test]
fn a_commitment_can_only_be_opened_by_its_committer() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let secret = preimage(&env, 4);
    let commitment = client.hash(&secret);

    client.commit(&alice, &commitment);

    // Bob holds no commitment, so his reveal has nothing to match against.
    assert_eq!(
        client.try_reveal(&bob, &secret),
        Err(Ok(Error::NoCommitment))
    );

    // Alice's commitment is untouched and still opens with the same preimage.
    assert_eq!(client.get_commitment(&alice), Some(commitment));
    client.reveal(&alice, &secret);
    assert!(!client.has_commitment(&alice));
}

/// Consuming a commitment is one state change and therefore exactly one event:
/// the log must not double-count a reveal.
#[test]
fn consuming_a_commitment_publishes_exactly_one_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let committer = Address::generate(&env);
    let secret = preimage(&env, 5);

    client.commit(&committer, &client.hash(&secret));
    client.reveal(&committer, &secret);

    assert_eq!(env.events().all().events().len(), 1);

    // And a replay, which changes nothing, publishes nothing.
    assert!(client.try_reveal(&committer, &secret).is_err());
    assert_eq!(env.events().all(), vec![&env]);
}
