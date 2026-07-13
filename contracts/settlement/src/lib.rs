#![no_std]

use soroban_sdk::{
    contract, contractclient, contractimpl, contracttype, symbol_short, Address, Env, Vec,
};

const BPS_DENOMINATOR: i128 = 10_000;
const PRICE_SCALE: i128 = 10_000_000;
const ADMIN_TIMELOCK_SECONDS: u64 = 86_400;

#[derive(Clone)]
#[contracttype]
pub struct Asset {
    pub address: Address,
}

#[derive(Clone)]
#[contracttype]
pub struct Price {
    pub asset: Address,
    pub price_e7: i128,
    pub updated_at: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct TimelockedU32 {
    pub value: u32,
    pub execute_after: u64,
}

#[contractclient(name = "OracleAdapterClient")]
pub trait OracleAdapter {
    fn price(env: Env, asset: Address) -> Price;
}

#[contractclient(name = "SoroswapRouterClient")]
pub trait SoroswapRouter {
    fn swap_exact_tokens_for_tokens(
        env: Env,
        amount_in: i128,
        amount_out_min: i128,
        path: Vec<Address>,
        to: Address,
        deadline: u64,
    ) -> Vec<i128>;
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Initialized,
    MaxSlippageBps,
    Oracle,
    PendingMaxSlippageBps,
    Router,
    SimulatedSlippageBps,
}

#[contract]
pub struct Settlement;

#[contractimpl]
impl Settlement {
    pub fn initialize(
        env: Env,
        admin: Address,
        router: Address,
        oracle: Address,
        max_slippage_bps: u32,
    ) {
        if env.storage().instance().has(&DataKey::Initialized) {
            panic!("settlement already initialized");
        }
        if max_slippage_bps > 10_000 {
            panic!("slippage cap exceeds 100 percent");
        }

        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Router, &router);
        env.storage().instance().set(&DataKey::Oracle, &oracle);
        env.storage()
            .instance()
            .set(&DataKey::MaxSlippageBps, &max_slippage_bps);
        env.storage()
            .instance()
            .set(&DataKey::SimulatedSlippageBps, &0_u32);
        env.storage().instance().set(&DataKey::Initialized, &true);
    }

