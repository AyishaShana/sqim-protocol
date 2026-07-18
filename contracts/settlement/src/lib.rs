#![no_std]
#![allow(clippy::too_many_arguments)]

use soroban_sdk::{
    auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation},
    contract, contractclient, contracterror, contractevent, contractimpl, contracttype,
    panic_with_error, symbol_short,
    token::TokenClient,
    vec, Address, Env, IntoVal, MuxedAddress, Val, Vec,
};

const BPS_DENOMINATOR: i128 = 10_000;
const PRICE_SCALE: i128 = 10_000_000;
const ADMIN_TIMELOCK_SECONDS: u64 = 86_400;
const MIN_REBALANCE_SWAP_AMOUNT: i128 = 100;
const MAX_ROUTE_LENGTH: u32 = 4;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 4001,
    SlippageCapInvalid = 4002,
    NotInitialized = 4003,
    TimelockNotReady = 4004,
    Unauthorized = 4005,
    InvalidAmount = 4006,
    InvalidRoute = 4007,
    SlippageExceeded = 4008,
    InvalidOracleQuote = 4009,
    InvalidWeights = 4010,
    RouterResultInvalid = 4011,
    ArithmeticOverflow = 4012,
    SnapshotMismatch = 4013,
    OraclePriceInvalid = 4014,
}

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
pub struct InvestResult {
    pub acquired: Vec<i128>,
    pub asset_prices_e7: Vec<i128>,
    pub deposit_price_e7: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct TimelockedU32 {
    pub value: u32,
    pub execute_after: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct TimelockedRoute {
    pub path: Vec<Address>,
    pub execute_after: u64,
}

#[contractevent(topics = ["swap"], data_format = "vec")]
#[derive(Clone)]
pub struct SwapEvent {
    #[topic]
    pub caller: Address,
    #[topic]
    pub recipient: Address,
    pub amount_in: i128,
    pub amount_out_min: i128,
}

#[contractevent(topics = ["swap_out"], data_format = "vec")]
#[derive(Clone)]
pub struct SwapExactOutEvent {
    #[topic]
    pub caller: Address,
    #[topic]
    pub recipient: Address,
    pub amount_in: i128,
    pub amount_out: i128,
}

#[contractevent(topics = ["invest"], data_format = "single-value")]
#[derive(Clone)]
pub struct InvestEvent {
    #[topic]
    pub basket: Address,
    pub amount: i128,
}

#[contractevent(topics = ["redeem"], data_format = "single-value")]
#[derive(Clone)]
pub struct RedeemEvent {
    #[topic]
    pub basket: Address,
    pub amount: i128,
}

#[contractevent(topics = ["rebalance_settled"], data_format = "single-value")]
#[derive(Clone)]
pub struct RebalanceSettledEvent {
    #[topic]
    pub basket: Address,
    pub holdings: Vec<i128>,
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

    fn swap_tokens_for_exact_tokens(
        env: Env,
        amount_out: i128,
        amount_in_max: i128,
        path: Vec<Address>,
        to: Address,
        deadline: u64,
    ) -> Vec<i128>;

    fn router_pair_for(env: Env, token_a: Address, token_b: Address) -> Address;

    fn router_get_amounts_in(env: Env, amount_out: i128, path: Vec<Address>) -> Vec<i128>;
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Initialized,
    MaxSlippageBps,
    Oracle,
    PendingMaxSlippageBps,
    PendingRoute(Address, Address),
    Route(Address, Address),
    Router,
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
            panic_with_error!(&env, Error::AlreadyInitialized);
        }
        if max_slippage_bps > 10_000 {
            panic_with_error!(&env, Error::SlippageCapInvalid);
        }

        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Router, &router);
        env.storage().instance().set(&DataKey::Oracle, &oracle);
        env.storage()
            .instance()
            .set(&DataKey::MaxSlippageBps, &max_slippage_bps);
        env.storage().instance().set(&DataKey::Initialized, &true);
    }

    pub fn schedule_max_slippage_bps(env: Env, admin: Address, max_slippage_bps: u32) -> u64 {
        if max_slippage_bps > 10_000 {
            panic_with_error!(&env, Error::SlippageCapInvalid);
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
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        if env.ledger().timestamp() < pending.execute_after {
            panic_with_error!(&env, Error::TimelockNotReady);
        }
        env.storage()
            .instance()
            .set(&DataKey::MaxSlippageBps, &pending.value);
        env.storage()
            .instance()
            .remove(&DataKey::PendingMaxSlippageBps);
    }

    pub fn schedule_route(
        env: Env,
        admin: Address,
        input: Address,
        output: Address,
        path: Vec<Address>,
    ) -> u64 {
        validate_route(&env, &path, &input, &output);
        require_admin(&env, &admin);
        let execute_after = timelock_ready_at(&env);
        env.storage().instance().set(
            &DataKey::PendingRoute(input, output),
            &TimelockedRoute {
                path,
                execute_after,
            },
        );
        execute_after
    }

    pub fn execute_route(env: Env, admin: Address, input: Address, output: Address) {
        require_admin(&env, &admin);
        let key = DataKey::PendingRoute(input.clone(), output.clone());
        let pending: TimelockedRoute = env
            .storage()
            .instance()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        if env.ledger().timestamp() < pending.execute_after {
            panic_with_error!(&env, Error::TimelockNotReady);
        }
        validate_route(&env, &pending.path, &input, &output);
        env.storage().instance().set(
            &DataKey::Route(input.clone(), output.clone()),
            &pending.path,
        );
        env.storage().instance().set(
            &DataKey::Route(output, input),
            &reverse_route(&env, &pending.path),
        );
        env.storage().instance().remove(&key);
    }

    pub fn route(env: Env, input: Address, output: Address) -> Vec<Address> {
        read_route(&env, &input, &output)
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
            panic_with_error!(&env, Error::InvalidAmount);
        }
        if slippage_bps > read_slippage_cap(&env) {
            panic_with_error!(&env, Error::SlippageExceeded);
        }

        if caller != to {
            panic_with_error!(&env, Error::Unauthorized);
        }
        validate_path_shape(&env, &path);
        let input = path.first_unchecked();
        let output = path.last_unchecked();
        validate_route(&env, &path, &input, &output);
        let requested_min = expected_amount_out
            .checked_mul((10_000 - slippage_bps) as i128)
            .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow))
            / 10_000;
        let oracle_expected = oracle_quote(&env, &input, &output, amount_in);
        let oracle_min = oracle_expected
            .checked_mul(BPS_DENOMINATOR - read_slippage_cap(&env) as i128)
            .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow))
            / BPS_DENOMINATOR;
        let amount_out_min = if requested_min > oracle_min {
            requested_min
        } else {
            oracle_min
        };
        let amounts = execute_exact_in_route(
            &env,
            &path,
            amount_in,
            &to,
            amount_out_min,
            oracle_expected,
            deadline,
        );

        SwapEvent {
            caller,
            recipient: to,
            amount_in,
            amount_out_min,
        }
        .publish(&env);
        amounts
    }

    pub fn swap_exact_out(
        env: Env,
        caller: Address,
        amount_out: i128,
        amount_in_max: i128,
        path: Vec<Address>,
        to: Address,
        deadline: u64,
    ) -> Vec<i128> {
        caller.require_auth();
        if caller != to {
            panic_with_error!(&env, Error::Unauthorized);
        }
        if amount_out <= 0 || amount_in_max <= 0 {
            panic_with_error!(&env, Error::InvalidAmount);
        }
        validate_path_shape(&env, &path);
        let input = path.first_unchecked();
        let output = path.last_unchecked();
        validate_route(&env, &path, &input, &output);

        let oracle_input = oracle_input_for_output(&env, &input, &output, amount_out);
        let oracle_max = max_input_with_slippage(&env, oracle_input);
        let effective_max = if amount_in_max < oracle_max {
            amount_in_max
        } else {
            oracle_max
        };
        let amounts =
            execute_exact_out_route(&env, &path, amount_out, effective_max, &to, deadline);
        let realized_input = amounts.first_unchecked();
        let realized_output = amounts.last_unchecked();
        if realized_output != amount_out || realized_input > oracle_max {
            panic_with_error!(&env, Error::SlippageExceeded);
        }

        SwapExactOutEvent {
            caller,
            recipient: to,
            amount_in: realized_input,
            amount_out,
        }
        .publish(&env);
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
    ) -> InvestResult {
        basket.require_auth();
        if amount <= 0 {
            panic_with_error!(&env, Error::InvalidAmount);
        }
        validate_weights(&env, &assets, &target_weights_bps);

        let settlement = env.current_contract_address();
        let oracle = read_oracle(&env);
        let oracle_client = OracleAdapterClient::new(&env, &oracle);
        let deposit_price = oracle_client.price(&deposit_asset).price_e7;
        let mut asset_prices = Vec::new(&env);
        for item in assets.iter() {
            let price = if item.address == deposit_asset {
                deposit_price
            } else {
                oracle_client.price(&item.address).price_e7
            };
            if price <= 0 {
                panic_with_error!(&env, Error::OraclePriceInvalid);
            }
            asset_prices.push_back(price);
        }
        let mut acquired = Vec::new(&env);
        let mut allocated = 0_i128;
        for i in 0..assets.len() {
            let asset = assets.get_unchecked(i).address;
            let target_input = if i + 1 == assets.len() {
                amount
                    .checked_sub(allocated)
                    .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow))
            } else {
                amount
                    .checked_mul(target_weights_bps.get_unchecked(i) as i128)
                    .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow))
                    / BPS_DENOMINATOR
            };
            allocated = allocated
                .checked_add(target_input)
                .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow));
            let realized = if deposit_asset == asset {
                target_input
            } else {
                let asset_price = asset_prices.get_unchecked(i);
                let expected = quote_from_prices(&env, target_input, deposit_price, asset_price);
                execute_swap_with_expected(
                    &env,
                    &deposit_asset,
                    &asset,
                    target_input,
                    &settlement,
                    expected,
                )
            };
            transfer_from_settlement(&env, &asset, &settlement, &basket, realized);
            acquired.push_back(realized);
        }

        InvestEvent { basket, amount }.publish(&env);
        InvestResult {
            acquired,
            asset_prices_e7: asset_prices,
            deposit_price_e7: deposit_price,
        }
    }

    pub fn redeem(
        env: Env,
        basket: Address,
        payout_asset: Address,
        assets: Vec<Asset>,
        amounts: Vec<i128>,
        expected_outputs: Vec<i128>,
    ) -> i128 {
        basket.require_auth();
        if assets.len() != amounts.len() || assets.len() != expected_outputs.len() {
            panic_with_error!(&env, Error::SnapshotMismatch);
        }

        let settlement = env.current_contract_address();
        let mut total = 0_i128;
        for i in 0..assets.len() {
            let asset = assets.get_unchecked(i).address;
            let amount = amounts.get_unchecked(i);
            if amount < 0 {
                panic_with_error!(&env, Error::InvalidAmount);
            }
            if amount == 0 {
                continue;
            }
            let expected = expected_outputs.get_unchecked(i);
            if expected <= 0 {
                panic_with_error!(&env, Error::InvalidOracleQuote);
            }
            let realized = if asset == payout_asset {
                amount
            } else {
                execute_swap_with_expected(
                    &env,
                    &asset,
                    &payout_asset,
                    amount,
                    &settlement,
                    expected,
                )
            };
            total = total
                .checked_add(realized)
                .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow));
        }
        transfer_from_settlement(&env, &payout_asset, &settlement, &basket, total);

        RedeemEvent {
            basket,
            amount: total,
        }
        .publish(&env);
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
            panic_with_error!(&env, Error::SnapshotMismatch);
        }

        let settlement = env.current_contract_address();
        let mut base_available = 0_i128;
        for i in 0..assets.len() {
            let asset = assets.get_unchecked(i).address;
            let old_amount = old_holdings.get_unchecked(i);
            let new_amount = new_holdings.get_unchecked(i);
            if old_amount < 0 || new_amount < 0 {
                panic_with_error!(&env, Error::InvalidAmount);
            }
            if old_amount > new_amount {
                let sell_amount = old_amount - new_amount;
                if sell_amount < MIN_REBALANCE_SWAP_AMOUNT {
                    continue;
                }
                let realized = execute_swap(&env, &asset, &base_asset, sell_amount, &settlement);
                base_available = base_available
                    .checked_add(realized)
                    .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow));
            }
        }

        let mut buys_remaining = 0_u32;
        for i in 0..assets.len() {
            let old_amount = old_holdings.get_unchecked(i);
            let new_amount = new_holdings.get_unchecked(i);
            if new_amount - old_amount >= MIN_REBALANCE_SWAP_AMOUNT {
                buys_remaining += 1;
            }
        }

        let mut total_base_needed = 0_i128;
        if buys_remaining > 1 {
            for i in 0..assets.len() {
                let old_amount = old_holdings.get_unchecked(i);
                let new_amount = new_holdings.get_unchecked(i);
                if new_amount - old_amount >= MIN_REBALANCE_SWAP_AMOUNT {
                    let asset = assets.get_unchecked(i).address;
                    total_base_needed = total_base_needed
                        .checked_add(oracle_quote(
                            &env,
                            &asset,
                            &base_asset,
                            new_amount - old_amount,
                        ))
                        .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow));
                }
            }
        }

        let mut base_spent = 0_i128;
        for i in 0..assets.len() {
            let asset = assets.get_unchecked(i).address;
            let old_amount = old_holdings.get_unchecked(i);
            let new_amount = new_holdings.get_unchecked(i);
            if new_amount - old_amount < MIN_REBALANCE_SWAP_AMOUNT {
                continue;
            }
            let base_in = if buys_remaining == 1 {
                base_available
                    .checked_sub(base_spent)
                    .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow))
            } else if total_base_needed == 0 {
                0
            } else {
                let desired_base = oracle_quote(&env, &asset, &base_asset, new_amount - old_amount);
                base_available
                    .checked_mul(desired_base)
                    .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow))
                    / total_base_needed
            };
            base_spent = base_spent
                .checked_add(base_in)
                .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow));
            buys_remaining -= 1;
            if base_in == 0 {
                continue;
            }
            let acquired = execute_swap(&env, &base_asset, &asset, base_in, &settlement);
            transfer_from_settlement(&env, &asset, &settlement, &basket, acquired);
        }

        let base_left = base_available
            .checked_sub(base_spent)
            .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow));
        if base_left > 0 {
            transfer_from_settlement(&env, &base_asset, &settlement, &basket, base_left);
        }

        let mut actual_holdings = Vec::new(&env);
        for item in assets.iter() {
            actual_holdings.push_back(TokenClient::new(&env, &item.address).balance(&basket));
        }

        RebalanceSettledEvent {
            basket,
            holdings: actual_holdings.clone(),
        }
        .publish(&env);
        actual_holdings
    }
}

