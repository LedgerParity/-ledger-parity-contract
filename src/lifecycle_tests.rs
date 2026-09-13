use super::*;
use soroban_sdk::testutils::{
    storage::Instance as _, storage::Persistent as _, Address as _, Ledger,
};

fn setup(max_ttl: u32) -> (Env, Address, Address, Bytes) {
    let env = Env::default();
    env.ledger().with_mut(|ledger| {
        ledger.sequence_number = 100;
        ledger.min_persistent_entry_ttl = 100;
        ledger.max_entry_ttl = max_ttl;
    });
    let id = env.register_contract(None, VerifyContract);
    let owner = Address::generate(&env);
    let hash = Bytes::from_array(&env, &[9; 32]);
    env.mock_all_auths();
    VerifyContractClient::new(&env, &id).store(
        &hash,
        &owner,
        &Bytes::from_slice(&env, b"original"),
    );
    (env, id, owner, hash)
}

#[test]
fn records_have_separate_persistent_entries() {
    let (env, id, owner, hash) = setup(TTL_TARGET * 2);
    let client = VerifyContractClient::new(&env, &id);
    let second = Bytes::from_array(&env, &[10; 32]);
    client.store(&second, &owner, &Bytes::new(&env));
    env.as_contract(&id, || {
        assert_eq!(env.storage().persistent().all().len(), 2);
        assert_eq!(env.storage().instance().all().len(), 0);
        assert_eq!(env.storage().persistent().get_ttl(&hash), TTL_TARGET);
        assert_eq!(env.storage().persistent().get_ttl(&second), TTL_TARGET);
        assert_eq!(env.storage().instance().get_ttl(), TTL_TARGET);
    });
}

#[test]
fn permissionless_renewal_preserves_record_and_only_extends_selected_key() {
    let (env, id, owner, hash) = setup(TTL_TARGET * 2);
    let client = VerifyContractClient::new(&env, &id);
    let second = Bytes::from_array(&env, &[10; 32]);
    client.store(&second, &owner, &Bytes::new(&env));
    let original = client.get_info(&hash);
    env.set_auths(&[]);
    env.ledger()
        .with_mut(|ledger| ledger.sequence_number += TTL_TARGET / 2 + 1);
    assert!(client.renew(&hash));
    assert!(env.auths().is_empty());
    let renewed = client.get_info(&hash);
    assert_eq!(renewed.owner, original.owner);
    assert_eq!(renewed.timestamp, original.timestamp);
    assert_eq!(renewed.metadata, original.metadata);
    env.as_contract(&id, || {
        assert_eq!(env.storage().persistent().get_ttl(&hash), TTL_TARGET);
        assert_eq!(
            env.storage().persistent().get_ttl(&second),
            TTL_TARGET / 2 - 1
        );
        assert_eq!(env.storage().instance().get_ttl(), TTL_TARGET);
    });
}

#[test]
fn reads_and_early_renewal_do_not_bump_ttl() {
    let (env, id, owner, hash) = setup(TTL_TARGET * 2);
    let client = VerifyContractClient::new(&env, &id);
    env.ledger().with_mut(|ledger| ledger.sequence_number += 10);
    assert!(client.renew(&hash));
    env.ledger()
        .with_mut(|ledger| ledger.sequence_number += TTL_TARGET / 2);
    assert!(client.verify(&hash));
    assert!(client.is_owner(&hash, &owner));
    client.get_info(&hash);
    env.as_contract(&id, || {
        assert_eq!(
            env.storage().persistent().get_ttl(&hash),
            TTL_TARGET / 2 - 10
        );
        assert_eq!(env.storage().instance().get_ttl(), TTL_TARGET / 2 - 10);
    });
}

#[test]
fn ttl_target_respects_lower_network_limit() {
    let (env, id, _, hash) = setup(1000);
    let client = VerifyContractClient::new(&env, &id);
    env.as_contract(&id, || {
        assert_eq!(
            env.storage().persistent().get_ttl(&hash),
            env.storage().max_ttl()
        );
        assert_eq!(env.storage().instance().get_ttl(), env.storage().max_ttl());
    });
    env.ledger()
        .with_mut(|ledger| ledger.sequence_number += 600);
    assert!(client.renew(&hash));
    env.as_contract(&id, || {
        assert_eq!(
            env.storage().persistent().get_ttl(&hash),
            env.storage().max_ttl()
        );
        assert_eq!(env.storage().instance().get_ttl(), env.storage().max_ttl());
    });
}

#[test]
fn absent_or_invalid_renewal_does_not_create_records() {
    let (env, id, _, _) = setup(TTL_TARGET * 2);
    let client = VerifyContractClient::new(&env, &id);
    let missing = Bytes::from_array(&env, &[11; 32]);
    assert!(!client.renew(&missing));
    assert!(client.try_renew(&Bytes::new(&env)).is_err());
    assert!(!client.verify(&missing));
    env.as_contract(&id, || {
        assert_eq!(env.storage().persistent().all().len(), 1)
    });
}

#[test]
fn archived_record_cannot_be_read_as_absent_or_overwritten() {
    let (env, id, owner, hash) = setup(TTL_TARGET * 3);
    let client = VerifyContractClient::new(&env, &id);
    // Keep the contract live so this exercises record archival specifically.
    env.as_contract(&id, || {
        env.storage()
            .instance()
            .extend_ttl(TTL_TARGET * 2, TTL_TARGET * 2)
    });
    env.ledger()
        .with_mut(|ledger| ledger.sequence_number += TTL_TARGET + 1);
    assert!(client.try_verify(&hash).is_err());
    assert!(client.try_get_info(&hash).is_err());
    assert!(client.try_is_owner(&hash, &owner).is_err());
    assert!(client.try_renew(&hash).is_err());
    assert!(client.try_store(&hash, &owner, &Bytes::new(&env)).is_err());
}
