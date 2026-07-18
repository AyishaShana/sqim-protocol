#![no_std]

use soroban_sdk::{
    auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation},
    contract, contractclient, contracterror, contractevent, contractimpl, contracttype,
    panic_with_error, symbol_short,
    token::TokenClient,
    vec, Address, Env, IntoVal, MuxedAddress, String, Val, Vec,
};

const BPS_DENOMINATOR: i128 = 10_000;
const NAV_SCALE: i128 = 10_000_000;
const ADMIN_TIMELOCK_SECONDS: u64 = 86_400;
const MIN_REBALANCE_SWAP_AMOUNT: i128 = 100;
const MIN_REDEMPTION_SWAP_AMOUNT: i128 = 2;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1001,
    InvalidFee = 1002,
    InvalidDrift = 1003,
    Unauthorized = 1004,
    NotInitialized = 1005,
    TimelockNotReady = 1006,
    Paused = 1007,
    MaxTransactionExceeded = 1008,
    InvalidAmount = 1009,
    InvalidWeights = 1010,
    InvalidQuorum = 1011,
    SettlementResultInvalid = 1012,
    InsufficientBalance = 1013,
    ArithmeticOverflow = 1014,
    RebalanceSupplyChanged = 1015,
    OraclePriceInvalid = 1016,
    DuplicateAddress = 1017,
    CostBasisInvalid = 1018,
    DriftExceeded = 1019,
    QuorumNotMet = 1020,
    UnauthorizedSigner = 1021,
    UnauthorizedToken = 1022,
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
pub struct Position {
    pub tracked_shares: i128,
    pub average_cost_per_share: i128,
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
    ) -> InvestResult;

    fn redeem(
        env: Env,
        basket: Address,
        payout_asset: Address,
        assets: Vec<Asset>,
        amounts: Vec<i128>,
        expected_outputs: Vec<i128>,
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

#[contractevent(topics = ["basis"], data_format = "vec")]
#[derive(Clone)]
pub struct BasisEvent {
    #[topic]
    pub from: Address,
    #[topic]
    pub to: Address,
    pub amount: i128,
    pub cost_per_share: i128,
}

#[contractevent(topics = ["pause"], data_format = "single-value")]
#[derive(Clone)]
pub struct PauseEvent {
    #[topic]
    pub admin: Address,
    pub paused: bool,
}

#[contractevent(topics = ["deposit"], data_format = "vec")]
#[derive(Clone)]
pub struct DepositEvent {
    #[topic]
    pub depositor: Address,
    #[topic]
    pub deposit_asset: Address,
    pub amount: i128,
    pub shares: i128,
    pub nav: i128,
    pub aum: i128,
}

#[contractevent(topics = ["withdraw"], data_format = "vec")]
#[derive(Clone)]
pub struct WithdrawEvent {
    #[topic]
    pub holder: Address,
    #[topic]
    pub payout_asset: Address,
    pub shares: i128,
    pub payout: i128,
    pub fee: i128,
    pub nav: i128,
    pub aum: i128,
}

#[contractevent(topics = ["rebalance"], data_format = "vec")]
#[derive(Clone)]
pub struct RebalanceEvent {
    #[topic]
    pub caller: Address,
    pub weights_bps: Vec<u32>,
    pub nav: i128,
    pub aum: i128,
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
    pub fn initialize(env: Env, config: BasketConfig) {
        if env.storage().instance().has(&DataKey::Initialized) {
            panic_with_error!(&env, Error::AlreadyInitialized);
        }
        config.admin.require_auth();
        validate_weights(&env, &config.assets, &config.target_weights_bps);
        if config.withdrawal_fee_bps > 10_000 {
            panic_with_error!(&env, Error::InvalidFee);
        }
        if config.max_drift_bps > 10_000 {
            panic_with_error!(&env, Error::InvalidDrift);
        }
        check_nonnegative(&env, config.max_transaction_amount);
        validate_rebalancer_quorum(&env, &config.rebalancers, config.rebalancer_threshold);

        env.storage().instance().set(&DataKey::Admin, &config.admin);
        env.storage().instance().set(&DataKey::Name, &config.name);
        env.storage()
            .instance()
            .set(&DataKey::Assets, &config.assets);
        env.storage()
            .instance()
            .set(&DataKey::TargetWeights, &config.target_weights_bps);
        env.storage()
            .instance()
            .set(&DataKey::ShareToken, &config.share_token);
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
        env.storage()
            .instance()
            .set(&DataKey::TotalBasketValue, &0_i128);
        env.storage().instance().set(&DataKey::Paused, &false);
        env.storage().instance().set(&DataKey::Initialized, &true);
    }

    pub fn name(env: Env) -> String {
        env.storage()
            .instance()
            .get(&DataKey::Name)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized))
    }

    pub fn assets(env: Env) -> Vec<Asset> {
        read_assets(&env)
    }

    pub fn target_weights_bps(env: Env) -> Vec<u32> {
        env.storage()
            .instance()
            .get(&DataKey::TargetWeights)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized))
    }

    pub fn share_token(env: Env) -> Address {
        read_share_token(&env)
    }

    pub fn total_basket_value(env: Env) -> i128 {
        basket_value(&env)
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

    pub fn on_share_transfer(env: Env, token: Address, from: Address, to: Address, amount: i128) {
        check_nonnegative(&env, amount);
        if token != read_share_token(&env) {
            panic_with_error!(&env, Error::UnauthorizedToken);
        }
        token.require_auth();
        let cost_per_share = move_position(&env, &from, &to, amount);
        BasisEvent {
            from,
            to,
            amount,
            cost_per_share,
        }
        .publish(&env);
    }

    pub fn pause(env: Env, admin: Address) {
        require_admin(&env, &admin);
        env.storage().instance().set(&DataKey::Paused, &true);
        PauseEvent {
            admin,
            paused: true,
        }
        .publish(&env);
    }

    pub fn unpause(env: Env, admin: Address) {
        require_admin(&env, &admin);
        env.storage().instance().set(&DataKey::Paused, &false);
        PauseEvent {
            admin,
            paused: false,
        }
        .publish(&env);
    }

    pub fn schedule_withdrawal_fee_bps(env: Env, admin: Address, value: u32) -> u64 {
        if value > 10_000 {
            panic_with_error!(&env, Error::InvalidFee);
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
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
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
            panic_with_error!(&env, Error::InvalidDrift);
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
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
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
        validate_rebalancer_quorum(&env, &rebalancers, threshold);
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
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
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
        check_nonnegative(&env, value);
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
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
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
        check_positive(&env, amount);
        enforce_max_transaction_amount(&env, amount);
        depositor.require_auth();

        let basket_address = env.current_contract_address();
        let deposit_asset = read_deposit_asset(&env);
        let assets = read_assets(&env);
        let share_token = read_share_token(&env);
        let total_supply_before = BasketShareTokenClient::new(&env, &share_token).total_supply();
        let current_holdings = if total_supply_before == 0 {
            Vec::new(&env)
        } else {
            holdings_for_assets(&env, &assets)
        };

        TokenClient::new(&env, &deposit_asset).transfer(
            &depositor,
            MuxedAddress::from(basket_address.clone()),
            &amount,
        );

        let weights = read_weights(&env);
        let settlement = read_settlement(&env);
        transfer_from_basket(&env, &deposit_asset, &basket_address, &settlement, amount);
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
        let invest_result = SettlementClient::new(&env, &settlement).invest(
            &basket_address,
            &deposit_asset,
            &amount,
            &assets,
            &weights,
        );
        apply_acquired_assets(&env, &assets, &invest_result.acquired);
        if invest_result.asset_prices_e7.len() != assets.len()
            || invest_result.deposit_price_e7 <= 0
        {
            panic_with_error!(&env, Error::SettlementResultInvalid);
        }

        let current_total_value = if total_supply_before == 0 {
            0
        } else {
            value_from_snapshot(
                &env,
                &current_holdings,
                &invest_result.asset_prices_e7,
                invest_result.deposit_price_e7,
            )
        };
        let current_nav = if total_supply_before == 0 {
            NAV_SCALE
        } else {
            current_total_value
                .checked_mul(NAV_SCALE)
                .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow))
                / total_supply_before
        };
        let shares_to_mint = amount
            .checked_mul(NAV_SCALE)
            .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow))
            / current_nav;
        check_positive(&env, shares_to_mint);

        authorize_contract_call(
            &env,
            share_token.clone(),
            symbol_short!("mint"),
            (depositor.clone(), shares_to_mint).into_val(&env),
        );
        BasketShareTokenClient::new(&env, &share_token).mint(&depositor, &shares_to_mint);

        add_position(&env, &depositor, shares_to_mint, current_nav);
        let acquired_value = if total_supply_before == 0 {
            amount
        } else {
            value_from_snapshot(
                &env,
                &invest_result.acquired,
                &invest_result.asset_prices_e7,
                invest_result.deposit_price_e7,
            )
        };
        let total_value_after = current_total_value
            .checked_add(acquired_value)
            .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow));
        let total_supply_after = total_supply_before
            .checked_add(shares_to_mint)
            .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow));
        let nav_after = total_value_after
            .checked_mul(NAV_SCALE)
            .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow))
            / total_supply_after;

        DepositEvent {
            depositor,
            deposit_asset,
            amount,
            shares: shares_to_mint,
            nav: nav_after,
            aum: total_value_after,
        }
        .publish(&env);
        shares_to_mint
    }

    pub fn withdraw(env: Env, holder: Address, basket_token_amount: i128) -> i128 {
        ensure_not_paused(&env);
        check_positive(&env, basket_token_amount);
        holder.require_auth();

        let share_token = read_share_token(&env);
        let share_client = BasketShareTokenClient::new(&env, &share_token);
        let holder_balance = share_client.balance(&holder);
        if holder_balance < basket_token_amount {
            panic_with_error!(&env, Error::InsufficientBalance);
        }

        let total_supply = share_client.total_supply();
        check_positive(&env, total_supply);
        let assets = read_assets(&env);
        let current_holdings = holdings_for_assets(&env, &assets);
        let asset_prices = prices_for_assets(&env, &assets);
        let payout_asset = read_deposit_asset(&env);
        let payout_price = deposit_price_from_snapshot(&env, &assets, &payout_asset, &asset_prices);
        let current_total_value =
            value_from_snapshot(&env, &current_holdings, &asset_prices, payout_price);
        let current_nav = current_total_value
            .checked_mul(NAV_SCALE)
            .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow))
            / total_supply;
        let gross_value = basket_token_amount
            .checked_mul(current_total_value)
            .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow))
            / total_supply;
        check_positive(&env, gross_value);
        enforce_max_transaction_amount(&env, gross_value);

        let redeemed_amounts = proportional_redemption_amounts(
            &env,
            &assets,
            &payout_asset,
            basket_token_amount,
            total_supply,
        );
        let expected_outputs = redemption_outputs_from_snapshot(
            &env,
            &assets,
            &payout_asset,
            &redeemed_amounts,
            &asset_prices,
            payout_price,
        );

        let basket_address = env.current_contract_address();
        let settlement = read_settlement(&env);
        for i in 0..assets.len() {
            let amount = redeemed_amounts.get_unchecked(i);
            if amount > 0 {
                transfer_from_basket(
                    &env,
                    &assets.get_unchecked(i).address,
                    &basket_address,
                    &settlement,
                    amount,
                );
            }
        }
        authorize_contract_call(
            &env,
            settlement.clone(),
            symbol_short!("redeem"),
            (
                basket_address.clone(),
                payout_asset.clone(),
                assets.clone(),
                redeemed_amounts.clone(),
                expected_outputs.clone(),
            )
                .into_val(&env),
        );
        let realized_value = SettlementClient::new(&env, &settlement).redeem(
            &basket_address,
            &payout_asset,
            &assets,
            &redeemed_amounts,
            &expected_outputs,
        );
        share_client.burn(&holder, &basket_token_amount);

        let fee = withdrawal_fee(
            &env,
            &holder,
            basket_token_amount,
            realized_value,
            current_nav,
        );
        let net = realized_value - fee;
        check_nonnegative(&env, net);
        reduce_position(&env, &holder, basket_token_amount);

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        if fee > 0 {
            transfer_from_basket(&env, &payout_asset, &basket_address, &admin, fee);
        }
        if net > 0 {
            transfer_from_basket(&env, &payout_asset, &basket_address, &holder, net);
        }
        let total_supply_after = total_supply
            .checked_sub(basket_token_amount)
            .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow));
        let total_value_after = current_total_value
            .checked_sub(gross_value)
            .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow));
        let nav_after = if total_supply_after == 0 {
            NAV_SCALE
        } else {
            total_value_after
                .checked_mul(NAV_SCALE)
                .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow))
                / total_supply_after
        };

        WithdrawEvent {
            holder,
            payout_asset,
            shares: basket_token_amount,
            payout: net,
            fee,
            nav: nav_after,
            aum: total_value_after,
        }
        .publish(&env);
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
        validate_weights(&env, &assets, &new_weights_bps);
        authorize_rebalance(&env, &caller, &rebalancer_signers);

        let old_weights = read_weights(&env);
        enforce_drift_bound(&env, &old_weights, &new_weights_bps);

        let share_token = read_share_token(&env);
        let total_supply_before = BasketShareTokenClient::new(&env, &share_token).total_supply();
        let old_holdings = holdings_for_assets(&env, &assets);
        let deposit_asset = read_deposit_asset(&env);
        let asset_prices = prices_for_assets(&env, &assets);
        let deposit_price =
            deposit_price_from_snapshot(&env, &assets, &deposit_asset, &asset_prices);
        let total_value = value_from_snapshot(&env, &old_holdings, &asset_prices, deposit_price);
        let new_holdings = target_holdings_from_snapshot(
            &env,
            &asset_prices,
            deposit_price,
            total_value,
            &new_weights_bps,
        );
        enforce_rebalance_transaction_amount_from_snapshot(
            &env,
            &old_holdings,
            &new_holdings,
            &asset_prices,
            deposit_price,
        );
        let settlement = read_settlement(&env);
        let basket_address = env.current_contract_address();
        for i in 0..assets.len() {
            let old_amount = old_holdings.get_unchecked(i);
            let new_amount = new_holdings.get_unchecked(i);
            if old_amount > new_amount && old_amount - new_amount >= MIN_REBALANCE_SWAP_AMOUNT {
                transfer_from_basket(
                    &env,
                    &assets.get_unchecked(i).address,
                    &basket_address,
                    &settlement,
                    old_amount - new_amount,
                );
            }
        }

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
            panic_with_error!(&env, Error::SettlementResultInvalid);
        }

        env.storage()
            .instance()
            .set(&DataKey::TargetWeights, &new_weights_bps);

        let total_supply_after = BasketShareTokenClient::new(&env, &share_token).total_supply();
        if total_supply_after != total_supply_before {
            panic_with_error!(&env, Error::RebalanceSupplyChanged);
        }

        let total_value_after =
            value_from_snapshot(&env, &updated_holdings, &asset_prices, deposit_price);
        let nav_after = if total_supply_after == 0 {
            NAV_SCALE
        } else {
            total_value_after
                .checked_mul(NAV_SCALE)
                .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow))
                / total_supply_after
        };
        RebalanceEvent {
            caller,
            weights_bps: new_weights_bps,
            nav: nav_after,
            aum: total_value_after,
        }
        .publish(&env);
        updated_holdings
    }
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