fn read_slippage_cap(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::MaxSlippageBps)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
}

fn require_admin(env: &Env, admin: &Address) {
    let stored_admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized));
    if *admin != stored_admin {
        panic_with_error!(env, Error::Unauthorized);
    }
    admin.require_auth();
}

fn timelock_ready_at(env: &Env) -> u64 {
    env.ledger()
        .timestamp()
        .checked_add(ADMIN_TIMELOCK_SECONDS)
        .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow))
}

fn read_oracle(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Oracle)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
}

fn read_route(env: &Env, input: &Address, output: &Address) -> Vec<Address> {
    let path = env
        .storage()
        .instance()
        .get(&DataKey::Route(input.clone(), output.clone()))
        .unwrap_or(vec![env, input.clone(), output.clone()]);
    validate_route(env, &path, input, output);
    path
}

fn validate_path_shape(env: &Env, path: &Vec<Address>) {
    if path.len() < 2 || path.len() > MAX_ROUTE_LENGTH {
        panic_with_error!(env, Error::InvalidRoute);
    }
    for i in 0..path.len() {
        let current = path.get_unchecked(i);
        for j in (i + 1)..path.len() {
            if current == path.get_unchecked(j) {
                panic_with_error!(env, Error::InvalidRoute);
            }
        }
    }
}

