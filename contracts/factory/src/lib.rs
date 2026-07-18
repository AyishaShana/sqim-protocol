#![no_std]

use soroban_sdk::{
    contract, contractclient, contracterror, contractevent, contractimpl, contracttype,
    panic_with_error, Address, BytesN, Env, String, Vec,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 3001,
    InvalidFee = 3002,
    InvalidDrift = 3003,
    InvalidTransactionLimit = 3004,
    InvalidWeights = 3005,
    InvalidQuorum = 3006,
    DuplicateAddress = 3007,
    NotInitialized = 3008,
    BasketNotFound = 3009,
    ArithmeticOverflow = 3010,
}

#[derive(Clone)]
#[contracttype]
pub struct Asset {
    pub address: Address,
}

#[derive(Clone)]
#[contracttype]
pub struct BasketConfig {
    pub admin: Address,
    pub name: String,
    pub assets: Vec<Asset>,
    pub target_weights_bps: Vec<u32>,
    pub share_token: Address,
    pub settlement: Address,
    pub oracle: Address,
    pub deposit_asset: Address,
    pub withdrawal_fee_bps: u32,
    pub rebalancers: Vec<Address>,
    pub rebalancer_threshold: u32,
    pub max_drift_bps: u32,
    pub max_transaction_amount: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct FactoryConfig {
    pub admin: Address,
    pub basket_wasm_hash: BytesN<32>,
    pub token_wasm_hash: BytesN<32>,
    pub settlement: Address,
    pub oracle: Address,
    pub deposit_asset: Address,
    pub withdrawal_fee_bps: u32,
    pub token_decimals: u32,
    pub rebalancers: Vec<Address>,
    pub rebalancer_threshold: u32,
    pub max_drift_bps: u32,
    pub max_transaction_amount: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct BasketSpec {
    pub creator: Address,
    pub name: String,
    pub assets: Vec<Asset>,
    pub target_weights_bps: Vec<u32>,
    pub basket: Address,
    pub basket_token: Address,
}

#[contractevent(topics = ["basket_created"], data_format = "vec")]
#[derive(Clone)]
pub struct BasketCreatedEvent {
    #[topic]
    pub creator: Address,
    pub basket: Address,
    pub basket_token: Address,
    pub name: String,
    pub assets: Vec<Asset>,
    pub target_weights_bps: Vec<u32>,
}

#[contractclient(name = "BasketClient")]
pub trait BasketContract {
    fn initialize(env: Env, config: BasketConfig);
}

#[contractclient(name = "BasketTokenClient")]
pub trait BasketTokenContract {
    fn initialize(env: Env, admin: Address, name: String, symbol: String, decimals: u32);
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Basket(u32),
    BasketCount,
    BasketWasmHash,
    CreatorBaskets(Address),
    DepositAsset,
    Initialized,
    Oracle,
    MaxDriftBps,
    MaxTransactionAmount,
    Rebalancers,
    RebalancerThreshold,
    Settlement,
    TokenDecimals,
    TokenWasmHash,
    WithdrawalFeeBps,
}

#[contract]
pub struct Factory;

#[contractimpl]
impl Factory {
    pub fn initialize(env: Env, config: FactoryConfig) {
        if env.storage().instance().has(&DataKey::Initialized) {
            panic_with_error!(&env, Error::AlreadyInitialized);
        }
        if config.withdrawal_fee_bps > 10_000 {
            panic_with_error!(&env, Error::InvalidFee);
        }
        if config.max_drift_bps > 10_000 {
            panic_with_error!(&env, Error::InvalidDrift);
        }
        if config.max_transaction_amount < 0 {
            panic_with_error!(&env, Error::InvalidTransactionLimit);
        }
        validate_rebalancer_quorum(&env, &config.rebalancers, config.rebalancer_threshold);
        config.admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &config.admin);
        env.storage()
            .instance()
            .set(&DataKey::BasketWasmHash, &config.basket_wasm_hash);
        env.storage()
            .instance()
            .set(&DataKey::TokenWasmHash, &config.token_wasm_hash);
        env.storage()
            .instance()
            .set(&DataKey::Settlement, &config.settlement);
        env.storage()
            .instance()
            .set(&DataKey::Oracle, &config.oracle);
        env.storage()
            .instance()
            .set(&DataKey::DepositAsset, &config.deposit_asset);
        env.storage()
            .instance()
            .set(&DataKey::WithdrawalFeeBps, &config.withdrawal_fee_bps);
        env.storage()
            .instance()
            .set(&DataKey::TokenDecimals, &config.token_decimals);
        env.storage()
            .instance()
            .set(&DataKey::Rebalancers, &config.rebalancers);
        env.storage()
            .instance()
            .set(&DataKey::RebalancerThreshold, &config.rebalancer_threshold);
        env.storage()
            .instance()
            .set(&DataKey::MaxDriftBps, &config.max_drift_bps);
        env.storage().instance().set(
            &DataKey::MaxTransactionAmount,
            &config.max_transaction_amount,
        );
        env.storage().instance().set(&DataKey::BasketCount, &0_u32);
        env.storage().instance().set(&DataKey::Initialized, &true);
    }

    pub fn create_basket(
        env: Env,
        creator: Address,
        name: String,
        assets: Vec<Asset>,
        target_weights_bps: Vec<u32>,
    ) -> Address {
        creator.require_auth();
        validate_weights(&env, &assets, &target_weights_bps);

        let id = read_count(&env);
        let basket_wasm_hash: BytesN<32> = env
            .storage()
            .instance()
            .get(&DataKey::BasketWasmHash)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        let token_wasm_hash: BytesN<32> = env
            .storage()
            .instance()
            .get(&DataKey::TokenWasmHash)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));

