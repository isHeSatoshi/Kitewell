//! Edge cases: flows that are unusual but valid.
//!
//! The boundaries are the interesting part of a bounded set — an entry of
//! exactly the maximum length, a list that has just been emptied, and the order
//! a caller gets back after a removal.

use soroban_sdk::{
    testutils::Address as _,
    vec, Address, Env, String, Vec,
};
use string_allowlist::{StringAllowlist, StringAllowlistClient, MAX_ENTRY_LEN};

fn setup(env: &Env) -> (StringAllowlistClient<'_>, Address) {
    env.mock_all_auths();
    let contract_id = env.register(StringAllowlist, ());
    let client = StringAllowlistClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.init(&admin);
    (client, admin)
}

/// A string of exactly `len` bytes, built on the host where allocation is free.
fn entry_of_len(env: &Env, len: usize) -> String {
    String::from_str(env, &"a".repeat(len))
}

#[test]
fn a_fresh_contract_is_empty() {
    let env = Env::default();
    let (client, _) = setup(&env);

    assert_eq!(client.count(), 0);
    assert_eq!(client.list(), Vec::new(&env));
    assert!(!client.contains(&String::from_str(&env, "anything")));
}

/// Removing an entry closes the gap instead of swapping the last element into
/// it, so the order an admin built is the order a client reads back.
#[test]
fn insertion_order_is_preserved_across_removals() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let a = String::from_str(&env, "a");
    let b = String::from_str(&env, "b");
    let c = String::from_str(&env, "c");

    client.add(&admin, &a);
    client.add(&admin, &b);
    client.add(&admin, &c);
    client.remove(&admin, &b);

    assert_eq!(client.list(), vec![&env, a, c]);
}

#[test]
fn an_entry_of_exactly_the_maximum_length_is_accepted() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let entry = entry_of_len(&env, MAX_ENTRY_LEN as usize);

    client.add(&admin, &entry);

    assert!(client.contains(&entry));
    assert_eq!(client.count(), 1);
}

#[test]
fn removing_the_only_entry_leaves_an_empty_list() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let entry = String::from_str(&env, "lab-a");

    client.add(&admin, &entry);
    client.remove(&admin, &entry);

    assert_eq!(client.count(), 0);
    assert_eq!(client.list(), Vec::new(&env));
}

#[test]
fn an_entry_can_be_added_again_after_removal() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let entry = String::from_str(&env, "lab-a");

    client.add(&admin, &entry);
    client.remove(&admin, &entry);
    client.add(&admin, &entry);

    assert!(client.contains(&entry));
    assert_eq!(client.count(), 1);
}

/// Entries are compared byte-for-byte. Normalising case would be a policy
/// decision, and the wrong one for anything like an asset code, so the contract
/// leaves it to the caller.
#[test]
fn entries_are_case_sensitive() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let lower = String::from_str(&env, "lab");
    let upper = String::from_str(&env, "LAB");

    client.add(&admin, &lower);

    assert!(client.contains(&lower));
    assert!(!client.contains(&upper));

    // Adding the differently-cased spelling is a genuinely new entry.
    client.add(&admin, &upper);
    assert_eq!(client.count(), 2);
}

/// Whitespace is not trimmed either: an entry is exactly the bytes given.
#[test]
fn entries_are_not_trimmed() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let padded = String::from_str(&env, " lab ");

    client.add(&admin, &padded);

    assert!(client.contains(&padded));
    assert!(!client.contains(&String::from_str(&env, "lab")));
}

/// The list holds its maximum exactly, and a removal makes room again.
#[test]
fn the_list_holds_exactly_the_maximum() {
    use string_allowlist::MAX_ENTRIES;

    let env = Env::default();
    let (client, admin) = setup(&env);

    let mut i = 0u32;
    while i < MAX_ENTRIES {
        client.add(&admin, &String::from_str(&env, &format!("entry-{i:03}")));
        i += 1;
    }

    assert_eq!(client.count(), MAX_ENTRIES);

    // One removal frees exactly one slot.
    client.remove(&admin, &String::from_str(&env, "entry-000"));
    client.add(&admin, &String::from_str(&env, "entry-new"));

    assert_eq!(client.count(), MAX_ENTRIES);
    assert!(client.contains(&String::from_str(&env, "entry-new")));
}