fn validate_route(env: &Env, path: &Vec<Address>, input: &Address, output: &Address) {
    validate_path_shape(env, path);
    if path.first_unchecked() != *input || path.last_unchecked() != *output {
        panic_with_error!(env, Error::InvalidRoute);
    }
}

fn reverse_route(env: &Env, path: &Vec<Address>) -> Vec<Address> {
    let mut reversed = Vec::new(env);
    let mut index = path.len();
    while index > 0 {
        index -= 1;
        reversed.push_back(path.get_unchecked(index));
    }
    reversed
}

fn oracle_quote(env: &Env, input: &Address, output: &Address, amount_in: i128) -> i128 {
    if amount_in == 0 || input == output {
        return amount_in;
    }
    let oracle = read_oracle(env);
    let client = OracleAdapterClient::new(env, &oracle);
    let input_price = client.price(input).price_e7;
    let output_price = client.price(output).price_e7;
    quote_from_prices(env, amount_in, input_price, output_price)
}

fn oracle_input_for_output(env: &Env, input: &Address, output: &Address, amount_out: i128) -> i128 {
    if amount_out <= 0 {
        panic_with_error!(env, Error::InvalidAmount);
    }
    if input == output {
        return amount_out;
    }
    let oracle = read_oracle(env);
    let client = OracleAdapterClient::new(env, &oracle);
    let input_price = client.price(input).price_e7;
    let output_price = client.price(output).price_e7;
    if input_price <= 0 || output_price <= 0 {
        panic_with_error!(env, Error::InvalidOracleQuote);
    }
    ceil_div(
        env,
        amount_out
            .checked_mul(output_price)
            .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow)),
        input_price,
    )
}

