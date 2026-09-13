#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Bytes, Env, Symbol};

const STORED: Symbol = symbol_short!("STORED");

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
        if env.storage().instance().has(&hash) {
            panic!("report already stored");
        }
        let info = ReportInfo {
            owner: owner.clone(),
            timestamp: env.ledger().timestamp(),
            metadata,
        };
        env.storage().instance().set(&hash, &info);
        env.events().publish((STORED, owner), hash);
    }

    /// Verify a report hash exists. Returns true if stored.
    pub fn verify(env: Env, hash: Bytes) -> bool {
        env.storage().instance().has(&hash)
    }

    /// Get info about a stored report. Panics if not found.
    pub fn get_info(env: Env, hash: Bytes) -> ReportInfo {
        env.storage()
            .instance()
            .get(&hash)
            .expect("report not found")
    }

    /// Check if a hash was stored by a specific owner.
    pub fn is_owner(env: Env, hash: Bytes, owner: Address) -> bool {
        match env.storage().instance().get::<_, ReportInfo>(&hash) {
            Some(info) => info.owner == owner,
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_store_and_verify() {
        let env = Env::default();
        let contract = VerifyContractClient::new(&env);
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
        let contract = VerifyContractClient::new(&env);
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
        let contract = VerifyContractClient::new(&env);
        let owner = Address::generate(&env);
        let other = Address::generate(&env);
        let hash = Bytes::from_array(&env, &[1u8; 32]);

        contract.store(&hash, &owner, &Bytes::new(&env));
        assert!(contract.is_owner(&hash, &owner));
        assert!(!contract.is_owner(&hash, &other));
    }

    #[test]
    #[should_panic(expected = "report already stored")]
    fn test_duplicate_store_panics() {
        let env = Env::default();
        let contract = VerifyContractClient::new(&env);
        let owner = Address::generate(&env);
        let hash = Bytes::from_array(&env, &[1u8; 32]);

        contract.store(&hash, &owner, &Bytes::new(&env));
        contract.store(&hash, &owner, &Bytes::new(&env));
    }
}
