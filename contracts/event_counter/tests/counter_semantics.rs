//! Counter semantics — the properties that make this a *shared, auditable*
//! counter rather than a number in someone's database.
//!
//! A consumer follows the event log instead of polling storage, so three things
//! are load-bearing: the key must be filterable, the value in the event must be
//! the value in storage, and the count must never move silently.

use event_counter::{EventCounter, EventCounterClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _},
    vec, Address, Env, IntoVal, Symbol, Val, Vec,
};

fn setup(env: &Env) -> (Address, EventCounterClient<'_>, Address) {
    env.mock_all_auths();
    let contract_id = env.register(EventCounter, ());
    let client = EventCounterClient::new(env, &contract_id);
    (contract_id, client, Address::generate(env))
}

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

/// The key is a topic, so a consumer can subscribe to one counter without
/// decoding the data of every event in the contract.
#[test]
fn the_key_is_a_topic() {
    let env = Env::default();
    let (contract_id, client, caller) = setup(&env);
    let watched = symbol_short!("epoch");
    let ignored = symbol_short!("other");

    client.increment(&caller, &ignored);
    client.increment_by(&caller, &watched, &12);

    // The last event is the one for the key we asked about, and it carries that
    // key as its second topic.
    assert_eq!(
        env.events().all(),
        vec![
            &env,
            increment_event(&env, &contract_id, &watched, &caller, 12)
        ]
    );
}

/// Every increment names who signed it, so the count is attributable rather
/// than anonymous.
#[test]
fn the_caller_is_attributable() {
    let env = Env::default();
    let (contract_id, client, alice) = setup(&env);
    let bob = Address::generate(&env);
    let key = symbol_short!("epoch");

    client.increment(&alice, &key);
    assert_eq!(
        env.events().all(),
        vec![
            &env,
            increment_event(&env, &contract_id, &key, &alice, 1)
        ]
    );

    client.increment(&bob, &key);
    assert_eq!(
        env.events().all(),
        vec![&env, increment_event(&env, &contract_id, &key, &bob, 2)]
    );
}

/// A consumer that only sees events can track the value without ever reading
/// storage, because each event carries the running total.
#[test]
fn the_log_alone_tracks_the_value() {
    let env = Env::default();
    let (contract_id, client, caller) = setup(&env);
    let key = symbol_short!("epoch");

    for (amount, expected) in [(3u64, 3u64), (4, 7), (0, 7), (13, 20)] {
        client.increment_by(&caller, &key, &amount);
        assert_eq!(
            env.events().all(),
            vec![
                &env,
                increment_event(&env, &contract_id, &key, &caller, expected)
            ]
        );
        // The event and storage never disagree.
        assert_eq!(client.get(&key), expected);
    }
}

/// The counter refuses to wrap. A silent wrap would be the worst possible
/// failure for a number two contracts might be reconciling against.
#[test]
fn the_counter_never_silently_wraps() {
    let env = Env::default();
    let (_, client, caller) = setup(&env);
    let key = symbol_short!("epoch");

    client.increment_by(&caller, &key, &u64::MAX);
    assert!(client.try_increment(&caller, &key).is_err());

    // Not zero, and not saturated past the maximum.
    assert_eq!(client.get(&key), u64::MAX);
}

/// After a reset the key reads exactly as it did before it was ever counted, so
/// no consumer can tell a cleared counter from a fresh one — which is what
/// "cleared" has to mean.
#[test]
fn a_reset_key_is_indistinguishable_from_a_never_counted_one() {
    let env = Env::default();
    let (_, client, caller) = setup(&env);
    let cleared = symbol_short!("clear");
    let fresh = symbol_short!("fresh");

    client.increment_by(&caller, &cleared, &99);
    client.reset(&caller, &cleared);

    assert_eq!(client.get(&cleared), client.get(&fresh));

    // And the next increment on each behaves identically.
    assert_eq!(client.increment(&caller, &cleared), 1);
    assert_eq!(client.increment(&caller, &fresh), 1);
}

/// A reset announces itself with the same key topic, so a consumer following
/// one key sees both kinds of change in order.
#[test]
fn a_reset_is_visible_on_the_same_topic() {
    let env = Env::default();
    let (contract_id, client, caller) = setup(&env);
    let key = symbol_short!("epoch");

    client.increment_by(&caller, &key, &5);
    client.reset(&caller, &key);

    assert_eq!(
        env.events().all(),
        vec![
            &env,
            (
                contract_id.clone(),
                vec![
                    &env,
                    symbol_short!("reset").into_val(&env),
                    key.clone().into_val(&env),
                ],
                caller.clone().into_val(&env),
            )
        ]
    );
}