fn max_input_with_slippage(env: &Env, expected_input: i128) -> i128 {
    ceil_div(
        env,
        expected_input
            .checked_mul(BPS_DENOMINATOR + read_slippage_cap(env) as i128)
            .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow)),
        BPS_DENOMINATOR,
    )
}

fn ceil_div(env: &Env, numerator: i128, denominator: i128) -> i128 {
    if numerator < 0 || denominator <= 0 {
        panic_with_error!(env, Error::InvalidAmount);
    }
    if numerator == 0 {
        0
    } else {
        numerator
            .checked_add(denominator - 1)
            .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow))
            / denominator
    }
}

fn quote_from_prices(env: &Env, amount_in: i128, input_price: i128, output_price: i128) -> i128 {
    if amount_in < 0 || input_price <= 0 || output_price <= 0 {
        panic_with_error!(env, Error::InvalidOracleQuote);
    }
    amount_in
        .checked_mul(input_price)
        .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow))
        / output_price
}

fn enforce_expected_slippage(env: &Env, expected: i128, realized_amount_out: i128) {
    if expected <= 0 || realized_amount_out < 0 {
        panic_with_error!(env, Error::InvalidAmount);
    }
    let min_out = expected
        .checked_mul(BPS_DENOMINATOR - read_slippage_cap(env) as i128)
        .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow))
        / BPS_DENOMINATOR;
    if realized_amount_out < min_out {
        panic_with_error!(env, Error::SlippageExceeded);
    }
    let _ = PRICE_SCALE;
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

