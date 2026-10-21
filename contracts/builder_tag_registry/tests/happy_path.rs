//! Happy path for the per-builder tag registry.
//!
//! Add a tag, read it back, remove it — plus the events an indexer sees.

use builder_tag_registry::{BuilderTagRegistry, BuilderTagRegistryClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _},
    vec, Address, Env, IntoVal, String, Val, Vec,
};

fn tagged_event(
    env: &Env,
    contract_id: &Address,
    builder: &Address,
    tag: &String,
    count: u32,
) -> (Address, Vec<Val>, Val) {
    (
        contract_id.clone(),
        vec![
            env,
            symbol_short!("tagged").into_val(env),
            builder.clone().into_val(env),
        ],
        (tag.clone(), count).into_val(env),
    )
}

fn untagged_event(
    env: &Env,
    contract_id: &Address,
    builder: &Address,
    tag: &String,
    count: u32,
) -> (Address, Vec<Val>, Val) {
    (
        contract_id.clone(),
        vec![
            env,
            symbol_short!("untagged").into_val(env),
            builder.clone().into_val(env),
        ],
        (tag.clone(), count).into_val(env),
    )
}

#[test]
fn add_tag_then_has_tag_is_true() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BuilderTagRegistry, ());
    let client = BuilderTagRegistryClient::new(&env, &contract_id);
    let builder = Address::generate(&env);
    let tag = String::from_str(&env, "soroban");

    assert!(!client.has_tag(&builder, &tag));
    client.add_tag(&builder, &tag);
    assert!(client.has_tag(&builder, &tag));
}

#[test]
fn remove_tag_makes_has_tag_false_again() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BuilderTagRegistry, ());
    let client = BuilderTagRegistryClient::new(&env, &contract_id);
    let builder = Address::generate(&env);
    let tag = String::from_str(&env, "soroban");

    client.add_tag(&builder, &tag);
    client.remove_tag(&builder, &tag);

    assert!(!client.has_tag(&builder, &tag));
}

#[test]
fn count_and_list_reflect_the_tags() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BuilderTagRegistry, ());
    let client = BuilderTagRegistryClient::new(&env, &contract_id);
    let builder = Address::generate(&env);
    let a = String::from_str(&env, "audit");
    let b = String::from_str(&env, "testnet");

    assert_eq!(client.count_tags(&builder), 0);
    assert_eq!(client.list_tags(&builder), Vec::new(&env));

    client.add_tag(&builder, &a);
    client.add_tag(&builder, &b);

    assert_eq!(client.count_tags(&builder), 2);
    assert_eq!(client.list_tags(&builder), vec![&env, a.clone(), b.clone()]);

    client.remove_tag(&builder, &a);

    assert_eq!(client.count_tags(&builder), 1);
    assert_eq!(client.list_tags(&builder), vec![&env, b]);
}

#[test]
fn add_tag_publishes_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BuilderTagRegistry, ());
    let client = BuilderTagRegistryClient::new(&env, &contract_id);
    let builder = Address::generate(&env);
    let tag = String::from_str(&env, "audit");

    client.add_tag(&builder, &tag);

    assert_eq!(
        env.events().all(),
        vec![
            &env,
            tagged_event(&env, &contract_id, &builder, &tag, 1)
        ]
    );
}

#[test]
fn remove_tag_publishes_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BuilderTagRegistry, ());
    let client = BuilderTagRegistryClient::new(&env, &contract_id);
    let builder = Address::generate(&env);
    let a = String::from_str(&env, "audit");
    let b = String::from_str(&env, "testnet");

    client.add_tag(&builder, &a);
    client.add_tag(&builder, &b);
    client.remove_tag(&builder, &a);

    assert_eq!(
        env.events().all(),
        vec![
            &env,
            untagged_event(&env, &contract_id, &builder, &a, 1)
        ]
    );
}

/// The read methods are pure: they must not publish anything.
#[test]
fn reads_publish_nothing() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BuilderTagRegistry, ());
    let client = BuilderTagRegistryClient::new(&env, &contract_id);
    let builder = Address::generate(&env);
    let tag = String::from_str(&env, "audit");

    client.add_tag(&builder, &tag);

    client.list_tags(&builder);
    assert_eq!(env.events().all(), vec![&env]);

    client.count_tags(&builder);
    assert_eq!(env.events().all(), vec![&env]);

    client.has_tag(&builder, &tag);
    assert_eq!(env.events().all(), vec![&env]);
}