fn check_positive(env: &Env, amount: i128) {
    if amount <= 0 {
        panic_with_error!(env, Error::InvalidAmount);
    }
}

fn check_nonnegative(env: &Env, amount: i128) {
    if amount < 0 {
        panic_with_error!(env, Error::InvalidAmount);
    }
}

fn read_assets(env: &Env) -> Vec<Asset> {
    env.storage()
        .instance()
        .get(&DataKey::Assets)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
}

fn read_weights(env: &Env) -> Vec<u32> {
    env.storage()
        .instance()
        .get(&DataKey::TargetWeights)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
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

fn read_withdrawal_fee_bps(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::WithdrawalFeeBps)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
}

fn read_deposit_asset(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::DepositAsset)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
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

fn read_share_token(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::ShareToken)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
}

fn read_total_value(env: &Env) -> i128 {
    basket_value(env)
}

fn nav(env: &Env) -> i128 {
    let share_token = read_share_token(env);
    let supply = BasketShareTokenClient::new(env, &share_token).total_supply();
    if supply == 0 {
        NAV_SCALE
    } else {
        read_total_value(env)
            .checked_mul(NAV_SCALE)
            .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow))
            / supply
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
        panic_with_error!(env, Error::Paused);
    }
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

fn require_timelock_ready(env: &Env, execute_after: u64) {
    if env.ledger().timestamp() < execute_after {
        panic_with_error!(env, Error::TimelockNotReady);
    }
}