fn execute_swap(
    env: &Env,
    input: &Address,
    output: &Address,
    amount_in: i128,
    owner: &Address,
) -> i128 {
    if amount_in == 0 {
        return 0;
    }
    if input == output {
        return amount_in;
    }

    let expected = oracle_quote(env, input, output, amount_in);
    execute_swap_with_expected(env, input, output, amount_in, owner, expected)
}

fn execute_swap_with_expected(
    env: &Env,
    input: &Address,
    output: &Address,
    amount_in: i128,
    owner: &Address,
    expected: i128,
) -> i128 {
    let amount_out_min = expected
        .checked_mul(BPS_DENOMINATOR - read_slippage_cap(env) as i128)
        .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow))
        / BPS_DENOMINATOR;
    if amount_out_min <= 0 {
        panic_with_error!(env, Error::InvalidOracleQuote);
    }

    let path = read_route(env, input, output);
    let deadline = env
        .ledger()
        .timestamp()
        .checked_add(300)
        .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow));
    let amounts = execute_exact_in_route(
        env,
        &path,
        amount_in,
        owner,
        amount_out_min,
        expected,
        deadline,
    );
    let realized = amounts.last_unchecked();
    enforce_expected_slippage(env, expected, realized);
    realized
}

fn execute_exact_in_route(
    env: &Env,
    path: &Vec<Address>,
    amount_in: i128,
    owner: &Address,
    amount_out_min: i128,
    oracle_expected: i128,
    deadline: u64,
) -> Vec<i128> {
    validate_path_shape(env, path);
    let input = path.first_unchecked();
    let output = path.last_unchecked();
    validate_route(env, path, &input, &output);
    let router: Address = env
        .storage()
        .instance()
        .get(&DataKey::Router)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized));
    let client = SoroswapRouterClient::new(env, &router);
    let pair = client.router_pair_for(&input, &path.get_unchecked(1));
    authorize_router_transfer(env, &pair, &input, amount_in, owner);
    let amounts = match client.try_swap_exact_tokens_for_tokens(
        &amount_in,
        &amount_out_min,
        path,
        owner,
        &deadline,
    ) {
        Ok(Ok(amounts)) => amounts,
        _ => panic_with_error!(env, Error::SlippageExceeded),
    };
    validate_router_amounts(env, path, &amounts, amount_in);
    enforce_expected_slippage(env, oracle_expected, amounts.last_unchecked());
    amounts
}

