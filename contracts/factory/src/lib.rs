#![no_std]

use soroban_sdk::{
    contract, contractclient, contractimpl, contracttype, symbol_short, Address, BytesN, Env,
    String, Vec,
};

#[derive(Clone)]
#[contracttype]
pub struct Asset {
    pub address: Address,
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

#[contractclient(name = "BasketClient")]
pub trait BasketContract {
    fn initialize(
        env: Env,
        admin: Address,
        name: String,
        assets: Vec<Asset>,
        target_weights_bps: Vec<u32>,
        share_token: Address,
        settlement: Address,
        oracle: Address,
        deposit_asset: Address,
        withdrawal_fee_bps: u32,
        rebalancers: Vec<Address>,
        rebalancer_threshold: u32,
        max_drift_bps: u32,
        max_transaction_amount: i128,
    );
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
    pub fn initialize(
        env: Env,
        admin: Address,
        basket_wasm_hash: BytesN<32>,
        token_wasm_hash: BytesN<32>,
        settlement: Address,
        oracle: Address,
        deposit_asset: Address,
        withdrawal_fee_bps: u32,
        token_decimals: u32,
        rebalancers: Vec<Address>,
        rebalancer_threshold: u32,
        max_drift_bps: u32,
        max_transaction_amount: i128,
    ) {
        if env.storage().instance().has(&DataKey::Initialized) {
            panic!("factory already initialized");
        }
        if withdrawal_fee_bps > 10_000 {
            panic!("basket withdrawal fee exceeds 100 percent");
        }
        if max_drift_bps > 10_000 {
            panic!("basket max drift exceeds 100 percent");
        }
        if max_transaction_amount < 0 {
            panic!("basket max transaction amount must not be negative");
        }
        validate_rebalancer_quorum(&rebalancers, rebalancer_threshold);
        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::BasketWasmHash, &basket_wasm_hash);
        env.storage()
            .instance()
            .set(&DataKey::TokenWasmHash, &token_wasm_hash);
        env.storage()
            .instance()
            .set(&DataKey::Settlement, &settlement);
        env.storage().instance().set(&DataKey::Oracle, &oracle);
        env.storage()
            .instance()
            .set(&DataKey::DepositAsset, &deposit_asset);
        env.storage()
            .instance()
            .set(&DataKey::WithdrawalFeeBps, &withdrawal_fee_bps);
        env.storage()
            .instance()
            .set(&DataKey::TokenDecimals, &token_decimals);
        env.storage()
            .instance()
            .set(&DataKey::Rebalancers, &rebalancers);
        env.storage()
            .instance()
            .set(&DataKey::RebalancerThreshold, &rebalancer_threshold);
        env.storage()
            .instance()
            .set(&DataKey::MaxDriftBps, &max_drift_bps);
        env.storage()
            .instance()
            .set(&DataKey::MaxTransactionAmount, &max_transaction_amount);
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
        validate_weights(&assets, &target_weights_bps);

        let id = read_count(&env);
        let basket_wasm_hash: BytesN<32> = env
            .storage()
            .instance()
            .get(&DataKey::BasketWasmHash)
            .unwrap();
        let token_wasm_hash: BytesN<32> = env
            .storage()
            .instance()
            .get(&DataKey::TokenWasmHash)
            .unwrap();

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

        BasketClient::new(&env, &basket).initialize(
            &creator,
            &name,
            &assets,
            &target_weights_bps,
            &basket_token,
            &read_settlement(&env),
            &read_oracle(&env),
            &read_deposit_asset(&env),
            &read_withdrawal_fee_bps(&env),
            &read_rebalancers(&env),
            &read_rebalancer_threshold(&env),
            &read_max_drift_bps(&env),
            &read_max_transaction_amount(&env),
        );

        let spec = BasketSpec {
            creator: creator.clone(),
            name,
            assets,
            target_weights_bps,
            basket: basket.clone(),
            basket_token: basket_token.clone(),
        };
        env.storage().persistent().set(&DataKey::Basket(id), &spec);
        env.storage()
            .persistent()
            .set(&DataKey::BasketCount, &(id + 1));

        let mut creator_baskets = Self::baskets_by_creator(env.clone(), creator.clone());
        creator_baskets.push_back(basket.clone());
        env.storage()
            .persistent()
            .set(&DataKey::CreatorBaskets(creator.clone()), &creator_baskets);

        env.events().publish(
            (symbol_short!("basket"), creator),
            (basket.clone(), basket_token),
        );
        basket
    }

    pub fn basket_count(env: Env) -> u32 {
        read_count(&env)
    }

    pub fn basket(env: Env, id: u32) -> BasketSpec {
        env.storage()
            .persistent()
            .get(&DataKey::Basket(id))
            .unwrap()
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
    env.storage().instance().get(&DataKey::Settlement).unwrap()
}

fn read_oracle(env: &Env) -> Address {
    env.storage().instance().get(&DataKey::Oracle).unwrap()
}

fn read_deposit_asset(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::DepositAsset)
        .unwrap()
}

fn read_withdrawal_fee_bps(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::WithdrawalFeeBps)
        .unwrap()
}

fn read_token_decimals(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::TokenDecimals)
        .unwrap()
}

fn read_rebalancers(env: &Env) -> Vec<Address> {
    env.storage().instance().get(&DataKey::Rebalancers).unwrap()
}

fn read_rebalancer_threshold(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::RebalancerThreshold)
        .unwrap()
}

fn read_max_drift_bps(env: &Env) -> u32 {
    env.storage().instance().get(&DataKey::MaxDriftBps).unwrap()
}

fn read_max_transaction_amount(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::MaxTransactionAmount)
        .unwrap_or(0)
}

fn validate_weights(assets: &Vec<Asset>, weights: &Vec<u32>) {
    if assets.is_empty() || assets.len() != weights.len() {
        panic!("basket assets and weights mismatch");
    }

    let mut total = 0u32;
    for weight in weights.iter() {
        total = total.checked_add(weight).unwrap();
    }
    if total != 10_000 {
        panic!("basket weights must total 10000 bps");
    }
}

fn validate_rebalancer_quorum(rebalancers: &Vec<Address>, threshold: u32) {
    if threshold > rebalancers.len() {
        panic!("invalid rebalancer quorum");
    }
    assert_unique(rebalancers);
}

fn assert_unique(addresses: &Vec<Address>) {
    for i in 0..addresses.len() {
        let left = addresses.get_unchecked(i);
        for j in (i + 1)..addresses.len() {
            if left == addresses.get_unchecked(j) {
                panic!("duplicate address");
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