        let basket = env
            .deployer()
            .with_current_contract(salt(&env, id, 1))
            .deploy_v2(basket_wasm_hash, ());
        let basket_token = env
            .deployer()
            .with_current_contract(salt(&env, id, 2))
            .deploy_v2(token_wasm_hash, ());

        BasketTokenClient::new(&env, &basket_token).initialize(
            &basket,
            &name,
            &String::from_str(&env, "SQIMB"),
            &read_token_decimals(&env),
        );

        BasketClient::new(&env, &basket).initialize(&BasketConfig {
            admin: creator.clone(),
            name: name.clone(),
            assets: assets.clone(),
            target_weights_bps: target_weights_bps.clone(),
            share_token: basket_token.clone(),
            settlement: read_settlement(&env),
            oracle: read_oracle(&env),
            deposit_asset: read_deposit_asset(&env),
            withdrawal_fee_bps: read_withdrawal_fee_bps(&env),
            rebalancers: read_rebalancers(&env),
            rebalancer_threshold: read_rebalancer_threshold(&env),
            max_drift_bps: read_max_drift_bps(&env),
            max_transaction_amount: read_max_transaction_amount(&env),
        });

        let spec = BasketSpec {
            creator: creator.clone(),
            name: name.clone(),
            assets: assets.clone(),
            target_weights_bps: target_weights_bps.clone(),
            basket: basket.clone(),
            basket_token: basket_token.clone(),
        };
        env.storage().persistent().set(&DataKey::Basket(id), &spec);
        env.storage().persistent().set(
            &DataKey::BasketCount,
            &id.checked_add(1)
                .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow)),
        );

        let mut creator_baskets = Self::baskets_by_creator(env.clone(), creator.clone());
        creator_baskets.push_back(basket.clone());
        env.storage()
            .persistent()
            .set(&DataKey::CreatorBaskets(creator.clone()), &creator_baskets);

        BasketCreatedEvent {
            creator,
            basket: basket.clone(),
            basket_token,
            name,
            assets,
            target_weights_bps,
        }
        .publish(&env);
        basket
    }

    pub fn basket_count(env: Env) -> u32 {
        read_count(&env)
    }

    pub fn basket(env: Env, id: u32) -> BasketSpec {
        env.storage()
            .persistent()
            .get(&DataKey::Basket(id))
            .unwrap_or_else(|| panic_with_error!(&env, Error::BasketNotFound))
    }

    pub fn baskets_by_creator(env: Env, creator: Address) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::CreatorBaskets(creator))
            .unwrap_or(Vec::new(&env))
    }
}

fn read_count(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::BasketCount)
        .or_else(|| env.storage().instance().get(&DataKey::BasketCount))
        .unwrap_or(0)
}

fn read_settlement(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Settlement)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
}

fn read_oracle(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Oracle)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
}

fn read_deposit_asset(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::DepositAsset)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
}

fn read_withdrawal_fee_bps(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::WithdrawalFeeBps)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
}

fn read_token_decimals(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::TokenDecimals)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
}

fn read_rebalancers(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::Rebalancers)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
}

fn read_rebalancer_threshold(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::RebalancerThreshold)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
}

fn read_max_drift_bps(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::MaxDriftBps)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
}

fn read_max_transaction_amount(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::MaxTransactionAmount)
        .unwrap_or(0)
}

fn validate_weights(env: &Env, assets: &Vec<Asset>, weights: &Vec<u32>) {
    if assets.is_empty() || assets.len() != weights.len() {
        panic_with_error!(env, Error::InvalidWeights);
    }

    let mut total = 0u32;
    for weight in weights.iter() {
        total = total
            .checked_add(weight)
            .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow));
    }
    if total != 10_000 {
        panic_with_error!(env, Error::InvalidWeights);
    }
}

fn validate_rebalancer_quorum(env: &Env, rebalancers: &Vec<Address>, threshold: u32) {
    if threshold > rebalancers.len() {
        panic_with_error!(env, Error::InvalidQuorum);
    }
    assert_unique(env, rebalancers);
}

fn assert_unique(env: &Env, addresses: &Vec<Address>) {
    for i in 0..addresses.len() {
        let left = addresses.get_unchecked(i);
        for j in (i + 1)..addresses.len() {
            if left == addresses.get_unchecked(j) {
                panic_with_error!(env, Error::DuplicateAddress);
            }
        }
    }
}

fn salt(env: &Env, id: u32, tag: u8) -> BytesN<32> {
    let mut bytes = [0u8; 32];
    bytes[0] = tag;
    bytes[28] = (id >> 24) as u8;
    bytes[29] = (id >> 16) as u8;
    bytes[30] = (id >> 8) as u8;
    bytes[31] = id as u8;
    BytesN::from_array(env, &bytes)
}