fn enforce_max_transaction_amount(env: &Env, value: i128) {
    let max = read_max_transaction_amount(env);
    if max > 0 && value > max {
        panic_with_error!(env, Error::MaxTransactionExceeded);
    }
}

fn read_holding(env: &Env, asset: &Address) -> i128 {
    TokenClient::new(env, asset).balance(&env.current_contract_address())
}

fn apply_acquired_assets(env: &Env, assets: &Vec<Asset>, acquired: &Vec<i128>) {
    if assets.len() != acquired.len() {
        panic_with_error!(env, Error::SettlementResultInvalid);
    }
    for i in 0..assets.len() {
        let amount = acquired.get_unchecked(i);
        check_nonnegative(env, amount);
    }
}

fn proportional_redemption_amounts(
    env: &Env,
    assets: &Vec<Asset>,
    payout_asset: &Address,
    shares: i128,
    total_supply: i128,
) -> Vec<i128> {
    let mut amounts = Vec::new(env);
    for asset in assets.iter() {
        let holding = read_holding(env, &asset.address);
        let amount = holding
            .checked_mul(shares)
            .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow))
            / total_supply;
        if asset.address != *payout_asset && amount < MIN_REDEMPTION_SWAP_AMOUNT {
            amounts.push_back(0);
        } else {
            amounts.push_back(amount);
        }
    }
    amounts
}

