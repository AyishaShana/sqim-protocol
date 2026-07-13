#![no_std]

use soroban_sdk::{
    auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation},
    contract, contractclient, contractimpl, contracttype, symbol_short,
    token::TokenClient,
    vec, Address, Env, IntoVal, MuxedAddress, String, Val, Vec,
};

const BPS_DENOMINATOR: i128 = 10_000;
const NAV_SCALE: i128 = 10_000_000;
const ADMIN_TIMELOCK_SECONDS: u64 = 86_400;

#[derive(Clone)]
#[contracttype]
pub struct Asset {
    pub address: Address,
}

#[derive(Clone)]
#[contracttype]
pub struct Position {
    pub tracked_shares: i128,
    pub average_cost_per_share: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct TimelockedU32 {
    pub value: u32,
    pub execute_after: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct TimelockedI128 {
    pub value: i128,
    pub execute_after: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct TimelockedRebalancers {
    pub rebalancers: Vec<Address>,
    pub threshold: u32,
    pub execute_after: u64,
}

#[contractclient(name = "BasketShareTokenClient")]
pub trait BasketShareToken {
    fn mint(env: Env, to: Address, amount: i128);
    fn burn(env: Env, from: Address, amount: i128);
    fn balance(env: Env, id: Address) -> i128;
    fn total_supply(env: Env) -> i128;
}

#[contractclient(name = "SettlementClient")]
pub trait Settlement {
    fn invest(
        env: Env,
        basket: Address,
        deposit_asset: Address,
        amount: i128,
        assets: Vec<Asset>,
        target_weights_bps: Vec<u32>,
    ) -> Vec<i128>;

    fn redeem(
        env: Env,
        basket: Address,
        payout_asset: Address,
        assets: Vec<Asset>,
        amounts: Vec<i128>,
    ) -> i128;

    fn rebalance(
        env: Env,
        basket: Address,
        base_asset: Address,
        assets: Vec<Asset>,
        old_holdings: Vec<i128>,
        new_holdings: Vec<i128>,
    ) -> Vec<i128>;
}

#[contractclient(name = "OracleAdapterClient")]
pub trait OracleAdapter {
    fn price(env: Env, asset: Address) -> OraclePrice;
}

#[derive(Clone)]
#[contracttype]
pub struct OraclePrice {
    pub asset: Address,
    pub price_e7: i128,
    pub updated_at: u64,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Assets,
    DepositAsset,
    Holding(Address),
    Initialized,
    Name,
    Oracle,
    Position(Address),
    MaxDriftBps,
    MaxTransactionAmount,
    Paused,
    PendingMaxDriftBps,
    PendingMaxTransactionAmount,
    PendingRebalancers,
    PendingWithdrawalFeeBps,
    Rebalancers,
    RebalancerThreshold,
    Settlement,
    ShareToken,
    TargetWeights,
    TotalBasketValue,
    WithdrawalFeeBps,
}

#[contract]
pub struct Basket;

#[contractimpl]
impl Basket {
    pub fn initialize(
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
    ) {
        if env.storage().instance().has(&DataKey::Initialized) {
            panic!("basket already initialized");
        }
        admin.require_auth();
        validate_weights(&assets, &target_weights_bps);
        if withdrawal_fee_bps > 10_000 {
            panic!("basket withdrawal fee exceeds 100 percent");
        }
        if max_drift_bps > 10_000 {
            panic!("basket max drift exceeds 100 percent");
        }
        check_nonnegative(max_transaction_amount);
        validate_rebalancer_quorum(&rebalancers, rebalancer_threshold);

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Name, &name);
        env.storage().instance().set(&DataKey::Assets, &assets);
        env.storage()
            .instance()
            .set(&DataKey::TargetWeights, &target_weights_bps);
        env.storage()
            .instance()
            .set(&DataKey::ShareToken, &share_token);
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
        env.storage()
            .instance()
            .set(&DataKey::TotalBasketValue, &0_i128);
        env.storage().instance().set(&DataKey::Paused, &false);
        env.storage().instance().set(&DataKey::Initialized, &true);
    }

    pub fn name(env: Env) -> String {
        env.storage().instance().get(&DataKey::Name).unwrap()
    }

    pub fn assets(env: Env) -> Vec<Asset> {
        read_assets(&env)
    }

    pub fn target_weights_bps(env: Env) -> Vec<u32> {
        env.storage()
            .instance()
            .get(&DataKey::TargetWeights)
            .unwrap()
    }

    pub fn share_token(env: Env) -> Address {
        read_share_token(&env)
    }

    pub fn total_basket_value(env: Env) -> i128 {
        read_total_value(&env)
    }

    pub fn paused(env: Env) -> bool {
        is_paused(&env)
    }

    pub fn max_transaction_amount(env: Env) -> i128 {
        read_max_transaction_amount(&env)
    }

    pub fn withdrawal_fee_bps(env: Env) -> u32 {
        read_withdrawal_fee_bps(&env)
    }

    pub fn max_drift_bps(env: Env) -> u32 {
        read_max_drift_bps(&env)
    }

    pub fn nav(env: Env) -> i128 {
        nav(&env)
    }

    pub fn position(env: Env, holder: Address) -> Position {
        read_position(&env, &holder).unwrap_or(Position {
            tracked_shares: 0,
            average_cost_per_share: nav(&env),
        })
    }

    pub fn holding(env: Env, asset: Address) -> i128 {
        read_holding(&env, &asset)
    }

    pub fn mark_to_market(env: Env, admin: Address, total_basket_value: i128) {
        check_nonnegative(total_basket_value);
        require_admin(&env, &admin);
        write_total_value(&env, total_basket_value);
    }

    pub fn pause(env: Env, admin: Address) {
        require_admin(&env, &admin);
        env.storage().instance().set(&DataKey::Paused, &true);
        env.events().publish((symbol_short!("pause"), admin), true);
    }

    pub fn unpause(env: Env, admin: Address) {
        require_admin(&env, &admin);
        env.storage().instance().set(&DataKey::Paused, &false);
        env.events()
            .publish((symbol_short!("unpause"), admin), false);
    }

    pub fn schedule_withdrawal_fee_bps(env: Env, admin: Address, value: u32) -> u64 {
        if value > 10_000 {
            panic!("basket withdrawal fee exceeds 100 percent");
        }
        require_admin(&env, &admin);
        let execute_after = timelock_ready_at(&env);
        env.storage().instance().set(
            &DataKey::PendingWithdrawalFeeBps,
            &TimelockedU32 {
                value,
                execute_after,
            },
        );
        execute_after
    }

    pub fn execute_withdrawal_fee_bps(env: Env, admin: Address) {
        require_admin(&env, &admin);
        let pending: TimelockedU32 = env
            .storage()
            .instance()
            .get(&DataKey::PendingWithdrawalFeeBps)
            .unwrap();
        require_timelock_ready(&env, pending.execute_after);
        env.storage()
            .instance()
            .set(&DataKey::WithdrawalFeeBps, &pending.value);
        env.storage()
            .instance()
            .remove(&DataKey::PendingWithdrawalFeeBps);
    }

    pub fn schedule_max_drift_bps(env: Env, admin: Address, value: u32) -> u64 {
        if value > 10_000 {
            panic!("basket max drift exceeds 100 percent");
        }
        require_admin(&env, &admin);
        let execute_after = timelock_ready_at(&env);
        env.storage().instance().set(
            &DataKey::PendingMaxDriftBps,
            &TimelockedU32 {
                value,
                execute_after,
            },
        );
        execute_after
    }

    pub fn execute_max_drift_bps(env: Env, admin: Address) {
        require_admin(&env, &admin);
        let pending: TimelockedU32 = env
            .storage()
            .instance()
            .get(&DataKey::PendingMaxDriftBps)
            .unwrap();
        require_timelock_ready(&env, pending.execute_after);
        env.storage()
            .instance()
            .set(&DataKey::MaxDriftBps, &pending.value);
        env.storage()
            .instance()
            .remove(&DataKey::PendingMaxDriftBps);
    }

    pub fn schedule_rebalancers(
        env: Env,
        admin: Address,
        rebalancers: Vec<Address>,
        threshold: u32,
    ) -> u64 {
        validate_rebalancer_quorum(&rebalancers, threshold);
        require_admin(&env, &admin);
        let execute_after = timelock_ready_at(&env);
        env.storage().instance().set(
            &DataKey::PendingRebalancers,
            &TimelockedRebalancers {
                rebalancers,
                threshold,
                execute_after,
            },
        );
        execute_after
    }

    pub fn execute_rebalancers(env: Env, admin: Address) {
        require_admin(&env, &admin);
        let pending: TimelockedRebalancers = env
            .storage()
            .instance()
            .get(&DataKey::PendingRebalancers)
            .unwrap();
        require_timelock_ready(&env, pending.execute_after);
        env.storage()
            .instance()
            .set(&DataKey::Rebalancers, &pending.rebalancers);
        env.storage()
            .instance()
            .set(&DataKey::RebalancerThreshold, &pending.threshold);
        env.storage()
            .instance()
            .remove(&DataKey::PendingRebalancers);
    }

    pub fn schedule_max_transaction_amount(env: Env, admin: Address, value: i128) -> u64 {
        check_nonnegative(value);
        require_admin(&env, &admin);
        let execute_after = timelock_ready_at(&env);
        env.storage().instance().set(
            &DataKey::PendingMaxTransactionAmount,
            &TimelockedI128 {
                value,
                execute_after,
            },
        );
        execute_after
    }

    pub fn execute_max_transaction_amount(env: Env, admin: Address) {
        require_admin(&env, &admin);
        let pending: TimelockedI128 = env
            .storage()
            .instance()
            .get(&DataKey::PendingMaxTransactionAmount)
            .unwrap();
        require_timelock_ready(&env, pending.execute_after);
        env.storage()
            .instance()
            .set(&DataKey::MaxTransactionAmount, &pending.value);
        env.storage()
            .instance()
            .remove(&DataKey::PendingMaxTransactionAmount);
    }

    pub fn deposit(env: Env, depositor: Address, amount: i128) -> i128 {
        ensure_not_paused(&env);
        check_positive(amount);
        enforce_max_transaction_amount(&env, amount);
        depositor.require_auth();

        let current_nav = nav(&env);
        let shares_to_mint = amount.checked_mul(NAV_SCALE).unwrap() / current_nav;
        check_positive(shares_to_mint);

        let basket_address = env.current_contract_address();
        let deposit_asset = read_deposit_asset(&env);
        TokenClient::new(&env, &deposit_asset).transfer(
            &depositor,
            &MuxedAddress::from(basket_address.clone()),
            &amount,
        );

        let assets = read_assets(&env);
        let weights = read_weights(&env);
        let settlement = read_settlement(&env);
        authorize_contract_call(
            &env,
            settlement.clone(),
            symbol_short!("invest"),
            (
                basket_address.clone(),
                deposit_asset.clone(),
                amount,
                assets.clone(),
                weights.clone(),
            )
                .into_val(&env),
        );
        let acquired = SettlementClient::new(&env, &settlement).invest(
            &basket_address,
            &deposit_asset,
            &amount,
            &assets,
            &weights,
        );
        apply_acquired_assets(&env, &assets, &acquired);

        let share_token = read_share_token(&env);
        authorize_contract_call(
            &env,
            share_token.clone(),
            symbol_short!("mint"),
            (depositor.clone(), shares_to_mint).into_val(&env),
        );
        BasketShareTokenClient::new(&env, &share_token).mint(&depositor, &shares_to_mint);

        add_position(&env, &depositor, shares_to_mint, current_nav);
        write_total_value(&env, read_total_value(&env).checked_add(amount).unwrap());

        env.events().publish(
            (symbol_short!("deposit"), depositor, deposit_asset),
            (amount, shares_to_mint),
        );
        shares_to_mint
    }

    pub fn withdraw(env: Env, holder: Address, basket_token_amount: i128) -> i128 {
        ensure_not_paused(&env);
        check_positive(basket_token_amount);
        holder.require_auth();

        let share_token = read_share_token(&env);
        let share_client = BasketShareTokenClient::new(&env, &share_token);
        let holder_balance = share_client.balance(&holder);
        if holder_balance < basket_token_amount {
            panic!("insufficient basket token balance");
        }

        let total_supply = share_client.total_supply();
        check_positive(total_supply);
        let current_nav = nav(&env);
        let gross_value = basket_token_amount.checked_mul(current_nav).unwrap() / NAV_SCALE;
        check_positive(gross_value);
        enforce_max_transaction_amount(&env, gross_value);

        let assets = read_assets(&env);
        let redeemed_amounts =
            proportional_redemption_amounts(&env, &assets, basket_token_amount, total_supply);
        reduce_holdings(&env, &assets, &redeemed_amounts);

        share_client.burn(&holder, &basket_token_amount);

        let basket_address = env.current_contract_address();
        let payout_asset = read_deposit_asset(&env);
        let settlement = read_settlement(&env);
        authorize_contract_call(
            &env,
            settlement.clone(),
            symbol_short!("redeem"),
            (
                basket_address.clone(),
                payout_asset.clone(),
                assets.clone(),
                redeemed_amounts.clone(),
            )
                .into_val(&env),
        );
        SettlementClient::new(&env, &settlement).redeem(
            &basket_address,
            &payout_asset,
            &assets,
            &redeemed_amounts,
        );
        let realized_value = gross_value;

        let fee = withdrawal_fee(
            &env,
            &holder,
            basket_token_amount,
            realized_value,
            current_nav,
        );
        let net = realized_value - fee;
        check_nonnegative(net);
        reduce_position(&env, &holder, basket_token_amount);

        let total_value = read_total_value(&env);
        write_total_value(&env, total_value.saturating_sub(gross_value));

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if fee > 0 {
            transfer_from_basket(&env, &payout_asset, &basket_address, &admin, fee);
        }
        if net > 0 {
            transfer_from_basket(&env, &payout_asset, &basket_address, &holder, net);
        }

        env.events().publish(
            (symbol_short!("withdraw"), holder, payout_asset),
            (basket_token_amount, net, fee),
        );
        net
    }

    pub fn rebalance(
        env: Env,
        caller: Address,
        new_weights_bps: Vec<u32>,
        rebalancer_signers: Vec<Address>,
    ) -> Vec<i128> {
        ensure_not_paused(&env);
        let assets = read_assets(&env);
        validate_weights(&assets, &new_weights_bps);
        authorize_rebalance(&env, &caller, &rebalancer_signers);

        let old_weights = read_weights(&env);
        enforce_drift_bound(&env, &old_weights, &new_weights_bps);

        let share_token = read_share_token(&env);
        let total_supply_before = BasketShareTokenClient::new(&env, &share_token).total_supply();
        let total_value = read_total_value(&env);
        let old_holdings = holdings_for_assets(&env, &assets);
        let deposit_asset = read_deposit_asset(&env);
        let new_holdings =
            target_holdings(&env, &assets, &deposit_asset, total_value, &new_weights_bps);
        enforce_rebalance_transaction_amount(
            &env,
            &assets,
            &deposit_asset,
            &old_holdings,
            &new_holdings,
        );
        let settlement = read_settlement(&env);
        let basket_address = env.current_contract_address();

        authorize_contract_call(
            &env,
            settlement.clone(),
            symbol_short!("rebalance"),
            (
                basket_address.clone(),
                deposit_asset.clone(),
                assets.clone(),
                old_holdings.clone(),
                new_holdings.clone(),
            )
                .into_val(&env),
        );
        let updated_holdings = SettlementClient::new(&env, &settlement).rebalance(
            &basket_address,
            &deposit_asset,
            &assets,
            &old_holdings,
            &new_holdings,
        );
        if updated_holdings.len() != assets.len() {
            panic!("settlement returned wrong rebalance holding count");
        }

        for i in 0..assets.len() {
            write_holding(
                &env,
                &assets.get_unchecked(i).address,
                updated_holdings.get_unchecked(i),
            );
        }
        env.storage()
            .instance()
            .set(&DataKey::TargetWeights, &new_weights_bps);

        let total_supply_after = BasketShareTokenClient::new(&env, &share_token).total_supply();
        if total_supply_after != total_supply_before {
            panic!("rebalance changed basket token supply");
        }

        env.events()
            .publish((symbol_short!("rebalance"), caller), new_weights_bps);
        updated_holdings
    }
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

fn check_positive(amount: i128) {
    if amount <= 0 {
        panic!("basket amount must be positive");
    }
}

fn check_nonnegative(amount: i128) {
    if amount < 0 {
        panic!("basket amount must not be negative");
    }
}

fn read_assets(env: &Env) -> Vec<Asset> {
    env.storage().instance().get(&DataKey::Assets).unwrap()
}

fn read_weights(env: &Env) -> Vec<u32> {
    env.storage()
        .instance()
        .get(&DataKey::TargetWeights)
        .unwrap()
}

fn read_rebalancers(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::Rebalancers)
        .unwrap_or(Vec::new(env))
}

fn read_rebalancer_threshold(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::RebalancerThreshold)
        .unwrap_or(0)
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

fn read_withdrawal_fee_bps(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::WithdrawalFeeBps)
        .unwrap()
}

fn read_deposit_asset(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::DepositAsset)
        .unwrap()
}

