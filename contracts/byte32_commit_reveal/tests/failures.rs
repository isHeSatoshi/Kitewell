//! Failure and error cases.
//!
//! Every rejection is asserted against the exact `Error` variant, not just
//! "something failed", so the error codes stay pinned to the contract spec.
//! Auth failures are asserted separately, because those surface as an
//! `InvokeError` rather than as one of our variants.

use byte32_commit_reveal::{Byte32CommitReveal, Byte32CommitRevealClient, Error};
use soroban_sdk::{
    testutils::{Address as _, Events as _},
    vec, Address, BytesN, Env,
};

fn preimage(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

#[test]
fn committing_twice_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let committer = Address::generate(&env);

    client.commit(&committer, &preimage(&env, 1));

    assert_eq!(
        client.try_commit(&committer, &preimage(&env, 2)),
        Err(Ok(Error::CommitmentAlreadyExists))
    );
    // The rejected second commit must not have replaced the first.
    assert_eq!(client.get_commitment(&committer), Some(preimage(&env, 1)));
}

#[test]
fn revealing_without_a_commitment_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let committer = Address::generate(&env);

    assert_eq!(
        client.try_reveal(&committer, &preimage(&env, 3)),
        Err(Ok(Error::NoCommitment))
    );
}

#[test]
fn cancelling_without_a_commitment_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let committer = Address::generate(&env);

    assert_eq!(
        client.try_cancel(&committer),
        Err(Ok(Error::NoCommitment))
    );
}

#[test]
fn revealing_a_wrong_preimage_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let committer = Address::generate(&env);
    let secret = preimage(&env, 4);

    client.commit(&committer, &client.hash(&secret));

    assert_eq!(
        client.try_reveal(&committer, &preimage(&env, 5)),
        Err(Ok(Error::PreimageMismatch))
    );
}

/// A wrong guess must be free to retry: the commitment survives the failed
/// reveal, and the correct preimage still works afterwards. If this were not
/// the case, a single typo would destroy the committer's binding.
#[test]
fn a_wrong_preimage_leaves_the_commitment_intact() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let committer = Address::generate(&env);
    let secret = preimage(&env, 6);
    let commitment = client.hash(&secret);

    client.commit(&committer, &commitment);

    assert!(client.try_reveal(&committer, &preimage(&env, 7)).is_err());
    assert!(client.has_commitment(&committer));
    assert_eq!(client.get_commitment(&committer), Some(commitment));

    // The real preimage still opens it.
    client.reveal(&committer, &secret);
    assert!(!client.has_commitment(&committer));
}

/// Off by one byte is still a mismatch — the comparison is over all 32 bytes.
#[test]
fn a_near_miss_preimage_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let committer = Address::generate(&env);
    let mut bytes = [8u8; 32];
    let secret = BytesN::from_array(&env, &bytes);
    client.commit(&committer, &client.hash(&secret));

    bytes[31] = 9;
    assert_eq!(
        client.try_reveal(&committer, &BytesN::from_array(&env, &bytes)),
        Err(Ok(Error::PreimageMismatch))
    );
}

#[test]
fn committing_on_someone_elses_behalf_is_rejected() {
    let env = Env::default();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);
    let committer = Address::generate(&env);

    // No auth is mocked, so `require_auth` on the committer must fail.
    assert!(client.try_commit(&committer, &preimage(&env, 8)).is_err());
    assert!(!client.has_commitment(&committer));
}

#[test]
fn revealing_without_auth_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let committer = Address::generate(&env);
    let secret = preimage(&env, 9);
    client.commit(&committer, &client.hash(&secret));

    env.set_auths(&[]);
    assert!(client.try_reveal(&committer, &secret).is_err());
    assert!(client.has_commitment(&committer));
}

#[test]
fn cancelling_without_auth_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let committer = Address::generate(&env);
    client.commit(&committer, &preimage(&env, 13));

    env.set_auths(&[]);
    assert!(client.try_cancel(&committer).is_err());
    assert!(client.has_commitment(&committer));
}

/// Nothing that fails may publish an event, so the log stays a faithful record
/// of state changes only.
#[test]
fn rejected_calls_publish_nothing() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Byte32CommitReveal, ());
    let client = Byte32CommitRevealClient::new(&env, &contract_id);

    let committer = Address::generate(&env);
    let secret = preimage(&env, 14);
    client.commit(&committer, &client.hash(&secret));

    // A duplicate commit, a wrong preimage and a bad cancel all fail silently.
    assert!(client.try_commit(&committer, &preimage(&env, 15)).is_err());
    assert_eq!(env.events().all(), vec![&env]);

    assert!(client.try_reveal(&committer, &preimage(&env, 16)).is_err());
    assert_eq!(env.events().all(), vec![&env]);

    let stranger = Address::generate(&env);
    assert!(client.try_cancel(&stranger).is_err());
    assert_eq!(env.events().all(), vec![&env]);
}