fn redemption_outputs_from_snapshot(
    env: &Env,
    assets: &Vec<Asset>,
    payout_asset: &Address,
    amounts: &Vec<i128>,
    prices: &Vec<i128>,
    payout_price: i128,
) -> Vec<i128> {
    if assets.len() != amounts.len() || assets.len() != prices.len() || payout_price <= 0 {
        panic_with_error!(env, Error::SettlementResultInvalid);
    }
    let mut outputs = Vec::new(env);
    for i in 0..assets.len() {
        let amount = amounts.get_unchecked(i);
        let expected = if assets.get_unchecked(i).address == *payout_asset {
            amount
        } else {
            amount
                .checked_mul(prices.get_unchecked(i))
                .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow))
                / payout_price
        };
        outputs.push_back(expected);
    }
    outputs
}

fn holdings_for_assets(env: &Env, assets: &Vec<Asset>) -> Vec<i128> {
    let mut holdings = Vec::new(env);
    for asset in assets.iter() {
        holdings.push_back(read_holding(env, &asset.address));
    }
    holdings
}

fn prices_for_assets(env: &Env, assets: &Vec<Asset>) -> Vec<i128> {
    let oracle = read_oracle(env);
    let oracle_client = OracleAdapterClient::new(env, &oracle);
    let mut prices = Vec::new(env);
    for asset in assets.iter() {
        let price = oracle_client.price(&asset.address).price_e7;
        if price <= 0 {
            panic_with_error!(env, Error::OraclePriceInvalid);
        }
        prices.push_back(price);
    }
    prices
}

