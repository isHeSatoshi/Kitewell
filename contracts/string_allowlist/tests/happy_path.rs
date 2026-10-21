//! Happy path for the admin-managed string allowlist.
//!
//! `init`, then add/remove/contains — plus the events an indexer sees for each.

use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _},
    vec, Address, Env, IntoVal, String, Val, Vec,
};
use string_allowlist::{StringAllowlist, StringAllowlistClient};

/// The one event a successful `add` must publish: topic `("added",)`, data
/// `(entry, count)`.
fn added_event(env: &Env, contract_id: &Address, entry: &String, count: u32) -> (Address, Vec<Val>, Val) {
    (
        contract_id.clone(),
        vec![env, symbol_short!("added").into_val(env)],
        (entry.clone(), count).into_val(env),
    )
}

/// The one event a successful `remove` must publish: topic `("removed",)`,
/// data `(entry, count)`.
fn removed_event(
    env: &Env,
    contract_id: &Address,
    entry: &String,
    count: u32,
) -> (Address, Vec<Val>, Val) {
    (
        contract_id.clone(),
        vec![env, symbol_short!("removed").into_val(env)],
        (entry.clone(), count).into_val(env),
    )
}

fn setup(env: &Env) -> (Address, StringAllowlistClient<'_>, Address) {
    env.mock_all_auths();
    let contract_id = env.register(StringAllowlist, ());
    let client = StringAllowlistClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.init(&admin);
    (contract_id, client, admin)
}

#[test]
fn init_sets_the_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(StringAllowlist, ());
    let client = StringAllowlistClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    assert_eq!(client.get_admin(), None);
    client.init(&admin);
    assert_eq!(client.get_admin(), Some(admin));
}

#[test]
fn add_then_contains_is_true() {
    let env = Env::default();
    let (_, client, admin) = setup(&env);
    let entry = String::from_str(&env, "lab-a");

    assert!(!client.contains(&entry));
    client.add(&admin, &entry);
    assert!(client.contains(&entry));
}

#[test]
fn remove_makes_contains_false_again() {
    let env = Env::default();
    let (_, client, admin) = setup(&env);
    let entry = String::from_str(&env, "lab-a");

    client.add(&admin, &entry);
    client.remove(&admin, &entry);

    assert!(!client.contains(&entry));
}

#[test]
fn count_and_list_reflect_the_contents() {
    let env = Env::default();
    let (_, client, admin) = setup(&env);
    let a = String::from_str(&env, "lab-a");
    let b = String::from_str(&env, "lab-b");

    assert_eq!(client.count(), 0);
    assert_eq!(client.list(), Vec::new(&env));

    client.add(&admin, &a);
    client.add(&admin, &b);

    assert_eq!(client.count(), 2);
    assert_eq!(client.list(), vec![&env, a.clone(), b.clone()]);

    client.remove(&admin, &a);

    assert_eq!(client.count(), 1);
    assert_eq!(client.list(), vec![&env, b]);
}

#[test]
fn add_publishes_event() {
    let env = Env::default();
    let (contract_id, client, admin) = setup(&env);
    let entry = String::from_str(&env, "lab-a");

    client.add(&admin, &entry);

    assert_eq!(
        env.events().all(),
        vec![&env, added_event(&env, &contract_id, &entry, 1)]
    );
}

#[test]
fn remove_publishes_event() {
    let env = Env::default();
    let (contract_id, client, admin) = setup(&env);
    let a = String::from_str(&env, "lab-a");
    let b = String::from_str(&env, "lab-b");

    client.add(&admin, &a);
    client.add(&admin, &b);
    client.remove(&admin, &a);

    assert_eq!(
        env.events().all(),
        vec![&env, removed_event(&env, &contract_id, &a, 1)]
    );
}

/// The read methods are pure: they must not publish anything, so the log stays
/// a faithful record of state changes only.
#[test]
fn reads_publish_nothing() {
    let env = Env::default();
    let (_, client, admin) = setup(&env);
    let entry = String::from_str(&env, "lab-a");

    client.add(&admin, &entry);

    client.contains(&entry);
    assert_eq!(env.events().all(), vec![&env]);

    client.count();
    assert_eq!(env.events().all(), vec![&env]);

    client.list();
    assert_eq!(env.events().all(), vec![&env]);

    client.get_admin();
    assert_eq!(env.events().all(), vec![&env]);
}
