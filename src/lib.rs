#![no_std]
#[cfg(test)]
extern crate std;
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Bytes, Env, Symbol, Vec,
};

const STORED: Symbol = symbol_short!("STORED");
// Ledger count, not a wall-clock guarantee (about 30 days at 5 seconds/ledger).
const TTL_TARGET: u32 = 518_400;
const MAX_BATCH_SIZE: u32 = 10;

#[contracttype]
#[derive(Clone)]
pub struct ReportInput {
    pub hash: Bytes,
    pub metadata: Bytes,
}

fn extend_contract_lifetime(env: &Env) {
    let target = TTL_TARGET.min(env.storage().max_ttl());
    let threshold = target / 2;
    env.storage().instance().extend_ttl(threshold, target);
}

fn extend_lifetime(env: &Env, hash: &Bytes) {
    let target = TTL_TARGET.min(env.storage().max_ttl());
    let threshold = target / 2;
    env.storage()
        .persistent()
        .extend_ttl(hash, threshold, target);
    extend_contract_lifetime(env);
}

#[contracttype]
pub struct ReportInfo {
    pub owner: Address,
    pub timestamp: u64,
    pub metadata: Bytes,
}

#[contract]
pub struct VerifyContract;

#[contractimpl]
impl VerifyContract {
    /// Store a report hash with owner and optional metadata.
    pub fn store(env: Env, hash: Bytes, owner: Address, metadata: Bytes) {
        owner.require_auth();
        assert!(hash.len() == 32, "hash must be 32 bytes");
        assert!(metadata.len() <= 1024, "metadata exceeds 1024 bytes");
        if env.storage().persistent().has(&hash) {
            panic!("report already stored");
        }
        let info = ReportInfo {
            owner: owner.clone(),
            timestamp: env.ledger().timestamp(),
            metadata,
        };
        env.storage().persistent().set(&hash, &info);
        extend_lifetime(&env, &hash);
        env.events().publish((STORED, owner), hash);
    }

    /// Store up to ten reports atomically under one owner's authorization.
    /// Every hash and metadata value is validated before any record is written.
    pub fn store_batch(env: Env, owner: Address, reports: Vec<ReportInput>) {
        owner.require_auth();
        let count = reports.len();
        assert!(
            count > 0 && count <= MAX_BATCH_SIZE,
            "batch must contain 1 to 10 reports"
        );

        let mut seen = Vec::new(&env);
        for report in reports.iter() {
            assert!(report.hash.len() == 32, "hash must be 32 bytes");
            assert!(report.metadata.len() <= 1024, "metadata exceeds 1024 bytes");
            assert!(
                !env.storage().persistent().has(&report.hash),
                "report already stored"
            );
            for prior in seen.iter() {
                assert!(prior != report.hash, "duplicate hash in batch");
            }
            seen.push_back(report.hash);
        }

        let timestamp = env.ledger().timestamp();
        for report in reports.iter() {
            let info = ReportInfo {
                owner: owner.clone(),
                timestamp,
                metadata: report.metadata,
            };
            env.storage().persistent().set(&report.hash, &info);
            let target = TTL_TARGET.min(env.storage().max_ttl());
            env.storage()
                .persistent()
                .extend_ttl(&report.hash, target / 2, target);
            env.events().publish((STORED, owner.clone()), report.hash);
        }
        // Shared contract instance/code lifetime needs extending only once per call.
        extend_contract_lifetime(&env);
    }

    /// Verify a report hash exists. Returns true if stored.
    pub fn verify(env: Env, hash: Bytes) -> bool {
        env.storage().persistent().has(&hash)
    }

    /// Anyone can pay to retain an existing record; no contents are changed.
    /// False means absent, not archived. Archived entries must be restored first.
    pub fn renew(env: Env, hash: Bytes) -> bool {
        assert!(hash.len() == 32, "hash must be 32 bytes");
        if !env.storage().persistent().has(&hash) {
            return false;
        }
        extend_lifetime(&env, &hash);
        true
    }

    /// Get info about a stored report. Panics if not found.
    pub fn get_info(env: Env, hash: Bytes) -> ReportInfo {
        env.storage()
            .persistent()
            .get(&hash)
            .expect("report not found")
    }

    /// Check if a hash was stored by a specific owner.
    pub fn is_owner(env: Env, hash: Bytes, owner: Address) -> bool {
        match env.storage().persistent().get::<_, ReportInfo>(&hash) {
            Some(info) => info.owner == owner,
            None => false,
        }
    }
}

