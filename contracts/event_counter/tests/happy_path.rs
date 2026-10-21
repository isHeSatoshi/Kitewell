//! Happy path for the per-key event counter.
//!
//! Increment, read back, reset — plus the events a consumer sees.

use event_counter::{EventCounter, EventCounterClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _},
    vec, Address, Env, IntoVal, Symbol, Val, Vec,
};

fn increment_event(
    env: &Env,
    contract_id: &Address,
    key: &Symbol,
    caller: &Address,
    value: u64,
) -> (Address, Vec<Val>, Val) {
    (
        contract_id.clone(),
        vec![
            env,
            symbol_short!("increment").into_val(env),
            key.clone().into_val(env),
        ],
        (caller.clone(), value).into_val(env),
    )
}

fn reset_event(
    env: &Env,
    contract_id: &Address,
    key: &Symbol,
    caller: &Address,
) -> (Address, Vec<Val>, Val) {
    (
        contract_id.clone(),
        vec![
            env,
            symbol_short!("reset").into_val(env),
            key.clone().into_val(env),
        ],
        caller.clone().into_val(env),
    )
}

fn setup(env: &Env) -> (Address, EventCounterClient<'_>, Address) {
    env.mock_all_auths();
    let contract_id = env.register(EventCounter, ());
    let client = EventCounterClient::new(env, &contract_id);
    (contract_id, client, Address::generate(env))
}

#[test]
fn the_first_increment_on_a_fresh_key_returns_one() {
    let env = Env::default();
    let (_, client, caller) = setup(&env);
    let key = symbol_short!("epoch");

    assert_eq!(client.increment(&caller, &key), 1);
    assert_eq!(client.get(&key), 1);
}

#[test]
fn increments_accumulate() {
    let env = Env::default();
    let (_, client, caller) = setup(&env);
    let key = symbol_short!("epoch");

    client.increment(&caller, &key);
    client.increment(&caller, &key);
    let third = client.increment(&caller, &key);

    assert_eq!(third, 3);
    assert_eq!(client.get(&key), 3);
}

#[test]
fn increment_by_adds_the_amount() {
    let env = Env::default();
    let (_, client, caller) = setup(&env);
    let key = symbol_short!("epoch");

    assert_eq!(client.increment_by(&caller, &key, &25), 25);
    assert_eq!(client.increment_by(&caller, &key, &100), 125);
    assert_eq!(client.get(&key), 125);
}

#[test]
fn reset_clears_the_counter_and_the_next_increment_starts_from_one() {
    let env = Env::default();
    let (_, client, caller) = setup(&env);
    let key = symbol_short!("epoch");

    client.increment_by(&caller, &key, &7);
    client.reset(&caller, &key);

    assert_eq!(client.get(&key), 0);
    assert_eq!(client.increment(&caller, &key), 1);
}

#[test]
fn increment_publishes_event() {
    let env = Env::default();
    let (contract_id, client, caller) = setup(&env);
    let key = symbol_short!("epoch");

    client.increment(&caller, &key);

    assert_eq!(
        env.events().all(),
        vec![
            &env,
            increment_event(&env, &contract_id, &key, &caller, 1)
        ]
    );
}

#[test]
fn increment_by_publishes_event_with_the_new_value() {
    let env = Env::default();
    let (contract_id, client, caller) = setup(&env);
    let key = symbol_short!("epoch");

    client.increment_by(&caller, &key, &40);

    assert_eq!(
        env.events().all(),
        vec![
            &env,
            increment_event(&env, &contract_id, &key, &caller, 40)
        ]
    );
}

#[test]
fn reset_publishes_event() {
    let env = Env::default();
    let (contract_id, client, caller) = setup(&env);
    let key = symbol_short!("epoch");

    client.increment(&caller, &key);
    client.reset(&caller, &key);

    assert_eq!(
        env.events().all(),
        vec![&env, reset_event(&env, &contract_id, &key, &caller)]
    );
}

/// `get` is a pure read and must not publish anything.
#[test]
fn reads_publish_nothing() {
    let env = Env::default();
    let (_, client, caller) = setup(&env);
    let key = symbol_short!("epoch");

    client.increment(&caller, &key);

    client.get(&key);
    assert_eq!(env.events().all(), vec![&env]);
}
