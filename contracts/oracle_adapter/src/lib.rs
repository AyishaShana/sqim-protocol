#![no_std]

use soroban_sdk::{
    contract, contractclient, contractimpl, contracttype, symbol_short, Address, Env, Symbol, Vec,
};

#[derive(Clone)]
#[contracttype]
pub struct Price {
    pub asset: Address,
    pub price_e7: i128,
    pub updated_at: u64,
}

#[derive(Clone)]
#[contracttype]
pub enum ReflectorAsset {
    Stellar(Address),
    Other(Symbol),
}

#[derive(Clone)]
#[contracttype]
pub struct ReflectorPriceData {
    pub price: i128,
    pub timestamp: u64,
}

#[contractclient(name = "ReflectorPulseClient")]
pub trait ReflectorPulse {
    fn decimals(env: Env) -> u32;
    fn lastprice(env: Env, asset: ReflectorAsset) -> Option<ReflectorPriceData>;
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    FallbackPrice(Address),
    FallbackSigners,
    FallbackThreshold,
    Initialized,
    MaxAgeSeconds,
    PrimaryAsset(Address),
    PrimaryEnabled,
    PrimaryOracle,
}

#[contract]
pub struct OracleAdapter;

#[contractimpl]
impl OracleAdapter {
    pub fn initialize(
        env: Env,
        admin: Address,
        primary_oracle: Address,
        primary_enabled: bool,
        max_age_seconds: u64,
        fallback_signers: Vec<Address>,
        fallback_threshold: u32,
    ) {
        if env.storage().instance().has(&DataKey::Initialized) {
            panic!("oracle adapter already initialized");
        }
        if max_age_seconds == 0 {
            panic!("oracle max age must be positive");
        }
        validate_quorum(&fallback_signers, fallback_threshold);

        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::PrimaryOracle, &primary_oracle);
        env.storage()
            .instance()
            .set(&DataKey::PrimaryEnabled, &primary_enabled);
        env.storage()
            .instance()
            .set(&DataKey::MaxAgeSeconds, &max_age_seconds);
        env.storage()
            .instance()
            .set(&DataKey::FallbackSigners, &fallback_signers);
        env.storage()
            .instance()
            .set(&DataKey::FallbackThreshold, &fallback_threshold);
        env.storage().instance().set(&DataKey::Initialized, &true);
    }

    pub fn set_primary_symbol(env: Env, admin: Address, asset: Address, symbol: Symbol) {
        require_admin(&env, &admin);
        env.storage().persistent().set(
            &DataKey::PrimaryAsset(asset),
            &ReflectorAsset::Other(symbol),
        );
    }

    pub fn set_primary_stellar(env: Env, admin: Address, asset: Address, oracle_asset: Address) {
        require_admin(&env, &admin);
        env.storage().persistent().set(
            &DataKey::PrimaryAsset(asset),
            &ReflectorAsset::Stellar(oracle_asset),
        );
    }

    pub fn set_primary_enabled(env: Env, admin: Address, enabled: bool) {
        require_admin(&env, &admin);
        env.storage()
            .instance()
            .set(&DataKey::PrimaryEnabled, &enabled);
    }

    pub fn set_fallback_price(
        env: Env,
        asset: Address,
        price_e7: i128,
        updated_at: u64,
        signers: Vec<Address>,
    ) {
        check_price(price_e7);
        let authorized = read_fallback_signers(&env);
        let threshold = read_fallback_threshold(&env);
        require_fallback_quorum(&authorized, threshold, &signers);

        let price = Price {
            asset: asset.clone(),
            price_e7,
            updated_at,
        };
        env.storage()
            .persistent()
            .set(&DataKey::FallbackPrice(asset.clone()), &price);
        env.events()
            .publish((symbol_short!("fallback"), asset), (price_e7, updated_at));
    }

    pub fn price(env: Env, asset: Address) -> Price {
        if let Some(price) = primary_price(&env, &asset) {
            return price;
        }
        if let Some(price) = fallback_price(&env, &asset) {
            return price;
        }
        panic!("oracle price unavailable or stale");
    }

    pub fn max_age_seconds(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::MaxAgeSeconds)
            .unwrap()
    }
}

