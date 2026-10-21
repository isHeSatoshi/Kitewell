//! Allowlist semantics — the properties that make this an allowlist and not
//! just a bag of strings.
//!
//! A client contract gates on `contains`. That makes three things load-bearing:
//! membership must be exact, a removed entry must be indistinguishable from one
//! that was never added, and the read side must be callable without a
//! signature.

use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _},
    vec, Address, Env, IntoVal, String,
};
use string_allowlist::{Error, StringAllowlist, StringAllowlistClient};

fn setup(env: &Env) -> (Address, StringAllowlistClient<'_>, Address) {
    env.mock_all_auths();
    let contract_id = env.register(StringAllowlist, ());
    let client = StringAllowlistClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.init(&admin);
    (contract_id, client, admin)
}

/// Membership is the contract's whole purpose: it must flip on add and back on
/// remove, and nothing else may move it.
#[test]
fn contains_is_the_authority_on_membership() {
    let env = Env::default();
    let (_, client, admin) = setup(&env);
    let allowed = String::from_str(&env, "lab-a");
    let other = String::from_str(&env, "lab-b");

    assert!(!client.contains(&allowed));

    client.add(&admin, &allowed);
    assert!(client.contains(&allowed));
    // Adding one entry must not admit any other.
    assert!(!client.contains(&other));

    client.remove(&admin, &allowed);
    assert!(!client.contains(&allowed));
}

/// After a removal there must be no trace of the entry a client could observe:
/// same `contains`, and absent from `list` and `count` alike. Otherwise a
/// contract that trusted the allowlist could be gated on a revoked name.
#[test]
fn a_removed_entry_is_indistinguishable_from_one_never_added() {
    let env = Env::default();
    let (_, client, admin) = setup(&env);
    let revoked = String::from_str(&env, "revoked");
    let never = String::from_str(&env, "never");

    client.add(&admin, &revoked);
    client.add(&admin, &String::from_str(&env, "kept"));
    client.remove(&admin, &revoked);

    assert_eq!(client.contains(&revoked), client.contains(&never));
    assert!(!client.list().contains(revoked.clone()));
    assert!(!client.list().contains(never));
    assert_eq!(client.count(), 1);
}

/// An indexer that only sees events must be able to reconstruct the list, so
/// each mutation publishes the entry and the resulting count.
#[test]
fn each_mutation_publishes_an_event_that_mirrors_the_list() {
    let env = Env::default();
    let (contract_id, client, admin) = setup(&env);
    let a = String::from_str(&env, "lab-a");
    let b = String::from_str(&env, "lab-b");

    client.add(&admin, &a);
    assert_eq!(
        env.events().all(),
        vec![
            &env,
            (
                contract_id.clone(),
                vec![&env, symbol_short!("added").into_val(&env)],
                (a.clone(), 1u32).into_val(&env),
            )
        ]
    );

    client.add(&admin, &b);
    assert_eq!(
        env.events().all(),
        vec![
            &env,
            (
                contract_id.clone(),
                vec![&env, symbol_short!("added").into_val(&env)],
                (b.clone(), 2u32).into_val(&env),
            )
        ]
    );

    client.remove(&admin, &a);
    assert_eq!(
        env.events().all(),
        vec![
            &env,
            (
                contract_id.clone(),
                vec![&env, symbol_short!("removed").into_val(&env)],
                (a.clone(), 1u32).into_val(&env),
            )
        ]
    );

    // The log and the live list agree.
    assert_eq!(client.list(), vec![&env, b]);
}

/// The asymmetry that makes an allowlist useful: only the admin can change it,
/// but anyone can read it. A read that needed a signature would be useless to
/// the callers that need it most.
#[test]
fn writes_are_admin_only_but_reads_are_open() {
    let env = Env::default();
    let (_, client, admin) = setup(&env);
    let entry = String::from_str(&env, "lab-a");
    client.add(&admin, &entry);

    // Drop every mocked auth: reads must still work.
    env.set_auths(&[]);

    assert!(client.contains(&entry));
    assert_eq!(client.count(), 1);
    assert_eq!(client.list(), vec![&env, entry.clone()]);
    assert_eq!(client.get_admin(), Some(admin.clone()));

    // While any write is refused.
    assert!(client.try_add(&admin, &String::from_str(&env, "lab-b")).is_err());
    assert!(client.try_remove(&admin, &entry).is_err());
    assert_eq!(client.count(), 1);
}

/// Both bounds hold: the length of an entry and the size of the list. Without
/// them the linear scan in `contains` has no ceiling.
#[test]
fn the_list_is_bounded_on_both_axes() {
    let env = Env::default();
    let (_, client, admin) = setup(&env);

    // Length bound.
    assert_eq!(
        client.try_add(&admin, &String::from_str(&env, &"a".repeat(65))),
        Err(Ok(Error::EntryTooLong))
    );
    // Size bound.
    let mut i = 0u32;
    while i < 100 {
        client.add(&admin, &String::from_str(&env, &format!("e{i:03}")));
        i += 1;
    }
    assert_eq!(
        client.try_add(&admin, &String::from_str(&env, "overflow")),
        Err(Ok(Error::AllowlistFull))
    );
}