#[cfg(test)]
mod lifecycle_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Ledger};
    use soroban_sdk::{IntoVal, Symbol};

    #[test]
    fn requires_owner_authorization() {
        let env = Env::default();
        let id = env.register_contract(None, VerifyContract);
        let client = VerifyContractClient::new(&env, &id);
        let owner = Address::generate(&env);
        let hash = Bytes::from_array(&env, &[3; 32]);
        let metadata = Bytes::new(&env);
        assert!(client.try_store(&hash, &owner, &metadata).is_err());
        assert!(!client.verify(&hash));
        env.mock_all_auths();
        client.store(&hash, &owner, &metadata);
        assert_eq!(
            env.auths(),
            std::vec![(
                owner.clone(),
                AuthorizedInvocation {
                    function: AuthorizedFunction::Contract((
                        id,
                        Symbol::new(&env, "store"),
                        (hash, owner, metadata).into_val(&env)
                    )),
                    sub_invocations: std::vec![],
                }
            )]
        );
    }

    #[test]
    fn rejects_invalid_input_without_storing() {
        let env = Env::default();
        let id = env.register_contract(None, VerifyContract);
        let client = VerifyContractClient::new(&env, &id);
        let owner = Address::generate(&env);
        env.mock_all_auths();
        for size in [0usize, 31, 33] {
            let hash = Bytes::from_slice(&env, &std::vec![0; size]);
            assert!(client.try_store(&hash, &owner, &Bytes::new(&env)).is_err());
            assert!(!client.verify(&hash));
        }
        let hash = Bytes::from_array(&env, &[4; 32]);
        assert!(client
            .try_store(&hash, &owner, &Bytes::from_slice(&env, &[0; 1025]))
            .is_err());
        assert!(!client.verify(&hash));
        client.store(&hash, &owner, &Bytes::from_slice(&env, &[0; 1024]));
        assert!(client.verify(&hash));
    }

    #[test]
    fn duplicate_cannot_replace_original_record() {
        let env = Env::default();
        let id = env.register_contract(None, VerifyContract);
        let client = VerifyContractClient::new(&env, &id);
        let owner = Address::generate(&env);
        let other = Address::generate(&env);
        let hash = Bytes::from_array(&env, &[5; 32]);
        env.mock_all_auths();
        env.ledger().with_mut(|ledger| ledger.timestamp = 1234);
        client.store(&hash, &owner, &Bytes::from_slice(&env, b"original"));
        assert!(client.try_store(&hash, &other, &Bytes::new(&env)).is_err());
        let info = client.get_info(&hash);
        assert_eq!(info.owner, owner);
        assert_eq!(info.timestamp, 1234);
        assert_eq!(info.metadata, Bytes::from_slice(&env, b"original"));
        assert!(!client.is_owner(&hash, &other));
        assert!(client
            .try_get_info(&Bytes::from_array(&env, &[6; 32]))
            .is_err());
    }

    #[test]
    fn test_store_and_verify() {
        let env = Env::default();
        let contract_id = env.register_contract(None, VerifyContract);
        let contract = VerifyContractClient::new(&env, &contract_id);
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let hash = Bytes::from_array(&env, &[1u8; 32]);
        let metadata = Bytes::from_array(&env, b"test-report");

        contract.store(&hash, &owner, &metadata);
        assert!(contract.verify(&hash));
        assert!(!contract.verify(&Bytes::from_array(&env, &[2u8; 32])));
    }

    #[test]
    fn test_get_info() {
        let env = Env::default();
        let contract_id = env.register_contract(None, VerifyContract);
        let contract = VerifyContractClient::new(&env, &contract_id);
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let hash = Bytes::from_array(&env, &[1u8; 32]);
        let metadata = Bytes::from_array(&env, b"v1");

        contract.store(&hash, &owner, &metadata);
        let info = contract.get_info(&hash);
        assert_eq!(info.owner, owner);
        assert_eq!(info.metadata, metadata);
    }

    #[test]
    fn batch_stores_each_report_under_one_owner_authorization() {
        let env = Env::default();
        let id = env.register_contract(None, VerifyContract);
        let client = VerifyContractClient::new(&env, &id);
        let owner = Address::generate(&env);
        let first = Bytes::from_array(&env, &[21; 32]);
        let second = Bytes::from_array(&env, &[22; 32]);
        let reports = Vec::from_array(
            &env,
            [
                ReportInput {
                    hash: first.clone(),
                    metadata: Bytes::from_slice(&env, b"first"),
                },
                ReportInput {
                    hash: second.clone(),
                    metadata: Bytes::from_slice(&env, b"second"),
                },
            ],
        );

        assert!(client.try_store_batch(&owner, &reports).is_err());
        assert!(!client.verify(&first));
        env.mock_all_auths();
        env.ledger().with_mut(|ledger| ledger.timestamp = 4567);
        client.store_batch(&owner, &reports);

        assert_eq!(env.auths().len(), 1);
        let first_info = client.get_info(&first);
        let second_info = client.get_info(&second);
        assert_eq!(first_info.owner, owner);
        assert_eq!(first_info.timestamp, 4567);
        assert_eq!(first_info.metadata, Bytes::from_slice(&env, b"first"));
        assert_eq!(second_info.owner, owner);
        assert_eq!(second_info.timestamp, 4567);
        assert_eq!(second_info.metadata, Bytes::from_slice(&env, b"second"));
    }

    #[test]
    fn invalid_batch_does_not_store_any_report() {
        let env = Env::default();
        let id = env.register_contract(None, VerifyContract);
        let client = VerifyContractClient::new(&env, &id);
        let owner = Address::generate(&env);
        let first = Bytes::from_array(&env, &[31; 32]);
        let already_stored = Bytes::from_array(&env, &[32; 32]);
        env.mock_all_auths();
        client.store(&already_stored, &owner, &Bytes::new(&env));

        let includes_existing = Vec::from_array(
            &env,
            [
                ReportInput {
                    hash: first.clone(),
                    metadata: Bytes::new(&env),
                },
                ReportInput {
                    hash: already_stored,
                    metadata: Bytes::new(&env),
                },
            ],
        );
        assert!(client.try_store_batch(&owner, &includes_existing).is_err());
        assert!(!client.verify(&first));

        let duplicate_in_batch = Vec::from_array(
            &env,
            [
                ReportInput {
                    hash: first.clone(),
                    metadata: Bytes::new(&env),
                },
                ReportInput {
                    hash: first.clone(),
                    metadata: Bytes::new(&env),
                },
            ],
        );
        assert!(client
            .try_store_batch(&owner, &duplicate_in_batch)
            .is_err());
        assert!(!client.verify(&first));

        let invalid_metadata = Vec::from_array(
            &env,
            [ReportInput {
                hash: first.clone(),
                metadata: Bytes::from_slice(&env, &[0; 1025]),
            }],
        );
        assert!(client.try_store_batch(&owner, &invalid_metadata).is_err());
        assert!(!client.verify(&first));
    }

    #[test]
    fn batch_size_must_be_between_one_and_ten() {
        let env = Env::default();
        let id = env.register_contract(None, VerifyContract);
        let client = VerifyContractClient::new(&env, &id);
        let owner = Address::generate(&env);
        let empty = Vec::<ReportInput>::new(&env);
        assert!(client.try_store_batch(&owner, &empty).is_err());

        let mut too_many = Vec::new(&env);
        for value in 40u8..51u8 {
            too_many.push_back(ReportInput {
                hash: Bytes::from_array(&env, &[value; 32]),
                metadata: Bytes::new(&env),
            });
        }
        assert!(client.try_store_batch(&owner, &too_many).is_err());
        assert_eq!(
            env.as_contract(&id, || env.storage().persistent().all().len()),
            0
        );

        let mut maximum = Vec::new(&env);
        for value in 60u8..70u8 {
            maximum.push_back(ReportInput {
                hash: Bytes::from_array(&env, &[value; 32]),
                metadata: Bytes::new(&env),
            });
        }
        env.mock_all_auths();
        client.store_batch(&owner, &maximum);
        assert_eq!(
            env.as_contract(&id, || env.storage().persistent().all().len()),
            10
        );
    }

    #[test]
    fn test_is_owner() {
        let env = Env::default();
        let contract_id = env.register_contract(None, VerifyContract);
        let contract = VerifyContractClient::new(&env, &contract_id);
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let other = Address::generate(&env);
        let hash = Bytes::from_array(&env, &[1u8; 32]);

        contract.store(&hash, &owner, &Bytes::new(&env));
        assert!(contract.is_owner(&hash, &owner));
        assert!(!contract.is_owner(&hash, &other));
    }

    #[test]
    #[should_panic]
    fn test_duplicate_store_panics() {
        let env = Env::default();
        let contract_id = env.register_contract(None, VerifyContract);
        let contract = VerifyContractClient::new(&env, &contract_id);
        env.mock_all_auths();
        let owner = Address::generate(&env);
        let hash = Bytes::from_array(&env, &[1u8; 32]);

        contract.store(&hash, &owner, &Bytes::new(&env));
        contract.store(&hash, &owner, &Bytes::new(&env));
    }
}
