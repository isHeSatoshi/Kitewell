//! Edge cases: flows that are unusual but valid.
//!
//! The interesting boundaries for a counter are the value extremes and the
//! independence of one key from another.

use event_counter::{EventCounter, EventCounterClient};
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env, Symbol};

fn setup(env: &Env) -> (EventCounterClient<'_>, Address) {
    env.mock_all_auths();
    let contract_id = env.register(EventCounter, ());
    let client = EventCounterClient::new(env, &contract_id);
    (client, Address::generate(env))
}

#[test]
fn a_key_that_was_never_incremented_reads_zero() {
    let env = Env::default();
    let (client, _) = setup(&env);

    assert_eq!(client.get(&symbol_short!("epoch")), 0);
}

/// Counters are per key, so advancing one must not move another.
#[test]
fn keys_are_independent() {
    let env = Env::default();
    let (client, caller) = setup(&env);
    let a = symbol_short!("alpha");
    let b = symbol_short!("beta");

    client.increment_by(&caller, &a, &10);
    client.increment(&caller, &b);

    assert_eq!(client.get(&a), 10);
    assert_eq!(client.get(&b), 1);
}

/// Keys are symbols compared whole, so one key is never a prefix match for
/// another.
#[test]
fn a_key_is_not_a_prefix_of_another() {
    let env = Env::default();
    let (client, caller) = setup(&env);
    let short = symbol_short!("epo");
    let longer = Symbol::new(&env, "epoch");

    client.increment_by(&caller, &short, &5);

    assert_eq!(client.get(&short), 5);
    assert_eq!(client.get(&longer), 0);
}

/// Keys are not limited to nine characters — `Symbol::new` handles anything up
/// to the symbol limit, and only the *event topic* pays for the length.
#[test]
fn a_long_key_is_a_valid_key() {
    let env = Env::default();
    let (client, caller) = setup(&env);
    let key = Symbol::new(&env, "ledger_close_count_v2");

    assert_eq!(client.increment_by(&caller, &key, &3), 3);
    assert_eq!(client.get(&key), 3);
}

/// A zero increment is allowed. It changes nothing, and it still announces —
/// which is the honest behaviour, because the contract has no way to know
/// whether a caller's announcement is meaningful.
#[test]
fn a_zero_increment_is_allowed_and_announces_nothing_new() {
    let env = Env::default();
    let (client, caller) = setup(&env);
    let key = symbol_short!("epoch");

    client.increment_by(&caller, &key, &5);
    assert_eq!(client.increment_by(&caller, &key, &0), 5);
    assert_eq!(client.get(&key), 5);
}

/// `u64::MAX` is reachable in one call, and once there the counter is exactly
/// full rather than wrapped.
#[test]
fn the_maximum_value_is_reachable() {
    let env = Env::default();
    let (client, caller) = setup(&env);
    let key = symbol_short!("epoch");

    assert_eq!(client.increment_by(&caller, &key, &u64::MAX), u64::MAX);
    assert_eq!(client.get(&key), u64::MAX);
}

/// Several callers can move the same counter; the value is shared and each
/// increment is attributable to whoever signed it.
#[test]
fn several_callers_share_one_counter() {
    let env = Env::default();
    let (client, alice) = setup(&env);
    let bob = Address::generate(&env);
    let key = symbol_short!("epoch");

    client.increment(&alice, &key);
    client.increment(&bob, &key);
    client.increment(&alice, &key);

    assert_eq!(client.get(&key), 3);
}

/// Reset is per key, so clearing one leaves the others alone.
#[test]
fn reset_only_clears_the_named_key() {
    let env = Env::default();
    let (client, caller) = setup(&env);
    let a = symbol_short!("alpha");
    let b = symbol_short!("beta");

    client.increment_by(&caller, &a, &4);
    client.increment_by(&caller, &b, &9);
    client.reset(&caller, &a);

    assert_eq!(client.get(&a), 0);
    assert_eq!(client.get(&b), 9);
}