fn read_settlement(env: &Env) -> Address {
    env.storage().instance().get(&DataKey::Settlement).unwrap()
}

fn read_oracle(env: &Env) -> Address {
    env.storage().instance().get(&DataKey::Oracle).unwrap()
}

fn read_share_token(env: &Env) -> Address {
    env.storage().instance().get(&DataKey::ShareToken).unwrap()
}

fn read_total_value(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::TotalBasketValue)
        .unwrap_or(0)
}

fn write_total_value(env: &Env, value: i128) {
    check_nonnegative(value);
    env.storage()
        .instance()
        .set(&DataKey::TotalBasketValue, &value);
}

fn nav(env: &Env) -> i128 {
    let share_token = read_share_token(env);
    let supply = BasketShareTokenClient::new(env, &share_token).total_supply();
    if supply == 0 {
        NAV_SCALE
    } else {
        read_total_value(env).checked_mul(NAV_SCALE).unwrap() / supply
    }
}

fn is_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::Paused)
        .unwrap_or(false)
}

fn ensure_not_paused(env: &Env) {
    if is_paused(env) {
        panic!("basket is paused");
    }
}

fn require_admin(env: &Env, admin: &Address) {
    let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    if *admin != stored_admin {
        panic!("only basket creator can perform admin action");
    }
    admin.require_auth();
}

