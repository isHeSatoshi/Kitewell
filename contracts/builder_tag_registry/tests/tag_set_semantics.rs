//! Tag-set semantics — the properties that make this a *registry* rather than
//! a list of strings.
//!
//! Three things are load-bearing for anyone reading these tags: the set must be
//! namespaced by builder, a removal must leave no trace, and the cap must be
//! per builder so one busy address cannot crowd out another.

use builder_tag_registry::{
    BuilderTagRegistry, BuilderTagRegistryClient, MAX_TAGS_PER_BUILDER,
};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _},
    vec, Address, Env, IntoVal, String, Vec,
};

fn setup(env: &Env) -> (Address, BuilderTagRegistryClient<'_>, Address) {
    env.mock_all_auths();
    let contract_id = env.register(BuilderTagRegistry, ());
    let client = BuilderTagRegistryClient::new(env, &contract_id);
    (contract_id, client, Address::generate(env))
}

/// Tags belong to the address that carries them. One builder's tags must not
/// appear on another's, including for the same tag string.
#[test]
fn tags_are_namespaced_by_builder() {
    let env = Env::default();
    let (_, client, alice) = setup(&env);
    let bob = Address::generate(&env);
    let shared = String::from_str(&env, "audit");

    client.add_tag(&alice, &shared);

    assert!(client.has_tag(&alice, &shared));
    assert!(!client.has_tag(&bob, &shared));
    assert_eq!(client.count_tags(&bob), 0);
    assert_eq!(client.list_tags(&bob), Vec::new(&env));
}

/// The cap is per builder, so filling one address must not consume anybody
/// else's budget — the bug a single global list would have.
#[test]
fn the_cap_is_per_builder() {
    let env = Env::default();
    let (_, client, alice) = setup(&env);
    let bob = Address::generate(&env);

    let mut i = 0u32;
    while i < MAX_TAGS_PER_BUILDER {
        client.add_tag(&alice, &String::from_str(&env, &format!("a{i}")));
        i += 1;
    }
    assert_eq!(client.count_tags(&alice), MAX_TAGS_PER_BUILDER);

    // Bob still has his full budget.
    let mut i = 0u32;
    while i < MAX_TAGS_PER_BUILDER {
        client.add_tag(&bob, &String::from_str(&env, &format!("b{i}")));
        i += 1;
    }
    assert_eq!(client.count_tags(&bob), MAX_TAGS_PER_BUILDER);
}

/// After a removal there must be no trace a reader could observe: same
/// `has_tag`, and absent from `list_tags` and `count_tags` alike. Otherwise a
/// consumer filtering on a revoked tag could still match the builder.
#[test]
fn a_removed_tag_leaves_no_trace() {
    let env = Env::default();
    let (_, client, builder) = setup(&env);
    let revoked = String::from_str(&env, "revoked");
    let never = String::from_str(&env, "never");

    client.add_tag(&builder, &revoked);
    client.add_tag(&builder, &String::from_str(&env, "kept"));
    client.remove_tag(&builder, &revoked);

    assert_eq!(client.has_tag(&builder, &revoked), client.has_tag(&builder, &never));
    assert!(!client.list_tags(&builder).contains(revoked.clone()));
    assert!(!client.list_tags(&builder).contains(never));
    assert_eq!(client.count_tags(&builder), 1);
}

/// A consumer that only sees events must be able to reconstruct the tag set, so
/// each mutation publishes the tag and the resulting count under the builder's
/// own topic.
#[test]
fn the_event_log_mirrors_the_tag_set() {
    let env = Env::default();
    let (contract_id, client, builder) = setup(&env);
    let a = String::from_str(&env, "audit");
    let b = String::from_str(&env, "testnet");

    client.add_tag(&builder, &a);
    assert_eq!(
        env.events().all(),
        vec![
            &env,
            (
                contract_id.clone(),
                vec![
                    &env,
                    symbol_short!("tagged").into_val(&env),
                    builder.clone().into_val(&env),
                ],
                (a.clone(), 1u32).into_val(&env),
            )
        ]
    );

    client.add_tag(&builder, &b);
    assert_eq!(
        env.events().all(),
        vec![
            &env,
            (
                contract_id.clone(),
                vec![
                    &env,
                    symbol_short!("tagged").into_val(&env),
                    builder.clone().into_val(&env),
                ],
                (b.clone(), 2u32).into_val(&env),
            )
        ]
    );

    client.remove_tag(&builder, &a);
    assert_eq!(
        env.events().all(),
        vec![
            &env,
            (
                contract_id.clone(),
                vec![
                    &env,
                    symbol_short!("untagged").into_val(&env),
                    builder.clone().into_val(&env),
                ],
                (a.clone(), 1u32).into_val(&env),
            )
        ]
    );

    // The log and the live set agree.
    assert_eq!(client.list_tags(&builder), vec![&env, b]);
}

/// A tag is a claim about a builder, so only that builder may make it. Without
/// the signature anyone could label anyone and the tags would be worth nothing.
#[test]
fn a_builder_cannot_be_labelled_without_their_signature() {
    let env = Env::default();
    let contract_id = env.register(BuilderTagRegistry, ());
    let client = BuilderTagRegistryClient::new(&env, &contract_id);
    let victim = Address::generate(&env);

    assert!(client
        .try_add_tag(&victim, &String::from_str(&env, "scammer"))
        .is_err());
    assert_eq!(client.count_tags(&victim), 0);
}

/// Reads stay open while writes are closed, so a consumer can filter on tags
/// without holding a key.
#[test]
fn reads_are_open_while_writes_are_closed() {
    let env = Env::default();
    let (_, client, builder) = setup(&env);
    let tag = String::from_str(&env, "audit");
    client.add_tag(&builder, &tag);

    env.set_auths(&[]);

    assert!(client.has_tag(&builder, &tag));
    assert_eq!(client.count_tags(&builder), 1);
    assert_eq!(client.list_tags(&builder), vec![&env, tag.clone()]);

    assert!(client.try_add_tag(&builder, &tag).is_err());
    assert!(client.try_remove_tag(&builder, &tag).is_err());
}