    pub fn set_simulated_slippage_bps(env: Env, admin: Address, slippage_bps: u32) {
        if slippage_bps > 10_000 {
            panic!("slippage exceeds 100 percent");
        }
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("only settlement admin can configure simulation");
        }
        admin.require_auth();
        env.storage()
            .instance()
            .set(&DataKey::SimulatedSlippageBps, &slippage_bps);
    }

    pub fn schedule_max_slippage_bps(env: Env, admin: Address, max_slippage_bps: u32) -> u64 {
        if max_slippage_bps > 10_000 {
            panic!("slippage cap exceeds 100 percent");
        }
        require_admin(&env, &admin);
        let execute_after = timelock_ready_at(&env);
        env.storage().instance().set(
            &DataKey::PendingMaxSlippageBps,
            &TimelockedU32 {
                value: max_slippage_bps,
                execute_after,
            },
        );
        execute_after
    }

    pub fn execute_max_slippage_bps(env: Env, admin: Address) {
        require_admin(&env, &admin);
        let pending: TimelockedU32 = env
            .storage()
            .instance()
            .get(&DataKey::PendingMaxSlippageBps)
            .unwrap();
        if env.ledger().timestamp() < pending.execute_after {
            panic!("settlement admin timelock not ready");
        }
        env.storage()
            .instance()
            .set(&DataKey::MaxSlippageBps, &pending.value);
        env.storage()
            .instance()
            .remove(&DataKey::PendingMaxSlippageBps);
    }

    pub fn swap(
        env: Env,
        caller: Address,
        amount_in: i128,
        expected_amount_out: i128,
        slippage_bps: u32,
        path: Vec<Address>,
        to: Address,
        deadline: u64,
    ) -> Vec<i128> {
        caller.require_auth();
        if amount_in <= 0 || expected_amount_out <= 0 {
            panic!("basket swap amounts must be positive");
        }
        if slippage_bps > read_slippage_cap(&env) {
            panic!("basket swap exceeds slippage cap");
        }

        let amount_out_min = expected_amount_out
            .checked_mul((10_000 - slippage_bps) as i128)
            .unwrap()
            / 10_000;
        let router: Address = env.storage().instance().get(&DataKey::Router).unwrap();
        let amounts = SoroswapRouterClient::new(&env, &router).swap_exact_tokens_for_tokens(
            &amount_in,
            &amount_out_min,
            &path,
            &to,
            &deadline,
        );
        if path.len() >= 2 && !amounts.is_empty() {
            let input = path.get_unchecked(0);
            let output = path.get_unchecked(path.len() - 1);
            enforce_slippage(&env, &input, &output, amount_in, amounts.last_unchecked());
        }

        env.events().publish(
            (symbol_short!("swap"), caller, to),
            (amount_in, amount_out_min),
        );
        amounts
    }

    pub fn max_slippage_bps(env: Env) -> u32 {
        read_slippage_cap(&env)
    }

    pub fn invest(
        env: Env,
        basket: Address,
        deposit_asset: Address,
        amount: i128,
        assets: Vec<Asset>,
        target_weights_bps: Vec<u32>,
    ) -> Vec<i128> {
        basket.require_auth();
        if amount <= 0 {
            panic!("basket invest amount must be positive");
        }
        validate_weights(&assets, &target_weights_bps);

        let mut acquired = Vec::new(&env);
        for i in 0..assets.len() {
            let asset = assets.get_unchecked(i).address;
            let target_input = amount
                .checked_mul(target_weights_bps.get_unchecked(i) as i128)
                .unwrap()
                / BPS_DENOMINATOR;
            let expected_out = oracle_quote(&env, &deposit_asset, &asset, target_input);
            let realized = apply_simulated_slippage(&env, expected_out);
            enforce_slippage(&env, &deposit_asset, &asset, target_input, realized);
            acquired.push_back(realized);
        }

        env.events()
            .publish((symbol_short!("invest"), basket), amount);
        acquired
    }

    pub fn redeem(
        env: Env,
        basket: Address,
        payout_asset: Address,
        assets: Vec<Asset>,
        amounts: Vec<i128>,
    ) -> i128 {
        basket.require_auth();
        if assets.len() != amounts.len() {
            panic!("basket redeem assets and amounts mismatch");
        }

        let mut total = 0_i128;
        for i in 0..assets.len() {
            let asset = assets.get_unchecked(i).address;
            let amount = amounts.get_unchecked(i);
            if amount < 0 {
                panic!("basket redeem amount must not be negative");
            }
            let expected_out = oracle_quote(&env, &asset, &payout_asset, amount);
            let realized = apply_simulated_slippage(&env, expected_out);
            enforce_slippage(&env, &asset, &payout_asset, amount, realized);
            total = total.checked_add(realized).unwrap();
        }

        env.events()
            .publish((symbol_short!("redeem"), basket), total);
        total
    }

    pub fn rebalance(
        env: Env,
        basket: Address,
        base_asset: Address,
        assets: Vec<Asset>,
        old_holdings: Vec<i128>,
        new_holdings: Vec<i128>,
    ) -> Vec<i128> {
        basket.require_auth();
        if assets.len() != old_holdings.len() || assets.len() != new_holdings.len() {
            panic!("basket rebalance assets and holdings mismatch");
        }

        for i in 0..assets.len() {
            let asset = assets.get_unchecked(i).address;
            let old_amount = old_holdings.get_unchecked(i);
            let new_amount = new_holdings.get_unchecked(i);
            if old_amount < 0 || new_amount < 0 {
                panic!("basket rebalance holding must not be negative");
            }
            if new_amount > old_amount {
                let buy_amount = new_amount - old_amount;
                let base_in = oracle_quote(&env, &asset, &base_asset, buy_amount);
                let realized = apply_simulated_slippage(&env, buy_amount);
                enforce_slippage(&env, &base_asset, &asset, base_in, realized);
            } else if old_amount > new_amount {
                let sell_amount = old_amount - new_amount;
                let expected_base_out = oracle_quote(&env, &asset, &base_asset, sell_amount);
                let realized = apply_simulated_slippage(&env, expected_base_out);
                enforce_slippage(&env, &asset, &base_asset, sell_amount, realized);
            }
        }

        env.events()
            .publish((symbol_short!("rebalance"), basket), new_holdings.clone());
        new_holdings
    }
}

