//! Failure and error cases.
//!
//! Every rejection is asserted against the exact `Error` variant rather than
//! "something failed", so the codes stay pinned to the contract spec. Auth
//! failures are asserted separately, because those surface as an `InvokeError`
//! rather than as one of our variants.

use event_counter::{Error, EventCounter, EventCounterClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _},
    vec, Address, Env,
};

fn setup(env: &Env) -> (EventCounterClient<'_>, Address) {
    env.mock_all_auths();
    let contract_id = env.register(EventCounter, ());
    let client = EventCounterClient::new(env, &contract_id);
    (client, Address::generate(env))
}

#[test]
fn incrementing_without_a_signature_is_rejected() {
    let env = Env::default();
    let contract_id = env.register(EventCounter, ());
    let client = EventCounterClient::new(&env, &contract_id);
    let caller = Address::generate(&env);

    // No auth is mocked, so `require_auth` on the caller must fail.
    assert!(client
        .try_increment(&caller, &symbol_short!("epoch"))
        .is_err());
    assert_eq!(client.get(&symbol_short!("epoch")), 0);
}

#[test]
fn incrementing_by_without_a_signature_is_rejected() {
    let env = Env::default();
    let contract_id = env.register(EventCounter, ());
    let client = EventCounterClient::new(&env, &contract_id);
    let caller = Address::generate(&env);

    assert!(client
        .try_increment_by(&caller, &symbol_short!("epoch"), &5)
        .is_err());
    assert_eq!(client.get(&symbol_short!("epoch")), 0);
}

#[test]
fn resetting_without_a_signature_is_rejected() {
    let env = Env::default();
    let (client, caller) = setup(&env);
    let key = symbol_short!("epoch");
    client.increment_by(&caller, &key, &3);

    env.set_auths(&[]);
    assert!(client.try_reset(&caller, &key).is_err());
    assert_eq!(client.get(&key), 3);
}

/// One past the maximum is refused rather than wrapping to zero.
#[test]
fn overflowing_the_counter_is_rejected() {
    let env = Env::default();
    let (client, caller) = setup(&env);
    let key = symbol_short!("epoch");

    client.increment_by(&caller, &key, &u64::MAX);

    assert_eq!(
        client.try_increment(&caller, &key),
        Err(Ok(Error::CounterOverflow))
    );
}

/// An overflow must leave the counter exactly as it was. A saturated or wrapped
/// value would silently corrupt a number other contracts may be reading.
#[test]
fn an_overflow_leaves_the_counter_unchanged() {
    let env = Env::default();
    let (client, caller) = setup(&env);
    let key = symbol_short!("epoch");

    client.increment_by(&caller, &key, &u64::MAX);
    assert!(client.try_increment(&caller, &key).is_err());
    assert!(client.try_increment_by(&caller, &key, &u64::MAX).is_err());

    assert_eq!(client.get(&key), u64::MAX);
}

/// Any amount that would cross the boundary is refused, not just one that lands
/// exactly on it.
#[test]
fn an_increment_that_would_cross_the_boundary_is_rejected() {
    let env = Env::default();
    let (client, caller) = setup(&env);
    let key = symbol_short!("epoch");

    client.increment_by(&caller, &key, &(u64::MAX - 1));

    assert_eq!(
        client.try_increment_by(&caller, &key, &2),
        Err(Ok(Error::CounterOverflow))
    );
    // Exactly reaching the maximum is still fine.
    assert_eq!(client.increment_by(&caller, &key, &1), u64::MAX);
}

#[test]
fn resetting_a_key_that_was_never_counted_is_rejected() {
    let env = Env::default();
    let (client, caller) = setup(&env);

    assert_eq!(
        client.try_reset(&caller, &symbol_short!("epoch")),
        Err(Ok(Error::NoCounter))
    );
}

/// A reset is only refused once: a key that has been counted and then cleared
/// is back to never-counted, so a second reset has nothing to clear either.
#[test]
fn resetting_twice_is_rejected_the_second_time() {
    let env = Env::default();
    let (client, caller) = setup(&env);
    let key = symbol_short!("epoch");

    client.increment(&caller, &key);
    client.reset(&caller, &key);

    assert_eq!(
        client.try_reset(&caller, &key),
        Err(Ok(Error::NoCounter))
    );
}

/// Nothing that fails may publish an event, so the log stays a faithful record
/// of state changes only.
#[test]
fn rejected_calls_publish_nothing() {
    let env = Env::default();
    let (client, caller) = setup(&env);
    let key = symbol_short!("epoch");

    // Reset with nothing to clear.
    assert!(client.try_reset(&caller, &key).is_err());
    assert_eq!(env.events().all(), vec![&env]);

    // Overflow.
    client.increment_by(&caller, &key, &u64::MAX);
    assert!(client.try_increment(&caller, &key).is_err());
    assert_eq!(env.events().all(), vec![&env]);
}