fn primary_price(env: &Env, asset: &Address) -> Option<Price> {
    if !env
        .storage()
        .instance()
        .get(&DataKey::PrimaryEnabled)
        .unwrap_or(false)
    {
        return None;
    }
    let oracle: Address = env
        .storage()
        .instance()
        .get(&DataKey::PrimaryOracle)
        .unwrap();
    let reflector_asset = primary_asset(env, asset);
    let client = ReflectorPulseClient::new(env, &oracle);
    let data = client.lastprice(&reflector_asset)?;
    let decimals = client.decimals();
    let price_e7 = normalize_price(data.price, decimals);
    let price = Price {
        asset: asset.clone(),
        price_e7,
        updated_at: data.timestamp,
    };
    if is_fresh(env, price.updated_at) {
        Some(price)
    } else {
        None
    }
}

fn primary_asset(env: &Env, asset: &Address) -> ReflectorAsset {
    env.storage()
        .persistent()
        .get(&DataKey::PrimaryAsset(asset.clone()))
        .unwrap_or(ReflectorAsset::Stellar(asset.clone()))
}

fn require_admin(env: &Env, admin: &Address) {
    let stored: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    if stored != *admin {
        panic!("oracle adapter admin mismatch");
    }
    admin.require_auth();
}

fn fallback_price(env: &Env, asset: &Address) -> Option<Price> {
    let price: Price = env
        .storage()
        .persistent()
        .get(&DataKey::FallbackPrice(asset.clone()))?;
    if is_fresh(env, price.updated_at) {
        Some(price)
    } else {
        None
    }
}

fn normalize_price(price: i128, decimals: u32) -> i128 {
    check_price(price);
    if decimals == 7 {
        price
    } else if decimals < 7 {
        price.checked_mul(10_i128.pow(7 - decimals)).unwrap()
    } else {
        price / 10_i128.pow(decimals - 7)
    }
}

fn is_fresh(env: &Env, updated_at: u64) -> bool {
    let max_age: u64 = env
        .storage()
        .instance()
        .get(&DataKey::MaxAgeSeconds)
        .unwrap();
    let now = env.ledger().timestamp();
    updated_at <= now && now - updated_at <= max_age
}

fn validate_quorum(signers: &Vec<Address>, threshold: u32) {
    if threshold == 0 || threshold > signers.len() {
        panic!("invalid fallback oracle quorum");
    }
    assert_unique(signers);
}

fn read_fallback_signers(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::FallbackSigners)
        .unwrap()
}

fn read_fallback_threshold(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::FallbackThreshold)
        .unwrap()
}

fn require_fallback_quorum(authorized: &Vec<Address>, threshold: u32, signers: &Vec<Address>) {
    assert_unique(signers);
    let mut count = 0_u32;
    for signer in signers.iter() {
        if !contains(authorized, &signer) {
            panic!("unauthorized fallback oracle signer");
        }
        signer.require_auth();
        count += 1;
    }
    if count < threshold {
        panic!("fallback oracle quorum not met");
    }
}

fn contains(addresses: &Vec<Address>, needle: &Address) -> bool {
    for address in addresses.iter() {
        if address == *needle {
            return true;
        }
    }
    false
}

fn assert_unique(addresses: &Vec<Address>) {
    for i in 0..addresses.len() {
        let left = addresses.get_unchecked(i);
        for j in (i + 1)..addresses.len() {
            if left == addresses.get_unchecked(j) {
                panic!("duplicate signer");
            }
        }
    }
}

fn check_price(price_e7: i128) {
    if price_e7 <= 0 {
        panic!("basket price must be positive");
    }
}

#[cfg(test)]
mod test {
    extern crate std;

    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        vec, Env,
    };

    #[test]
    #[should_panic]
    fn stale_price_without_valid_fallback_quorum_fails() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1_000);

        let admin = Address::generate(&env);
        let signer_a = Address::generate(&env);
        let signer_b = Address::generate(&env);
        let asset = Address::generate(&env);
        let oracle_id = env.register(OracleAdapter, ());
        let client = OracleAdapterClient::new(&env, &oracle_id);

        client.initialize(
            &admin,
            &Address::generate(&env),
            &false,
            &60_u64,
            &vec![&env, signer_a.clone(), signer_b.clone()],
            &2_u32,
        );

        client.set_fallback_price(
            &asset,
            &10_000_000_i128,
            &900_u64,
            &vec![&env, signer_a, signer_b],
        );
        client.price(&asset);
    }
}
