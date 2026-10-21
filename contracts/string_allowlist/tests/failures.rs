//! Failure and error cases.
//!
//! Every rejection is asserted against the exact `Error` variant rather than
//! "something failed", so the codes stay pinned to the contract spec. Auth
//! failures are asserted separately, because those surface as an `InvokeError`
//! rather than as one of our variants.

use soroban_sdk::{
    testutils::{Address as _, Events as _},
    vec, Address, Env, String,
};
use string_allowlist::{Error, StringAllowlist, StringAllowlistClient, MAX_ENTRIES, MAX_ENTRY_LEN};

fn setup(env: &Env) -> (StringAllowlistClient<'_>, Address) {
    env.mock_all_auths();
    let contract_id = env.register(StringAllowlist, ());
    let client = StringAllowlistClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.init(&admin);
    (client, admin)
}

fn entry_of_len(env: &Env, len: usize) -> String {
    String::from_str(env, &"a".repeat(len))
}

#[test]
fn init_cannot_run_twice() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let usurper = Address::generate(&env);

    assert_eq!(client.try_init(&usurper), Err(Ok(Error::AlreadyInit)));
    // And the original admin is untouched.
    assert_eq!(client.get_admin(), Some(admin));
}

#[test]
fn a_non_admin_cannot_add() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let stranger = Address::generate(&env);

    assert_eq!(
        client.try_add(&stranger, &String::from_str(&env, "lab-a")),
        Err(Ok(Error::NotAdmin))
    );
    assert_eq!(client.count(), 0);
}

#[test]
fn a_non_admin_cannot_remove() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let stranger = Address::generate(&env);
    let entry = String::from_str(&env, "lab-a");

    client.add(&admin, &entry);

    assert_eq!(
        client.try_remove(&stranger, &entry),
        Err(Ok(Error::NotAdmin))
    );
    assert!(client.contains(&entry));
}

/// Before `init` there is no admin, so every mutating call is refused. The
/// error is the same `NotAdmin` as a wrong caller, so the contract does not
/// leak whether it has been initialised.
#[test]
fn mutating_before_init_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(StringAllowlist, ());
    let client = StringAllowlistClient::new(&env, &contract_id);
    let anyone = Address::generate(&env);

    assert_eq!(
        client.try_add(&anyone, &String::from_str(&env, "lab-a")),
        Err(Ok(Error::NotAdmin))
    );
    assert_eq!(
        client.try_remove(&anyone, &String::from_str(&env, "lab-a")),
        Err(Ok(Error::NotAdmin))
    );
}

/// An empty entry would make `contains("")` true for every caller, which is
/// never what an allowlist means.
#[test]
fn an_empty_entry_is_rejected() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    assert_eq!(
        client.try_add(&admin, &String::from_str(&env, "")),
        Err(Ok(Error::EmptyEntry))
    );
    assert_eq!(client.count(), 0);
}

#[test]
fn an_over_long_entry_is_rejected() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    assert_eq!(
        client.try_add(&admin, &entry_of_len(&env, MAX_ENTRY_LEN as usize + 1)),
        Err(Ok(Error::EntryTooLong))
    );
    assert_eq!(client.count(), 0);
}

/// A duplicate add is reported rather than silently ignored, so a caller is
/// never left believing an add did something it did not.
#[test]
fn adding_a_duplicate_is_rejected() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let entry = String::from_str(&env, "lab-a");

    client.add(&admin, &entry);

    assert_eq!(
        client.try_add(&admin, &entry),
        Err(Ok(Error::AlreadyAllowed))
    );
    assert_eq!(client.count(), 1);
}

#[test]
fn removing_an_absent_entry_is_rejected() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    assert_eq!(
        client.try_remove(&admin, &String::from_str(&env, "never-added")),
        Err(Ok(Error::NotAllowed))
    );
    assert_eq!(client.count(), 0);
}

#[test]
fn a_full_list_rejects_further_adds() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let mut i = 0u32;
    while i < MAX_ENTRIES {
        client.add(&admin, &String::from_str(&env, &format!("entry-{i:03}")));
        i += 1;
    }

    assert_eq!(
        client.try_add(&admin, &String::from_str(&env, "one-too-many")),
        Err(Ok(Error::AllowlistFull))
    );
    assert_eq!(client.count(), MAX_ENTRIES);
}

/// `init` requires the incoming admin's signature, so the initialiser cannot be
/// front-run: an onlooker who sees the deploy cannot seize the list by calling
/// `init` first.
#[test]
fn initialising_without_auth_is_rejected() {
    let env = Env::default();
    let contract_id = env.register(StringAllowlist, ());
    let client = StringAllowlistClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    // No auth is mocked, so `require_auth` on the admin must fail.
    assert!(client.try_init(&admin).is_err());
    assert_eq!(client.get_admin(), None);
}

#[test]
fn adding_without_auth_is_rejected() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    env.set_auths(&[]);
    assert!(client.try_add(&admin, &String::from_str(&env, "lab-a")).is_err());
    assert_eq!(client.count(), 0);
}

#[test]
fn a_wrong_signature_cannot_remove() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let entry = String::from_str(&env, "lab-a");
    client.add(&admin, &entry);

    env.set_auths(&[]);
    assert!(client.try_remove(&admin, &entry).is_err());
    assert!(client.contains(&entry));
}

/// Nothing that fails may publish an event, so the log stays a faithful record
/// of state changes only.
#[test]
fn rejected_calls_publish_nothing() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let entry = String::from_str(&env, "lab-a");
    client.add(&admin, &entry);

    assert!(client.try_add(&admin, &entry).is_err());
    assert_eq!(env.events().all(), vec![&env]);

    assert!(client.try_add(&admin, &String::from_str(&env, "")).is_err());
    assert_eq!(env.events().all(), vec![&env]);

    assert!(client
        .try_remove(&admin, &String::from_str(&env, "absent"))
        .is_err());
    assert_eq!(env.events().all(), vec![&env]);

    let stranger = Address::generate(&env);
    assert!(client
        .try_add(&stranger, &String::from_str(&env, "lab-b"))
        .is_err());
    assert_eq!(env.events().all(), vec![&env]);
}