fn deposit_price_from_snapshot(
    env: &Env,
    assets: &Vec<Asset>,
    deposit_asset: &Address,
    prices: &Vec<i128>,
) -> i128 {
    for i in 0..assets.len() {
        if assets.get_unchecked(i).address == *deposit_asset {
            return prices.get_unchecked(i);
        }
    }
    let price = OracleAdapterClient::new(env, &read_oracle(env))
        .price(deposit_asset)
        .price_e7;
    if price <= 0 {
        panic_with_error!(env, Error::OraclePriceInvalid);
    }
    price
}

fn value_from_snapshot(
    env: &Env,
    holdings: &Vec<i128>,
    prices: &Vec<i128>,
    deposit_price: i128,
) -> i128 {
    if holdings.len() != prices.len() || deposit_price <= 0 {
        panic_with_error!(env, Error::SettlementResultInvalid);
    }
    let mut total = 0_i128;
    for i in 0..holdings.len() {
        total = total
            .checked_add(
                holdings
                    .get_unchecked(i)
                    .checked_mul(prices.get_unchecked(i))
                    .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow))
                    / deposit_price,
            )
            .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow));
    }
    total
}

fn target_holdings_from_snapshot(
    env: &Env,
    prices: &Vec<i128>,
    deposit_price: i128,
    total_value: i128,
    weights: &Vec<u32>,
) -> Vec<i128> {
    if prices.len() != weights.len() {
        panic_with_error!(env, Error::SettlementResultInvalid);
    }
    let mut holdings = Vec::new(env);
    for i in 0..weights.len() {
        let target_value = total_value
            .checked_mul(weights.get_unchecked(i) as i128)
            .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow))
            / BPS_DENOMINATOR;
        holdings.push_back(
            target_value
                .checked_mul(deposit_price)
                .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow))
                / prices.get_unchecked(i),
        );
    }
    holdings
}

