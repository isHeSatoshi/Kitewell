//! Failure and error cases.
//!
//! Every rejection is asserted against the exact `Error` variant rather than
//! "something failed", so the codes stay pinned to the contract spec. Auth
//! failures are asserted separately, because those surface as an `InvokeError`
//! rather than as one of our variants.

use builder_tag_registry::{
    BuilderTagRegistry, BuilderTagRegistryClient, Error, MAX_TAG_LEN, MAX_TAGS_PER_BUILDER,
};
use soroban_sdk::{
    testutils::{Address as _, Events as _},
    vec, Address, Env, String,
};

fn setup(env: &Env) -> (BuilderTagRegistryClient<'_>, Address) {
    env.mock_all_auths();
    let contract_id = env.register(BuilderTagRegistry, ());
    let client = BuilderTagRegistryClient::new(env, &contract_id);
    (client, Address::generate(env))
}

#[test]
fn an_empty_tag_is_rejected() {
    let env = Env::default();
    let (client, builder) = setup(&env);

    assert_eq!(
        client.try_add_tag(&builder, &String::from_str(&env, "")),
        Err(Ok(Error::EmptyTag))
    );
    assert_eq!(client.count_tags(&builder), 0);
}

#[test]
fn an_over_long_tag_is_rejected() {
    let env = Env::default();
    let (client, builder) = setup(&env);
    let too_long = String::from_str(&env, &"t".repeat(MAX_TAG_LEN as usize + 1));

    assert_eq!(
        client.try_add_tag(&builder, &too_long),
        Err(Ok(Error::TagTooLong))
    );
    assert_eq!(client.count_tags(&builder), 0);
}

/// A duplicate is reported rather than silently ignored, so a caller is never
/// left believing an add did something it did not.
#[test]
fn a_duplicate_tag_is_rejected() {
    let env = Env::default();
    let (client, builder) = setup(&env);
    let tag = String::from_str(&env, "audit");

    client.add_tag(&builder, &tag);

    assert_eq!(
        client.try_add_tag(&builder, &tag),
        Err(Ok(Error::TagAlreadyPresent))
    );
    assert_eq!(client.count_tags(&builder), 1);
}

#[test]
fn removing_a_tag_that_is_not_present_is_rejected() {
    let env = Env::default();
    let (client, builder) = setup(&env);

    assert_eq!(
        client.try_remove_tag(&builder, &String::from_str(&env, "never-added")),
        Err(Ok(Error::TagNotPresent))
    );
}

#[test]
fn exceeding_the_cap_is_rejected() {
    let env = Env::default();
    let (client, builder) = setup(&env);

    let mut i = 0u32;
    while i < MAX_TAGS_PER_BUILDER {
        client.add_tag(&builder, &String::from_str(&env, &format!("t{i}")));
        i += 1;
    }

    assert_eq!(
        client.try_add_tag(&builder, &String::from_str(&env, "one-too-many")),
        Err(Ok(Error::TooManyTags))
    );
    assert_eq!(client.count_tags(&builder), MAX_TAGS_PER_BUILDER);
}

/// The cap is checked after the duplicate check, so a builder at the limit who
/// re-adds a tag they already hold gets `TagAlreadyPresent`, not `TooManyTags`.
/// Either way nothing changes, but the error tells the caller which problem
/// they actually have.
#[test]
fn a_duplicate_is_reported_even_when_the_builder_is_full() {
    let env = Env::default();
    let (client, builder) = setup(&env);

    let mut i = 0u32;
    while i < MAX_TAGS_PER_BUILDER {
        client.add_tag(&builder, &String::from_str(&env, &format!("t{i}")));
        i += 1;
    }

    assert_eq!(
        client.try_add_tag(&builder, &String::from_str(&env, "t0")),
        Err(Ok(Error::TagAlreadyPresent))
    );
}

#[test]
fn adding_a_tag_without_a_signature_is_rejected() {
    let env = Env::default();
    let contract_id = env.register(BuilderTagRegistry, ());
    let client = BuilderTagRegistryClient::new(&env, &contract_id);
    let builder = Address::generate(&env);

    // No auth is mocked, so `require_auth` on the builder must fail.
    assert!(client
        .try_add_tag(&builder, &String::from_str(&env, "audit"))
        .is_err());
    assert_eq!(client.count_tags(&builder), 0);
}

#[test]
fn removing_a_tag_without_a_signature_is_rejected() {
    let env = Env::default();
    let (client, builder) = setup(&env);
    let tag = String::from_str(&env, "audit");
    client.add_tag(&builder, &tag);

    env.set_auths(&[]);
    assert!(client.try_remove_tag(&builder, &tag).is_err());
    assert!(client.has_tag(&builder, &tag));
}

/// Nothing that fails may publish an event, so the log stays a faithful record
/// of state changes only.
#[test]
fn rejected_calls_publish_nothing() {
    let env = Env::default();
    let (client, builder) = setup(&env);
    let tag = String::from_str(&env, "audit");
    client.add_tag(&builder, &tag);

    assert!(client.try_add_tag(&builder, &tag).is_err());
    assert_eq!(env.events().all(), vec![&env]);

    assert!(client
        .try_add_tag(&builder, &String::from_str(&env, ""))
        .is_err());
    assert_eq!(env.events().all(), vec![&env]);

    assert!(client
        .try_remove_tag(&builder, &String::from_str(&env, "absent"))
        .is_err());
    assert_eq!(env.events().all(), vec![&env]);
}
