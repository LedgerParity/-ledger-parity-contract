#![no_std]
#[cfg(test)]
extern crate std;
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Bytes, Env, Symbol,
};

const STORED: Symbol = symbol_short!("STORED");
// Ledger count, not a wall-clock guarantee (about 30 days at 5 seconds/ledger).
const TTL_TARGET: u32 = 518_400;

fn extend_lifetime(env: &Env, hash: &Bytes) {
    let target = TTL_TARGET.min(env.storage().max_ttl());
    let threshold = target / 2;
    env.storage()
        .persistent()
        .extend_ttl(hash, threshold, target);
    // The SDK extends both the instance and its Wasm code, independently.
    env.storage().instance().extend_ttl(threshold, target);
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