fn execute_exact_out_route(
    env: &Env,
    path: &Vec<Address>,
    amount_out: i128,
    amount_in_max: i128,
    owner: &Address,
    deadline: u64,
) -> Vec<i128> {
    validate_path_shape(env, path);
    let router: Address = env
        .storage()
        .instance()
        .get(&DataKey::Router)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized));
    let client = SoroswapRouterClient::new(env, &router);
    let quoted = client.router_get_amounts_in(&amount_out, path);
    if quoted.len() != path.len() || quoted.is_empty() {
        panic_with_error!(env, Error::RouterResultInvalid);
    }
    let required_input = quoted.first_unchecked();
    if required_input <= 0 || required_input > amount_in_max {
        panic_with_error!(env, Error::SlippageExceeded);
    }
    let input = path.first_unchecked();
    let pair = client.router_pair_for(&input, &path.get_unchecked(1));
    authorize_router_transfer(env, &pair, &input, required_input, owner);
    let amounts = match client.try_swap_tokens_for_exact_tokens(
        &amount_out,
        &amount_in_max,
        path,
        owner,
        &deadline,
    ) {
        Ok(Ok(amounts)) => amounts,
        _ => panic_with_error!(env, Error::SlippageExceeded),
    };
    validate_router_amounts(env, path, &amounts, required_input);
    amounts
}

fn validate_router_amounts(
    env: &Env,
    path: &Vec<Address>,
    amounts: &Vec<i128>,
    expected_input: i128,
) {
    if amounts.len() != path.len() || amounts.is_empty() {
        panic_with_error!(env, Error::RouterResultInvalid);
    }
    if amounts.first_unchecked() != expected_input || amounts.last_unchecked() <= 0 {
        panic_with_error!(env, Error::RouterResultInvalid);
    }
}

fn authorize_router_transfer(
    env: &Env,
    pair: &Address,
    input: &Address,
    amount_in: i128,
    owner: &Address,
) {
    if *owner != env.current_contract_address() {
        return;
    }
    let token_transfer = InvokerContractAuthEntry::Contract(SubContractInvocation {
        context: ContractContext {
            contract: input.clone(),
            fn_name: symbol_short!("transfer"),
            // The deployed Soroswap router uses the legacy SEP-41 Address
            // destination signature, so the authorization preimage must match it.
            args: (owner.clone(), pair.clone(), amount_in).into_val(env),
        },
        sub_invocations: vec![env],
    });
    env.authorize_as_current_contract(vec![env, token_transfer]);
}

fn transfer_from_settlement(
    env: &Env,
    token: &Address,
    from: &Address,
    to: &Address,
    amount: i128,
) {
    if amount == 0 {
        return;
    }
    authorize_contract_call(
        env,
        token.clone(),
        symbol_short!("transfer"),
        (from.clone(), MuxedAddress::from(to.clone()), amount).into_val(env),
    );
    TokenClient::new(env, token).transfer(from, MuxedAddress::from(to.clone()), &amount);
}

fn authorize_contract_call(
    env: &Env,
    contract: Address,
    fn_name: soroban_sdk::Symbol,
    args: Vec<Val>,
) {
    env.authorize_as_current_contract(vec![
        env,
        InvokerContractAuthEntry::Contract(SubContractInvocation {
            context: ContractContext {
                contract,
                fn_name,
                args,
            },
            sub_invocations: vec![env],
        }),
    ]);
}

#[cfg(test)]
mod test {
    extern crate std;

    use super::*;
    use oracle_adapter::{OracleAdapter, OracleAdapterClient};
    use soroban_sdk::{
        contract, contractimpl,
        testutils::{Address as _, Ledger},
        token::StellarAssetClient,
        Env,
    };

    #[contract]
    struct RouterAuthProbe;

    #[contract]
    struct RouterReadProbe;

    #[contractimpl]
    impl RouterReadProbe {
        pub fn reserves() -> (i128, i128) {
            (1_000_000_000, 1_000_000_000)
        }
    }

    #[contractimpl]
    impl RouterAuthProbe {
        pub fn initialize(env: Env, pair: Address) {
            env.storage().instance().set(&symbol_short!("pair"), &pair);
        }

        pub fn router_pair_for(env: Env, _token_a: Address, _token_b: Address) -> Address {
            env.storage()
                .instance()
                .get(&symbol_short!("pair"))
                .unwrap()
        }