fn timelock_ready_at(env: &Env) -> u64 {
    env.ledger()
        .timestamp()
        .checked_add(ADMIN_TIMELOCK_SECONDS)
        .unwrap()
}

fn require_timelock_ready(env: &Env, execute_after: u64) {
    if env.ledger().timestamp() < execute_after {
        panic!("basket admin timelock not ready");
    }
}

fn enforce_max_transaction_amount(env: &Env, value: i128) {
    let max = read_max_transaction_amount(env);
    if max > 0 && value > max {
        panic!("basket transaction exceeds max size");
    }
}

fn read_holding(env: &Env, asset: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::Holding(asset.clone()))
        .unwrap_or(0)
}

fn write_holding(env: &Env, asset: &Address, amount: i128) {
    env.storage()
        .persistent()
        .set(&DataKey::Holding(asset.clone()), &amount);
}

fn apply_acquired_assets(env: &Env, assets: &Vec<Asset>, acquired: &Vec<i128>) {
    if assets.len() != acquired.len() {
        panic!("settlement returned wrong basket asset count");
    }
    for i in 0..assets.len() {
        let asset = assets.get_unchecked(i).address;
        let amount = acquired.get_unchecked(i);
        check_nonnegative(amount);
        write_holding(env, &asset, read_holding(env, &asset) + amount);
    }
}