fn read_slippage_cap(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::MaxSlippageBps)
        .unwrap()
}

fn require_admin(env: &Env, admin: &Address) {
    let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    if *admin != stored_admin {
        panic!("only settlement admin can perform admin action");
    }
    admin.require_auth();
}

fn timelock_ready_at(env: &Env) -> u64 {
    env.ledger()
        .timestamp()
        .checked_add(ADMIN_TIMELOCK_SECONDS)
        .unwrap()
}

fn read_oracle(env: &Env) -> Address {
    env.storage().instance().get(&DataKey::Oracle).unwrap()
}

fn read_simulated_slippage_bps(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::SimulatedSlippageBps)
        .unwrap_or(0)
}

fn oracle_quote(env: &Env, input: &Address, output: &Address, amount_in: i128) -> i128 {
    if amount_in == 0 || input == output {
        return amount_in;
    }
    let oracle = read_oracle(env);
    let client = OracleAdapterClient::new(env, &oracle);
    let input_price = client.price(input).price_e7;
    let output_price = client.price(output).price_e7;
    amount_in.checked_mul(input_price).unwrap() / output_price
}

fn apply_simulated_slippage(env: &Env, expected_out: i128) -> i128 {
    let slippage = read_simulated_slippage_bps(env) as i128;
    expected_out
        .checked_mul(BPS_DENOMINATOR - slippage)
        .unwrap()
        / BPS_DENOMINATOR
}

fn enforce_slippage(
    env: &Env,
    input: &Address,
    output: &Address,
    amount_in: i128,
    realized_amount_out: i128,
) {
    if amount_in == 0 && realized_amount_out == 0 {
        return;
    }
    if amount_in <= 0 || realized_amount_out < 0 {
        panic!("invalid swap amounts");
    }
    let expected = oracle_quote(env, input, output, amount_in);
    let min_out = expected
        .checked_mul(BPS_DENOMINATOR - read_slippage_cap(env) as i128)
        .unwrap()
        / BPS_DENOMINATOR;
    if realized_amount_out < min_out {
        panic!("swap slippage exceeds oracle tolerance");
    }
    let _ = PRICE_SCALE;
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

#[cfg(test)]
mod test {
    extern crate std;

    use super::*;
    use oracle_adapter::OracleAdapter;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        Env,
    };

    fn setup() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let router = Address::generate(&env);
        let oracle = env.register(OracleAdapter, ());
        let settlement = env.register(Settlement, ());
        let client = SettlementClient::new(&env, &settlement);
        client.initialize(&admin, &router, &oracle, &500_u32);
        (env, admin, settlement)
    }

    #[test]
    #[should_panic]
    fn slippage_cap_change_cannot_execute_before_timelock() {
        let (env, admin, settlement) = setup();
        let client = SettlementClient::new(&env, &settlement);

        client.schedule_max_slippage_bps(&admin, &250_u32);
        client.execute_max_slippage_bps(&admin);
    }

    #[test]
    fn slippage_cap_change_executes_after_timelock() {
        let (env, admin, settlement) = setup();
        let client = SettlementClient::new(&env, &settlement);

        let ready_at = client.schedule_max_slippage_bps(&admin, &250_u32);
        env.ledger().set_timestamp(ready_at);
        client.execute_max_slippage_bps(&admin);

        assert_eq!(client.max_slippage_bps(), 250_u32);
    }
}
