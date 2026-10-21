//! Edge cases: flows that are unusual but valid.
//!
//! The interesting boundaries for a capped set are the exact limit, the slot
//! freed by a removal, and the order a caller reads back.

use builder_tag_registry::{BuilderTagRegistry, BuilderTagRegistryClient, MAX_TAG_LEN};
use soroban_sdk::{testutils::Address as _, vec, Address, Env, String, Vec};

fn setup(env: &Env) -> (BuilderTagRegistryClient<'_>, Address) {
    env.mock_all_auths();
    let contract_id = env.register(BuilderTagRegistry, ());
    let client = BuilderTagRegistryClient::new(env, &contract_id);
    (client, Address::generate(env))
}

/// A string of exactly `len` bytes, built on the host where allocation is free.
fn tag_of_len(env: &Env, len: usize) -> String {
    String::from_str(env, &"t".repeat(len))
}

#[test]
fn a_fresh_builder_has_no_tags() {
    let env = Env::default();
    let (client, builder) = setup(&env);

    assert_eq!(client.count_tags(&builder), 0);
    assert_eq!(client.list_tags(&builder), Vec::new(&env));
    assert!(!client.has_tag(&builder, &String::from_str(&env, "anything")));
}

/// Removing a tag closes the gap instead of swapping the last tag into it, so
/// the order a builder built is the order a reader gets back.
#[test]
fn order_is_preserved_across_removals() {
    let env = Env::default();
    let (client, builder) = setup(&env);
    let a = String::from_str(&env, "a");
    let b = String::from_str(&env, "b");
    let c = String::from_str(&env, "c");

    client.add_tag(&builder, &a);
    client.add_tag(&builder, &b);
    client.add_tag(&builder, &c);
    client.remove_tag(&builder, &b);

    assert_eq!(client.list_tags(&builder), vec![&env, a, c]);
}

#[test]
fn a_tag_of_exactly_the_maximum_length_is_accepted() {
    let env = Env::default();
    let (client, builder) = setup(&env);
    let tag = tag_of_len(&env, MAX_TAG_LEN as usize);

    client.add_tag(&builder, &tag);

    assert!(client.has_tag(&builder, &tag));
    assert_eq!(client.count_tags(&builder), 1);
}

#[test]
fn a_tag_can_be_added_again_after_removal() {
    let env = Env::default();
    let (client, builder) = setup(&env);
    let tag = String::from_str(&env, "audit");

    client.add_tag(&builder, &tag);
    client.remove_tag(&builder, &tag);
    client.add_tag(&builder, &tag);

    assert!(client.has_tag(&builder, &tag));
    assert_eq!(client.count_tags(&builder), 1);
}

/// Tags are compared byte-for-byte; case is not normalised, because whether
/// `"Audit"` and `"audit"` are the same label is a policy question.
#[test]
fn tags_are_case_sensitive() {
    let env = Env::default();
    let (client, builder) = setup(&env);
    let lower = String::from_str(&env, "audit");
    let upper = String::from_str(&env, "AUDIT");

    client.add_tag(&builder, &lower);

    assert!(client.has_tag(&builder, &lower));
    assert!(!client.has_tag(&builder, &upper));

    client.add_tag(&builder, &upper);
    assert_eq!(client.count_tags(&builder), 2);
}

/// The cap is exact: the eighth tag fits, the ninth does not, and removing one
/// frees exactly one slot.
#[test]
fn the_cap_is_exact_and_a_removal_frees_one_slot() {
    use builder_tag_registry::MAX_TAGS_PER_BUILDER;

    let env = Env::default();
    let (client, builder) = setup(&env);

    let mut i = 0u32;
    while i < MAX_TAGS_PER_BUILDER {
        client.add_tag(&builder, &String::from_str(&env, &format!("t{i}")));
        i += 1;
    }
    assert_eq!(client.count_tags(&builder), MAX_TAGS_PER_BUILDER);

    client.remove_tag(&builder, &String::from_str(&env, "t0"));
    client.add_tag(&builder, &String::from_str(&env, "fresh"));

    assert_eq!(client.count_tags(&builder), MAX_TAGS_PER_BUILDER);
    assert!(client.has_tag(&builder, &String::from_str(&env, "fresh")));
}

/// Removing a tag that is not on the builder must not disturb the tags that
/// are, even when the tag is carried by somebody else.
#[test]
fn a_failed_removal_leaves_the_set_alone() {
    let env = Env::default();
    let (client, builder) = setup(&env);
    let other = Address::generate(&env);
    let shared = String::from_str(&env, "audit");
    let mine = String::from_str(&env, "soroban");

    client.add_tag(&builder, &mine);
    client.add_tag(&other, &shared);

    assert!(client.try_remove_tag(&builder, &shared).is_err());
    assert_eq!(client.list_tags(&builder), vec![&env, mine]);
}