fn proportional_redemption_amounts(
    env: &Env,
    assets: &Vec<Asset>,
    shares: i128,
    total_supply: i128,
) -> Vec<i128> {
    let mut amounts = Vec::new(env);
    for asset in assets.iter() {
        let holding = read_holding(env, &asset.address);
        amounts.push_back(holding.checked_mul(shares).unwrap() / total_supply);
    }
    amounts
}

fn holdings_for_assets(env: &Env, assets: &Vec<Asset>) -> Vec<i128> {
    let mut holdings = Vec::new(env);
    for asset in assets.iter() {
        holdings.push_back(read_holding(env, &asset.address));
    }
    holdings
}

fn target_holdings(
    env: &Env,
    assets: &Vec<Asset>,
    deposit_asset: &Address,
    total_value: i128,
    weights: &Vec<u32>,
) -> Vec<i128> {
    let mut holdings = Vec::new(env);
    let oracle = read_oracle(env);
    let deposit_price = OracleAdapterClient::new(env, &oracle)
        .price(deposit_asset)
        .price_e7;
    for i in 0..weights.len() {
        let asset = assets.get_unchecked(i).address;
        let asset_price = OracleAdapterClient::new(env, &oracle)
            .price(&asset)
            .price_e7;
        let target_value = total_value
            .checked_mul(weights.get_unchecked(i) as i128)
            .unwrap()
            / BPS_DENOMINATOR;
        let target_amount = target_value.checked_mul(deposit_price).unwrap() / asset_price;
        holdings.push_back(target_amount);
    }
    holdings
}

fn enforce_rebalance_transaction_amount(
    env: &Env,
    assets: &Vec<Asset>,
    deposit_asset: &Address,
    old_holdings: &Vec<i128>,
    new_holdings: &Vec<i128>,
) {
    let max = read_max_transaction_amount(env);
    if max == 0 {
        return;
    }

    let oracle = read_oracle(env);
    let deposit_price = OracleAdapterClient::new(env, &oracle)
        .price(deposit_asset)
        .price_e7;
    let mut moved_value = 0_i128;
    for i in 0..assets.len() {
        let old_amount = old_holdings.get_unchecked(i);
        let new_amount = new_holdings.get_unchecked(i);
        let delta = if old_amount > new_amount {
            old_amount - new_amount
        } else {
            new_amount - old_amount
        };
        if delta == 0 {
            continue;
        }
        let asset = assets.get_unchecked(i).address;
        let asset_price = OracleAdapterClient::new(env, &oracle)
            .price(&asset)
            .price_e7;
        let base_value = delta.checked_mul(asset_price).unwrap() / deposit_price;
        moved_value = moved_value.checked_add(base_value).unwrap();
    }
    enforce_max_transaction_amount(env, moved_value);
}