        pub fn swap_exact_tokens_for_tokens(
            env: Env,
            amount_in: i128,
            _amount_out_min: i128,
            path: Vec<Address>,
            to: Address,
            _deadline: u64,
        ) -> Vec<i128> {
            to.require_auth();
            let _: (i128, i128) = env.invoke_contract(
                &path.get_unchecked(1),
                &symbol_short!("reserves"),
                vec![&env],
            );
            let pair: Address = env
                .storage()
                .instance()
                .get(&symbol_short!("pair"))
                .unwrap();
            env.invoke_contract::<()>(
                &path.get_unchecked(0),
                &symbol_short!("transfer"),
                (to, pair, amount_in).into_val(&env),
            );
            let mut amounts = Vec::new(&env);
            for _ in 0..path.len() {
                amounts.push_back(amount_in);
            }
            amounts
        }

        pub fn router_get_amounts_in(env: Env, amount_out: i128, path: Vec<Address>) -> Vec<i128> {
            let mut amounts = Vec::new(&env);
            amounts.push_back(amount_out.checked_mul(2).unwrap());
            for _ in 1..path.len() {
                amounts.push_back(amount_out);
            }
            amounts
        }

        pub fn swap_tokens_for_exact_tokens(
            env: Env,
            amount_out: i128,
            amount_in_max: i128,
            path: Vec<Address>,
            to: Address,
            _deadline: u64,
        ) -> Vec<i128> {
            to.require_auth();
            let required_input = amount_out.checked_mul(2).unwrap();
            if required_input > amount_in_max {
                panic!("router exact-out maximum exceeded");
            }
            let pair: Address = env
                .storage()
                .instance()
                .get(&symbol_short!("pair"))
                .unwrap();
            env.invoke_contract::<()>(
                &path.get_unchecked(0),
                &symbol_short!("transfer"),
                (to, pair, required_input).into_val(&env),
            );
            let mut amounts = Vec::new(&env);
            amounts.push_back(required_input);
            for _ in 1..path.len() {
                amounts.push_back(amount_out);
            }
            amounts
        }
    }

    #[contract]
    struct ContractAuthProbe;

    #[contractimpl]
    impl ContractAuthProbe {
        pub fn route(env: Env, router: Address, input: Address, output: Address, amount_in: i128) {
            let owner = env.current_contract_address();
            authorize_contract_call(
                &env,
                input.clone(),
                symbol_short!("transfer"),
                (owner.clone(), MuxedAddress::from(output.clone()), 1_i128).into_val(&env),
            );
            TokenClient::new(&env, &input).transfer(
                &owner,
                MuxedAddress::from(output.clone()),
                &1_i128,
            );
            let client = SoroswapRouterClient::new(&env, &router);
            let pair = client.router_pair_for(&input, &output);
            let path = vec![&env, input.clone(), output];
            let amount_out_min = amount_in;
            let deadline = env.ledger().timestamp() + 300;
            authorize_router_transfer(&env, &pair, &input, amount_in, &owner);
            client.swap_exact_tokens_for_tokens(
                &amount_in,
                &amount_out_min,
                &path,
                &owner,
                &deadline,
            );
        }
    }

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

    #[test]
    fn contract_authorizes_router_and_nested_asset_transfer() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let pair = Address::generate(&env);
        let input = env.register_stellar_asset_contract_v2(admin.clone());
        let output = env.register(RouterReadProbe, ());
        let router = env.register(RouterAuthProbe, ());
        RouterAuthProbeClient::new(&env, &router).initialize(&pair);
        let owner = env.register(ContractAuthProbe, ());
        let amount = 35_000_000_i128;

        env.mock_all_auths();
        StellarAssetClient::new(&env, &input.address()).mint(&owner, &(amount + 1));
        env.set_auths(&[]);

        ContractAuthProbeClient::new(&env, &owner).route(
            &router,
            &input.address(),
            &output,
            &amount,
        );