fn enforce_rebalance_transaction_amount_from_snapshot(
    env: &Env,
    old_holdings: &Vec<i128>,
    new_holdings: &Vec<i128>,
    prices: &Vec<i128>,
    deposit_price: i128,
) {
    let max = read_max_transaction_amount(env);
    if max == 0 {
        return;
    }
    let mut moved_value = 0_i128;
    for i in 0..old_holdings.len() {
        let old_amount = old_holdings.get_unchecked(i);
        let new_amount = new_holdings.get_unchecked(i);
        let delta = if old_amount > new_amount {
            old_amount - new_amount
        } else {
            new_amount - old_amount
        };
        moved_value = moved_value
            .checked_add(
                delta
                    .checked_mul(prices.get_unchecked(i))
                    .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow))
                    / deposit_price,
            )
            .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow));
    }
    enforce_max_transaction_amount(env, moved_value);
}

fn basket_value(env: &Env) -> i128 {
    let assets = read_assets(env);
    let deposit_asset = read_deposit_asset(env);
    let oracle = read_oracle(env);
    let oracle_client = OracleAdapterClient::new(env, &oracle);
    let deposit_price = oracle_client.price(&deposit_asset).price_e7;
    if deposit_price <= 0 {
        panic_with_error!(env, Error::OraclePriceInvalid);
    }

    let mut total = 0_i128;
    for asset in assets.iter() {
        let holding = read_holding(env, &asset.address);
        if holding == 0 {
            continue;
        }
        let asset_price = oracle_client.price(&asset.address).price_e7;
        if asset_price <= 0 {
            panic_with_error!(env, Error::OraclePriceInvalid);
        }
        total = total
            .checked_add(
                holding
                    .checked_mul(asset_price)
                    .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow))
                    / deposit_price,
            )
            .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow));
    }
    total
}

fn authorize_rebalance(env: &Env, caller: &Address, signers: &Vec<Address>) {
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized));
    caller.require_auth();
    if *caller == admin {
        return;
    }

    let rebalancers = read_rebalancers(env);
    let threshold = read_rebalancer_threshold(env);
    if threshold == 0 {
        panic_with_error!(env, Error::Unauthorized);
    }
    assert_unique(env, signers);
    let mut count = 0_u32;
    for signer in signers.iter() {
        if !contains(&rebalancers, &signer) {
            panic_with_error!(env, Error::UnauthorizedSigner);
        }
        if signer != *caller {
            signer.require_auth();
        }
        count += 1;
    }
    if count < threshold {
        panic_with_error!(env, Error::QuorumNotMet);
    }
}

fn enforce_drift_bound(env: &Env, old_weights: &Vec<u32>, new_weights: &Vec<u32>) {
    let max_drift = read_max_drift_bps(env);
    for i in 0..old_weights.len() {
        let old = old_weights.get_unchecked(i);
        let new = new_weights.get_unchecked(i);
        let drift = old.abs_diff(new);
        if drift > max_drift {
            panic_with_error!(env, Error::DriftExceeded);
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
    let new_shares = current
        .tracked_shares
        .checked_add(shares)
        .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow));
    let weighted_cost = current
        .average_cost_per_share
        .checked_mul(current.tracked_shares)
        .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow))
        .checked_add(
            cost_per_share
                .checked_mul(shares)
                .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow)),
        )
        .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow));
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