fn authorize_rebalance(env: &Env, caller: &Address, signers: &Vec<Address>) {
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    caller.require_auth();
    if *caller == admin {
        return;
    }

    let rebalancers = read_rebalancers(env);
    let threshold = read_rebalancer_threshold(env);
    if threshold == 0 {
        panic!("rebalance caller is not authorized");
    }
    assert_unique(signers);
    let mut count = 0_u32;
    for signer in signers.iter() {
        if !contains(&rebalancers, &signer) {
            panic!("unauthorized rebalancer signer");
        }
        if signer != *caller {
            signer.require_auth();
        }
        count += 1;
    }
    if count < threshold {
        panic!("rebalancer quorum not met");
    }
}

fn enforce_drift_bound(env: &Env, old_weights: &Vec<u32>, new_weights: &Vec<u32>) {
    let max_drift = read_max_drift_bps(env);
    for i in 0..old_weights.len() {
        let old = old_weights.get_unchecked(i);
        let new = new_weights.get_unchecked(i);
        let drift = old.abs_diff(new);
        if drift > max_drift {
            panic!("rebalance exceeds max drift per call");
        }
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
                panic!("duplicate address");
            }
        }
    }
}

fn reduce_holdings(env: &Env, assets: &Vec<Asset>, amounts: &Vec<i128>) {
    for i in 0..assets.len() {
        let asset = assets.get_unchecked(i).address;
        let amount = amounts.get_unchecked(i);
        let holding = read_holding(env, &asset);
        if amount > holding {
            panic!("basket holding underflow");
        }
        write_holding(env, &asset, holding - amount);
    }
}

fn read_position(env: &Env, holder: &Address) -> Option<Position> {
    env.storage()
        .persistent()
        .get(&DataKey::Position(holder.clone()))
}

fn write_position(env: &Env, holder: &Address, position: &Position) {
    env.storage()
        .persistent()
        .set(&DataKey::Position(holder.clone()), position);
}

fn add_position(env: &Env, holder: &Address, shares: i128, cost_per_share: i128) {
    let current = read_position(env, holder).unwrap_or(Position {
        tracked_shares: 0,
        average_cost_per_share: cost_per_share,
    });
    let new_shares = current.tracked_shares.checked_add(shares).unwrap();
    let weighted_cost = current
        .average_cost_per_share
        .checked_mul(current.tracked_shares)
        .unwrap()
        .checked_add(cost_per_share.checked_mul(shares).unwrap())
        .unwrap();
    write_position(
        env,
        holder,
        &Position {
            tracked_shares: new_shares,
            average_cost_per_share: weighted_cost / new_shares,
        },
    );
}

fn reduce_position(env: &Env, holder: &Address, shares: i128) {
    if let Some(mut position) = read_position(env, holder) {
        if shares >= position.tracked_shares {
            env.storage()
                .persistent()
                .remove(&DataKey::Position(holder.clone()));
        } else {
            position.tracked_shares -= shares;
            write_position(env, holder, &position);
        }
    }
}

fn withdrawal_fee(
    env: &Env,
    holder: &Address,
    shares: i128,
    realized_value: i128,
    fallback_cost_per_share: i128,
) -> i128 {
    let fee_bps = read_withdrawal_fee_bps(env);
    let position = read_position(env, holder).unwrap_or(Position {
        tracked_shares: 0,
        average_cost_per_share: fallback_cost_per_share,
    });
    let cost_per_share = if position.tracked_shares >= shares {
        position.average_cost_per_share
    } else {
        fallback_cost_per_share
    };
    let cost_basis = shares.checked_mul(cost_per_share).unwrap() / NAV_SCALE;
    if realized_value <= cost_basis {
        0
    } else {
        (realized_value - cost_basis)
            .checked_mul(fee_bps as i128)
            .unwrap()
            / BPS_DENOMINATOR
    }
}