        assert_eq!(TokenClient::new(&env, &input.address()).balance(&owner), 0);
        assert_eq!(
            TokenClient::new(&env, &input.address()).balance(&pair),
            amount
        );
    }

    fn setup_live_router_probe(
        max_slippage_bps: u32,
        input_price: i128,
        output_price: i128,
    ) -> (Env, Address, Address, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1_000);
        let admin = Address::generate(&env);
        let signer = Address::generate(&env);
        let pair = Address::generate(&env);
        let input_admin = Address::generate(&env);
        let output_admin = Address::generate(&env);
        let input = env.register_stellar_asset_contract_v2(input_admin);
        let output = env.register_stellar_asset_contract_v2(output_admin);
        let router = env.register(RouterAuthProbe, ());
        RouterAuthProbeClient::new(&env, &router).initialize(&pair);
        let oracle = env.register(OracleAdapter, ());
        let oracle_client = OracleAdapterClient::new(&env, &oracle);
        oracle_client.initialize(
            &admin,
            &Address::generate(&env),
            &false,
            &3_600_u64,
            &vec![&env, signer.clone()],
            &1_u32,
        );
        oracle_client.set_fallback_price(
            &input.address(),
            &input_price,
            &1_000_u64,
            &vec![&env, signer.clone()],
        );
        oracle_client.set_fallback_price(
            &output.address(),
            &output_price,
            &1_000_u64,
            &vec![&env, signer],
        );
        let settlement = env.register(Settlement, ());
        SettlementClient::new(&env, &settlement).initialize(
            &admin,
            &router,
            &oracle,
            &max_slippage_bps,
        );
        (
            env,
            admin,
            settlement,
            input.address(),
            output.address(),
            pair,
        )
    }

    #[test]
    fn exact_out_uses_router_quote_and_realized_amounts() {
        let (env, _admin, settlement, input, output, pair) =
            setup_live_router_probe(500, 10_000_000, 20_000_000);
        let caller = Address::generate(&env);
        let intermediate = env.register(RouterReadProbe, ());
        let amount_out = 50_000_i128;
        StellarAssetClient::new(&env, &input).mint(&caller, &(amount_out * 3));

        let amounts = SettlementClient::new(&env, &settlement).swap_exact_out(
            &caller,
            &amount_out,
            &(amount_out * 3),
            &vec![&env, input.clone(), intermediate, output],
            &caller,
            &(env.ledger().timestamp() + 300),
        );

        assert_eq!(amounts.first_unchecked(), amount_out * 2);
        assert_eq!(amounts.last_unchecked(), amount_out);
        assert_eq!(
            TokenClient::new(&env, &input).balance(&pair),
            amount_out * 2
        );
    }

    #[test]
    fn exact_in_accepts_a_multihop_route() {
        let (env, _admin, settlement, input, output, pair) =
            setup_live_router_probe(500, 10_000_000, 10_000_000);
        let caller = Address::generate(&env);
        let intermediate = env.register(RouterReadProbe, ());
        let amount_in = 75_000_i128;
        StellarAssetClient::new(&env, &input).mint(&caller, &amount_in);

        let amounts = SettlementClient::new(&env, &settlement).swap(
            &caller,
            &amount_in,
            &amount_in,
            &500_u32,
            &vec![&env, input.clone(), intermediate, output],
            &caller,
            &(env.ledger().timestamp() + 300),
        );

        assert_eq!(amounts.len(), 3);
        assert_eq!(amounts.last_unchecked(), amount_in);
        assert_eq!(TokenClient::new(&env, &input).balance(&pair), amount_in);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4008)")]
    fn exact_out_rejects_router_quote_outside_oracle_tolerance() {
        let (env, _admin, settlement, input, output, _pair) =
            setup_live_router_probe(500, 10_000_000, 10_000_000);
        let caller = Address::generate(&env);
        let amount_out = 50_000_i128;
        StellarAssetClient::new(&env, &input).mint(&caller, &(amount_out * 3));

        SettlementClient::new(&env, &settlement).swap_exact_out(
            &caller,
            &amount_out,
            &(amount_out * 3),
            &vec![&env, input, output],
            &caller,
            &(env.ledger().timestamp() + 300),
        );
    }

    #[test]
    fn route_change_is_timelocked_and_installs_reverse_route() {
        let (env, admin, settlement) = setup();
        let input = Address::generate(&env);
        let intermediate = Address::generate(&env);
        let output = Address::generate(&env);
        let path = vec![&env, input.clone(), intermediate.clone(), output.clone()];
        let client = SettlementClient::new(&env, &settlement);

        let ready_at = client.schedule_route(&admin, &input, &output, &path);
        assert_eq!(
            client.route(&input, &output),
            vec![&env, input.clone(), output.clone()]
        );
        env.ledger().set_timestamp(ready_at);
        client.execute_route(&admin, &input, &output);

        assert_eq!(client.route(&input, &output), path);
        assert_eq!(
            client.route(&output, &input),
            vec![&env, output, intermediate, input]
        );
    }
}