fn move_position(env: &Env, from: &Address, to: &Address, shares: i128) -> i128 {
    if shares == 0 || from == to {
        return nav(env);
    }
    let mut source =
        read_position(env, from).unwrap_or_else(|| panic_with_error!(env, Error::CostBasisInvalid));
    if source.tracked_shares < shares {
        panic_with_error!(env, Error::CostBasisInvalid);
    }

    let cost_per_share = source.average_cost_per_share;
    source.tracked_shares -= shares;
    if source.tracked_shares == 0 {
        env.storage()
            .persistent()
            .remove(&DataKey::Position(from.clone()));
    } else {
        write_position(env, from, &source);
    }
    add_position(env, to, shares, cost_per_share);
    cost_per_share
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
    let cost_basis = shares
        .checked_mul(cost_per_share)
        .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow))
        / NAV_SCALE;
    if realized_value <= cost_basis {
        0
    } else {
        (realized_value - cost_basis)
            .checked_mul(fee_bps as i128)
            .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow))
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
    use basket_token::{BasketToken, BasketTokenClient as ShareTokenClient};
    use oracle_adapter::{OracleAdapter, OracleAdapterClient};
    use settlement::Settlement as SettlementContract;
    use soroban_sdk::{
        contract, contractimpl, contracttype,
        testutils::{Address as _, Ledger},
        token::{StellarAssetClient, TokenClient},
        vec, Env, MuxedAddress, String,
    };

    const UNIT: i128 = NAV_SCALE;

    #[derive(Clone)]
    #[contracttype]
    enum MockRouterKey {
        Rate(Address, Address),
    }

    #[contract]
    struct MockRouter;

    #[contractimpl]
    impl MockRouter {
        pub fn set_rate(
            env: Env,
            input: Address,
            output: Address,
            numerator: i128,
            denominator: i128,
        ) {
            if numerator <= 0 || denominator <= 0 {
                panic!("invalid mock router rate");
            }
            env.storage().instance().set(
                &MockRouterKey::Rate(input, output),
                &(numerator, denominator),
            );
        }

        pub fn router_pair_for(env: Env, _token_a: Address, _token_b: Address) -> Address {
            env.current_contract_address()
        }

        pub fn swap_exact_tokens_for_tokens(
            env: Env,
            amount_in: i128,
            amount_out_min: i128,
            path: Vec<Address>,
            to: Address,
            _deadline: u64,
        ) -> Vec<i128> {
            to.require_auth();
            let input = path.get_unchecked(0);
            let output = path.get_unchecked(path.len() - 1);
            let (numerator, denominator): (i128, i128) = env
                .storage()
                .instance()
                .get(&MockRouterKey::Rate(input.clone(), output.clone()))
                .unwrap_or((1, 1));
            let amount_out = amount_in.checked_mul(numerator).unwrap() / denominator;
            if amount_out < amount_out_min {
                panic!("mock router insufficient output");
            }
            let router = env.current_contract_address();
            TokenClient::new(&env, &input).transfer(
                &to,
                MuxedAddress::from(router.clone()),
                &amount_in,
            );
            transfer_from_basket(&env, &output, &router, &to, amount_out);
            vec![&env, amount_in, amount_out]
        }
    }

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
        router_id: Address,
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

        fn set_rate(&self, input: &Address, output: &Address, numerator: i128, denominator: i128) {
            MockRouterClient::new(&self.env, &self.router_id).set_rate(
                input,
                output,
                &numerator,
                &denominator,
            );
        }

        fn set_uniform_asset_price(&self, price_e7: i128) {
            for asset in self.assets.iter() {
                self.set_price(&asset.address, price_e7);
                if price_e7 == 2 * UNIT {
                    self.set_rate(&self.deposit_asset, &asset.address, 1, 2);
                    self.set_rate(&asset.address, &self.deposit_asset, 2, 1);
                } else if price_e7 * 2 == UNIT {
                    self.set_rate(&self.deposit_asset, &asset.address, 2, 1);
                    self.set_rate(&asset.address, &self.deposit_asset, 1, 2);
                } else {
                    self.set_rate(&self.deposit_asset, &asset.address, 1, 1);
                    self.set_rate(&asset.address, &self.deposit_asset, 1, 1);
                }
            }
        }
    }

    fn setup() -> Fixture {
        let env = Env::default();
        env.cost_estimate().budget().reset_unlimited();
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
        let router_id = env.register(MockRouter, ());
        let oracle_id = env.register(OracleAdapter, ());

        OracleAdapterClient::new(&env, &oracle_id).initialize(
            &creator,
            &Address::generate(&env),
            &false,
            &600_u64,
            &vec![&env, fallback_signer_a.clone(), fallback_signer_b.clone()],
            &2_u32,
        );
        settlement::SettlementClient::new(&env, &settlement_id)
            .initialize(&creator, &router_id, &oracle_id, &500_u32);

        let share_token = ShareTokenClient::new(&env, &share_token_id);
        share_token.initialize(
            &basket_id,
            &String::from_str(&env, "Sqim Test Basket"),
            &String::from_str(&env, "SQIMT"),
            &7,
        );

        let asset_a = env
            .register_stellar_asset_contract_v2(creator.clone())
            .address();
        let asset_b = env
            .register_stellar_asset_contract_v2(creator.clone())
            .address();
        let assets = vec![
            &env,
            Asset {
                address: asset_a.clone(),
            },
            Asset {
                address: asset_b.clone(),
            },
        ];
        let weights = vec![&env, 6_000_u32, 4_000_u32];

        BasketClient::new(&env, &basket_id).initialize(&BasketConfig {
            admin: creator.clone(),
            name: String::from_str(&env, "Sqim Real Basket"),
            assets: assets.clone(),
            target_weights_bps: weights,
            share_token: share_token_id.clone(),
            settlement: settlement_id.clone(),
            oracle: oracle_id.clone(),
            deposit_asset: deposit_asset.clone(),
            withdrawal_fee_bps: 1_000,
            rebalancers: vec![&env, rebalancer_a.clone(), rebalancer_b.clone()],
            rebalancer_threshold: 2,
            max_drift_bps: 2_000,
            max_transaction_amount: 500 * UNIT,
        });

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
            router_id,
            oracle_id,
            deposit_asset,
            assets,
        };
        fixture.mint_cash(&fixture.depositor, 1_000 * UNIT);
        fixture.mint_cash(&fixture.second_depositor, 1_000 * UNIT);
        fixture.mint_cash(&fixture.router_id, 10_000 * UNIT);
        StellarAssetClient::new(&fixture.env, &asset_a).mint(&fixture.router_id, &(10_000 * UNIT));
        StellarAssetClient::new(&fixture.env, &asset_b).mint(&fixture.router_id, &(10_000 * UNIT));
        fixture.set_price(&fixture.deposit_asset, UNIT);
        for asset in fixture.assets.iter() {
            fixture.set_price(&asset.address, UNIT);
            fixture.set_rate(&fixture.deposit_asset, &asset.address, 1, 1);
            fixture.set_rate(&asset.address, &fixture.deposit_asset, 1, 1);
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
        fixture.set_uniform_asset_price(2 * UNIT);

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
        fixture.set_uniform_asset_price(2 * UNIT);

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
        fixture.set_uniform_asset_price(2 * UNIT);

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
        fixture.set_uniform_asset_price(UNIT / 2);

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

        fixture.basket().withdraw(&fixture.depositor, &UNIT);
    }

    #[test]
    fn basket_token_transfers_to_third_party_outside_protocol() {
        let fixture = setup();

        fixture.basket().deposit(&fixture.depositor, &(100 * UNIT));
        fixture.share_token().transfer(
            &fixture.depositor,
            MuxedAddress::from(fixture.third_party.clone()),
            &(40 * UNIT),
        );

        assert_eq!(fixture.share_token().balance(&fixture.depositor), 60 * UNIT);
        assert_eq!(
            fixture.share_token().balance(&fixture.third_party),
            40 * UNIT
        );
        assert_eq!(
            fixture
                .basket()
                .position(&fixture.third_party)
                .average_cost_per_share,
            UNIT
        );
        assert_eq!(
            fixture
                .basket()
                .position(&fixture.third_party)
                .tracked_shares,
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
        fixture.set_rate(
            &fixture.deposit_asset,
            &fixture.assets.get_unchecked(0).address,
            9,
            10,
        );

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
        for asset in fixture.assets.iter() {
            fixture.set_rate(&asset.address, &fixture.deposit_asset, 9, 10);
        }

        fixture.basket().withdraw(&fixture.depositor, &(10 * UNIT));
    }

    #[test]
    #[should_panic]
    fn rebalance_swap_fails_if_slippage_exceeds_tolerance() {
        let fixture = setup();
        fixture.basket().deposit(&fixture.depositor, &(100 * UNIT));
        fixture.set_rate(
            &fixture.assets.get_unchecked(0).address,
            &fixture.deposit_asset,
            9,
            10,
        );

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