fn transfer_from_basket(env: &Env, token: &Address, from: &Address, to: &Address, amount: i128) {
    authorize_contract_call(
        env,
        token.clone(),
        symbol_short!("transfer"),
        (from.clone(), MuxedAddress::from(to.clone()), amount).into_val(env),
    );
    TokenClient::new(env, token).transfer(from, &MuxedAddress::from(to.clone()), &amount);
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
    use basket_token::{BasketToken, BasketTokenClient as ShareTokenClient};
    use oracle_adapter::{OracleAdapter, OracleAdapterClient};
    use settlement::Settlement as SettlementContract;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token::{StellarAssetClient, TokenClient},
        vec, Env, MuxedAddress, String,
    };

    const UNIT: i128 = NAV_SCALE;

    struct Fixture {
        env: Env,
        creator: Address,
        depositor: Address,
        second_depositor: Address,
        third_party: Address,
        fallback_signer_a: Address,
        fallback_signer_b: Address,
        rebalancer_a: Address,
        rebalancer_b: Address,
        basket_id: Address,
        share_token_id: Address,
        settlement_id: Address,
        oracle_id: Address,
        deposit_asset: Address,
        assets: Vec<Asset>,
    }

    impl Fixture {
        fn basket(&self) -> BasketClient<'_> {
            BasketClient::new(&self.env, &self.basket_id)
        }

        fn share_token(&self) -> ShareTokenClient<'_> {
            ShareTokenClient::new(&self.env, &self.share_token_id)
        }

        fn cash(&self) -> TokenClient<'_> {
            TokenClient::new(&self.env, &self.deposit_asset)
        }

        fn cash_admin(&self) -> StellarAssetClient<'_> {
            StellarAssetClient::new(&self.env, &self.deposit_asset)
        }

        fn mint_cash(&self, to: &Address, amount: i128) {
            self.cash_admin().mint(to, &amount);
        }

        fn set_price(&self, asset: &Address, price_e7: i128) {
            OracleAdapterClient::new(&self.env, &self.oracle_id).set_fallback_price(
                asset,
                &price_e7,
                &self.env.ledger().timestamp(),
                &vec![
                    &self.env,
                    self.fallback_signer_a.clone(),
                    self.fallback_signer_b.clone(),
                ],
            );
        }
    }

    fn setup() -> Fixture {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();

        let creator = Address::generate(&env);
        let depositor = Address::generate(&env);
        let second_depositor = Address::generate(&env);
        let third_party = Address::generate(&env);
        let fallback_signer_a = Address::generate(&env);
        let fallback_signer_b = Address::generate(&env);
        let rebalancer_a = Address::generate(&env);
        let rebalancer_b = Address::generate(&env);

        let deposit_asset_contract = env.register_stellar_asset_contract_v2(creator.clone());
        let deposit_asset = deposit_asset_contract.address();
        let basket_id = env.register(Basket, ());
        let share_token_id = env.register(BasketToken, ());
        let settlement_id = env.register(SettlementContract, ());
        let oracle_id = env.register(OracleAdapter, ());

        OracleAdapterClient::new(&env, &oracle_id).initialize(
            &creator,
            &Address::generate(&env),
            &false,
            &600_u64,
            &vec![&env, fallback_signer_a.clone(), fallback_signer_b.clone()],
            &2_u32,
        );
        settlement::SettlementClient::new(&env, &settlement_id).initialize(
            &creator,
            &Address::generate(&env),
            &oracle_id,
            &500_u32,
        );

        let share_token = ShareTokenClient::new(&env, &share_token_id);
        share_token.initialize(
            &basket_id,
            &String::from_str(&env, "Sqim Test Basket"),
            &String::from_str(&env, "SQIMT"),
            &7,
        );

        let assets = vec![
            &env,
            Asset {
                address: Address::generate(&env),
            },
            Asset {
                address: Address::generate(&env),
            },
        ];
        let weights = vec![&env, 6_000_u32, 4_000_u32];

        BasketClient::new(&env, &basket_id).initialize(
            &creator,
            &String::from_str(&env, "Sqim Real Basket"),
            &assets,
            &weights,
            &share_token_id,
            &settlement_id,
            &oracle_id,
            &deposit_asset,
            &1_000_u32,
            &vec![&env, rebalancer_a.clone(), rebalancer_b.clone()],
            &2_u32,
            &2_000_u32,
            &(500 * UNIT),
        );

        let fixture = Fixture {
            env,
            creator,
            depositor,
            second_depositor,
            third_party,
            fallback_signer_a,
            fallback_signer_b,
            rebalancer_a,
            rebalancer_b,
            basket_id,
            share_token_id,
            settlement_id,
            oracle_id,
            deposit_asset,
            assets,
        };
        fixture.mint_cash(&fixture.depositor, 1_000 * UNIT);
        fixture.mint_cash(&fixture.second_depositor, 1_000 * UNIT);
        fixture.set_price(&fixture.deposit_asset, UNIT);
        for asset in fixture.assets.iter() {
            fixture.set_price(&asset.address, UNIT);
        }
        fixture
    }

    #[test]
    fn first_deposit_mints_at_initial_nav() {
        let fixture = setup();

        let minted = fixture.basket().deposit(&fixture.depositor, &(100 * UNIT));

        assert_eq!(minted, 100 * UNIT);
        assert_eq!(
            fixture.share_token().balance(&fixture.depositor),
            100 * UNIT
        );
        assert_eq!(fixture.share_token().total_supply(), 100 * UNIT);
        assert_eq!(fixture.basket().total_basket_value(), 100 * UNIT);
        assert_eq!(fixture.basket().nav(), UNIT);
        assert_eq!(
            fixture
                .basket()
                .position(&fixture.depositor)
                .average_cost_per_share,
            UNIT
        );
    }

    #[test]
    fn second_deposit_at_different_nav_mints_fewer_shares_and_reweights_basis() {
        let fixture = setup();

        fixture.basket().deposit(&fixture.depositor, &(100 * UNIT));
        fixture.mint_cash(&fixture.basket_id, 100 * UNIT);
        fixture
            .basket()
            .mark_to_market(&fixture.creator, &(200 * UNIT));

        let minted = fixture
            .basket()
            .deposit(&fixture.second_depositor, &(100 * UNIT));

        assert_eq!(minted, 50 * UNIT);
        assert_eq!(
            fixture.share_token().balance(&fixture.second_depositor),
            50 * UNIT
        );
        assert_eq!(fixture.share_token().total_supply(), 150 * UNIT);
        assert_eq!(fixture.basket().total_basket_value(), 300 * UNIT);
        assert_eq!(fixture.basket().nav(), 2 * UNIT);
        assert_eq!(
            fixture
                .basket()
                .position(&fixture.second_depositor)
                .average_cost_per_share,
            2 * UNIT
        );
    }

    #[test]
    fn partial_withdrawal_charges_fee_only_on_profit() {
        let fixture = setup();

        fixture.basket().deposit(&fixture.depositor, &(100 * UNIT));
        fixture.mint_cash(&fixture.basket_id, 100 * UNIT);
        fixture
            .basket()
            .mark_to_market(&fixture.creator, &(200 * UNIT));

        let net = fixture.basket().withdraw(&fixture.depositor, &(50 * UNIT));

        assert_eq!(net, 95 * UNIT);
        assert_eq!(fixture.cash().balance(&fixture.depositor), 995 * UNIT);
        assert_eq!(fixture.cash().balance(&fixture.creator), 5 * UNIT);
        assert_eq!(fixture.share_token().balance(&fixture.depositor), 50 * UNIT);
        assert_eq!(fixture.basket().total_basket_value(), 100 * UNIT);
    }

    #[test]
    fn full_withdrawal_closes_position() {
        let fixture = setup();

        fixture.basket().deposit(&fixture.depositor, &(100 * UNIT));
        fixture.mint_cash(&fixture.basket_id, 100 * UNIT);
        fixture
            .basket()
            .mark_to_market(&fixture.creator, &(200 * UNIT));

        let net = fixture.basket().withdraw(&fixture.depositor, &(100 * UNIT));

        assert_eq!(net, 190 * UNIT);
        assert_eq!(fixture.cash().balance(&fixture.depositor), 1_090 * UNIT);
        assert_eq!(fixture.cash().balance(&fixture.creator), 10 * UNIT);
        assert_eq!(fixture.share_token().balance(&fixture.depositor), 0);
        assert_eq!(fixture.share_token().total_supply(), 0);
        assert_eq!(
            fixture.basket().position(&fixture.depositor).tracked_shares,
            0
        );
    }

    #[test]
    fn withdrawal_at_loss_has_zero_fee() {
        let fixture = setup();

        fixture.basket().deposit(&fixture.depositor, &(100 * UNIT));
        fixture
            .basket()
            .mark_to_market(&fixture.creator, &(50 * UNIT));

        let net = fixture.basket().withdraw(&fixture.depositor, &(100 * UNIT));

        assert_eq!(net, 50 * UNIT);
        assert_eq!(fixture.cash().balance(&fixture.depositor), 950 * UNIT);
        assert_eq!(fixture.cash().balance(&fixture.creator), 0);
    }

    #[test]
    #[should_panic]
    fn unauthorized_withdrawal_attempt_fails() {
        let fixture = setup();
        fixture.basket().deposit(&fixture.depositor, &(10 * UNIT));
        fixture.env.set_auths(&[]);

        fixture.basket().withdraw(&fixture.depositor, &(1 * UNIT));
    }

    #[test]
    fn basket_token_transfers_to_third_party_outside_protocol() {
        let fixture = setup();

        fixture.basket().deposit(&fixture.depositor, &(100 * UNIT));
        fixture.share_token().transfer(
            &fixture.depositor,
            &MuxedAddress::from(fixture.third_party.clone()),
            &(40 * UNIT),
        );

        assert_eq!(fixture.share_token().balance(&fixture.depositor), 60 * UNIT);
        assert_eq!(
            fixture.share_token().balance(&fixture.third_party),
            40 * UNIT
        );

        let net = fixture
            .basket()
            .withdraw(&fixture.third_party, &(40 * UNIT));

        assert_eq!(net, 40 * UNIT);
        assert_eq!(fixture.cash().balance(&fixture.third_party), 40 * UNIT);
        assert_eq!(fixture.share_token().balance(&fixture.third_party), 0);
    }

    #[test]
    fn rebalance_shifts_weights_without_changing_supply() {
        let fixture = setup();
        fixture.basket().deposit(&fixture.depositor, &(100 * UNIT));
        let supply_before = fixture.share_token().total_supply();

        let holdings = fixture.basket().rebalance(
            &fixture.rebalancer_a,
            &vec![&fixture.env, 5_000_u32, 5_000_u32],
            &vec![
                &fixture.env,
                fixture.rebalancer_a.clone(),
                fixture.rebalancer_b.clone(),
            ],
        );

        assert_eq!(holdings.get_unchecked(0), 50 * UNIT);
        assert_eq!(holdings.get_unchecked(1), 50 * UNIT);
        assert_eq!(
            fixture.basket().target_weights_bps(),
            vec![&fixture.env, 5_000_u32, 5_000_u32]
        );
        assert_eq!(fixture.share_token().total_supply(), supply_before);
    }

    #[test]
    #[should_panic]
    fn rebalance_fails_if_caller_is_not_creator_or_authorized() {
        let fixture = setup();
        fixture.basket().deposit(&fixture.depositor, &(100 * UNIT));
        let attacker = Address::generate(&fixture.env);

        fixture.basket().rebalance(
            &attacker,
            &vec![&fixture.env, 5_000_u32, 5_000_u32],
            &vec![&fixture.env],
        );
    }

    #[test]
    #[should_panic]
    fn rebalance_fails_if_drift_exceeds_bound_even_with_valid_signers() {
        let fixture = setup();
        fixture.basket().deposit(&fixture.depositor, &(100 * UNIT));

        fixture.basket().rebalance(
            &fixture.rebalancer_a,
            &vec![&fixture.env, 3_000_u32, 7_000_u32],
            &vec![
                &fixture.env,
                fixture.rebalancer_a.clone(),
                fixture.rebalancer_b.clone(),
            ],
        );
    }

    #[test]
    #[should_panic]
    fn swap_fails_if_slippage_exceeds_tolerance() {
        let fixture = setup();
        settlement::SettlementClient::new(&fixture.env, &fixture.settlement_id)
            .set_simulated_slippage_bps(&fixture.creator, &600_u32);

        fixture.basket().deposit(&fixture.depositor, &(100 * UNIT));
    }

    #[test]
    #[should_panic]
    fn pause_halts_deposit() {
        let fixture = setup();
        fixture.basket().pause(&fixture.creator);

        fixture.basket().deposit(&fixture.depositor, &(10 * UNIT));
    }

    #[test]
    #[should_panic]
    fn pause_halts_withdraw() {
        let fixture = setup();
        fixture.basket().deposit(&fixture.depositor, &(100 * UNIT));
        fixture.basket().pause(&fixture.creator);

        fixture.basket().withdraw(&fixture.depositor, &(10 * UNIT));
    }

    #[test]
    #[should_panic]
    fn pause_halts_rebalance() {
        let fixture = setup();
        fixture.basket().deposit(&fixture.depositor, &(100 * UNIT));
        fixture.basket().pause(&fixture.creator);

        fixture.basket().rebalance(
            &fixture.rebalancer_a,
            &vec![&fixture.env, 5_000_u32, 5_000_u32],
            &vec![
                &fixture.env,
                fixture.rebalancer_a.clone(),
                fixture.rebalancer_b.clone(),
            ],
        );
    }

    #[test]
    #[should_panic]
    fn withdrawal_fee_change_cannot_execute_before_timelock() {
        let fixture = setup();

        fixture
            .basket()
            .schedule_withdrawal_fee_bps(&fixture.creator, &500_u32);
        fixture
            .basket()
            .execute_withdrawal_fee_bps(&fixture.creator);
    }

    #[test]
    fn withdrawal_fee_change_executes_after_timelock() {
        let fixture = setup();

        let ready_at = fixture
            .basket()
            .schedule_withdrawal_fee_bps(&fixture.creator, &500_u32);
        fixture.env.ledger().set_timestamp(ready_at);
        fixture
            .basket()
            .execute_withdrawal_fee_bps(&fixture.creator);

        assert_eq!(fixture.basket().withdrawal_fee_bps(), 500_u32);
    }

    #[test]
    #[should_panic]
    fn max_transaction_size_blocks_large_deposit() {
        let fixture = setup();

        fixture.basket().deposit(&fixture.depositor, &(600 * UNIT));
    }

    #[test]
    #[should_panic]
    fn max_transaction_size_blocks_large_withdrawal() {
        let fixture = setup();
        fixture.basket().deposit(&fixture.depositor, &(100 * UNIT));
        let ready_at = fixture
            .basket()
            .schedule_max_transaction_amount(&fixture.creator, &(50 * UNIT));
        fixture.env.ledger().set_timestamp(ready_at);
        fixture
            .basket()
            .execute_max_transaction_amount(&fixture.creator);

        fixture.basket().withdraw(&fixture.depositor, &(60 * UNIT));
    }

    #[test]
    #[should_panic]
    fn max_transaction_size_blocks_large_rebalance() {
        let fixture = setup();
        fixture.basket().deposit(&fixture.depositor, &(100 * UNIT));
        let ready_at = fixture
            .basket()
            .schedule_max_transaction_amount(&fixture.creator, &(10 * UNIT));
        fixture.env.ledger().set_timestamp(ready_at);
        fixture
            .basket()
            .execute_max_transaction_amount(&fixture.creator);

        fixture.basket().rebalance(
            &fixture.rebalancer_a,
            &vec![&fixture.env, 5_000_u32, 5_000_u32],
            &vec![
                &fixture.env,
                fixture.rebalancer_a.clone(),
                fixture.rebalancer_b.clone(),
            ],
        );
    }

    #[test]
    fn rebalancer_set_change_executes_only_after_timelock() {
        let fixture = setup();
        let new_rebalancer_a = Address::generate(&fixture.env);
        let new_rebalancer_b = Address::generate(&fixture.env);

        let ready_at = fixture.basket().schedule_rebalancers(
            &fixture.creator,
            &vec![
                &fixture.env,
                new_rebalancer_a.clone(),
                new_rebalancer_b.clone(),
            ],
            &2_u32,
        );
        fixture.env.ledger().set_timestamp(ready_at);
        fixture.basket().execute_rebalancers(&fixture.creator);
        fixture.set_price(&fixture.deposit_asset, UNIT);
        for asset in fixture.assets.iter() {
            fixture.set_price(&asset.address, UNIT);
        }

        fixture.basket().deposit(&fixture.depositor, &(100 * UNIT));
        fixture.basket().rebalance(
            &new_rebalancer_a,
            &vec![&fixture.env, 5_000_u32, 5_000_u32],
            &vec![
                &fixture.env,
                new_rebalancer_a.clone(),
                new_rebalancer_b.clone(),
            ],
        );
    }

    #[test]
    #[should_panic]
    fn withdraw_swap_fails_if_slippage_exceeds_tolerance() {
        let fixture = setup();
        fixture.basket().deposit(&fixture.depositor, &(100 * UNIT));
        settlement::SettlementClient::new(&fixture.env, &fixture.settlement_id)
            .set_simulated_slippage_bps(&fixture.creator, &600_u32);

        fixture.basket().withdraw(&fixture.depositor, &(10 * UNIT));
    }

    #[test]
    #[should_panic]
    fn rebalance_swap_fails_if_slippage_exceeds_tolerance() {
        let fixture = setup();
        fixture.basket().deposit(&fixture.depositor, &(100 * UNIT));
        settlement::SettlementClient::new(&fixture.env, &fixture.settlement_id)
            .set_simulated_slippage_bps(&fixture.creator, &600_u32);

        fixture.basket().rebalance(
            &fixture.rebalancer_a,
            &vec![&fixture.env, 5_000_u32, 5_000_u32],
            &vec![
                &fixture.env,
                fixture.rebalancer_a.clone(),
                fixture.rebalancer_b.clone(),
            ],
        );
    }
}
