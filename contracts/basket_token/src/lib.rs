#![no_std]

use soroban_sdk::{
    contract, contractclient, contracterror, contractevent, contractimpl, contracttype,
    panic_with_error, Address, Env, MuxedAddress, String,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 2001,
    NotInitialized = 2002,
    InvalidAmount = 2003,
    InsufficientBalance = 2004,
    InsufficientAllowance = 2005,
    ArithmeticOverflow = 2006,
}

#[contractevent(topics = ["approve"], data_format = "vec")]
#[derive(Clone)]
pub struct ApproveEvent {
    #[topic]
    pub from: Address,
    #[topic]
    pub spender: Address,
    pub amount: i128,
    pub expiration_ledger: u32,
}

#[contractevent(topics = ["transfer"], data_format = "single-value")]
#[derive(Clone)]
pub struct TransferEvent {
    #[topic]
    pub from: Address,
    #[topic]
    pub to: Address,
    pub amount: i128,
}

#[contractevent(topics = ["burn"], data_format = "single-value")]
#[derive(Clone)]
pub struct BurnEvent {
    #[topic]
    pub from: Address,
    pub amount: i128,
}

#[contractevent(topics = ["mint"], data_format = "single-value")]
#[derive(Clone)]
pub struct MintEvent {
    #[topic]
    pub to: Address,
    pub amount: i128,
}

#[contractclient(name = "BasketCostBasisClient")]
pub trait BasketCostBasis {
    fn on_share_transfer(env: Env, token: Address, from: Address, to: Address, amount: i128);
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Allowance(Address, Address),
    Balance(Address),
    Decimals,
    Initialized,
    Name,
    Symbol,
    TotalSupply,
}

#[contract]
pub struct BasketToken;

#[contractimpl]
impl BasketToken {
    pub fn initialize(env: Env, admin: Address, name: String, symbol: String, decimals: u32) {
        if env.storage().instance().has(&DataKey::Initialized) {
            panic_with_error!(&env, Error::AlreadyInitialized);
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Name, &name);
        env.storage().instance().set(&DataKey::Symbol, &symbol);
        env.storage().instance().set(&DataKey::Decimals, &decimals);
        env.storage().instance().set(&DataKey::TotalSupply, &0_i128);
        env.storage().instance().set(&DataKey::Initialized, &true);
    }

    pub fn admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized))
    }

    pub fn name(env: Env) -> String {
        env.storage()
            .instance()
            .get(&DataKey::Name)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized))
    }

    pub fn symbol(env: Env) -> String {
        env.storage()
            .instance()
            .get(&DataKey::Symbol)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized))
    }

    pub fn decimals(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::Decimals)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized))
    }

    pub fn balance(env: Env, id: Address) -> i128 {
        read_balance(&env, &id)
    }

    pub fn total_supply(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::TotalSupply)
            .unwrap_or(0)
    }

    pub fn allowance(env: Env, from: Address, spender: Address) -> i128 {
        env.storage()
            .temporary()
            .get(&DataKey::Allowance(from, spender))
            .unwrap_or(0)
    }

    pub fn approve(
        env: Env,
        from: Address,
        spender: Address,
        amount: i128,
        expiration_ledger: u32,
    ) {
        check_nonnegative(&env, amount);
        from.require_auth();

        let key = DataKey::Allowance(from.clone(), spender.clone());
        if amount == 0 {
            env.storage().temporary().remove(&key);
        } else {
            env.storage().temporary().set(&key, &amount);
            env.storage()
                .temporary()
                .extend_ttl(&key, expiration_ledger, expiration_ledger);
        }

        ApproveEvent {
            from,
            spender,
            amount,
            expiration_ledger,
        }
        .publish(&env);
    }

    pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
        check_nonnegative(&env, amount);
        from.require_auth();

        let to_address = to.address();
        notify_cost_basis(&env, &from, &to_address, amount);
        move_balance(&env, &from, &to_address, amount);
        TransferEvent {
            from,
            to: to_address,
            amount,
        }
        .publish(&env);
    }

    pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
        check_nonnegative(&env, amount);
        spender.require_auth();
        spend_allowance(&env, &from, &spender, amount);

        notify_cost_basis(&env, &from, &to, amount);
        move_balance(&env, &from, &to, amount);
        TransferEvent { from, to, amount }.publish(&env);
    }

    pub fn burn(env: Env, from: Address, amount: i128) {
        check_nonnegative(&env, amount);
        from.require_auth();

        spend_balance(&env, &from, amount);
        write_total_supply(&env, read_total_supply(&env) - amount);
        BurnEvent { from, amount }.publish(&env);
    }

    pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
        check_nonnegative(&env, amount);
        spender.require_auth();
        spend_allowance(&env, &from, &spender, amount);

        spend_balance(&env, &from, amount);
        write_total_supply(&env, read_total_supply(&env) - amount);
        BurnEvent { from, amount }.publish(&env);
    }

    pub fn mint(env: Env, to: Address, amount: i128) {
        check_nonnegative(&env, amount);
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        admin.require_auth();

        write_balance(
            &env,
            &to,
            read_balance(&env, &to)
                .checked_add(amount)
                .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow)),
        );
        write_total_supply(
            &env,
            read_total_supply(&env)
                .checked_add(amount)
                .unwrap_or_else(|| panic_with_error!(&env, Error::ArithmeticOverflow)),
        );
        MintEvent { to, amount }.publish(&env);
    }
}

fn check_nonnegative(env: &Env, amount: i128) {
    if amount < 0 {
        panic_with_error!(env, Error::InvalidAmount);
    }
}

fn read_balance(env: &Env, id: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::Balance(id.clone()))
        .unwrap_or(0)
}

fn read_total_supply(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::TotalSupply)
        .unwrap_or(0)
}

fn write_total_supply(env: &Env, amount: i128) {
    if amount < 0 {
        panic_with_error!(env, Error::InvalidAmount);
    }
    env.storage().instance().set(&DataKey::TotalSupply, &amount);
}

fn write_balance(env: &Env, id: &Address, amount: i128) {
    let key = DataKey::Balance(id.clone());
    if amount == 0 {
        env.storage().persistent().remove(&key);
    } else {
        env.storage().persistent().set(&key, &amount);
    }
}

fn spend_balance(env: &Env, id: &Address, amount: i128) {
    let balance = read_balance(env, id);
    if balance < amount {
        panic_with_error!(env, Error::InsufficientBalance);
    }
    write_balance(env, id, balance - amount);
}

fn move_balance(env: &Env, from: &Address, to: &Address, amount: i128) {
    spend_balance(env, from, amount);
    write_balance(
        env,
        to,
        read_balance(env, to)
            .checked_add(amount)
            .unwrap_or_else(|| panic_with_error!(env, Error::ArithmeticOverflow)),
    );
}

fn notify_cost_basis(env: &Env, from: &Address, to: &Address, amount: i128) {
    if amount == 0 || from == to {
        return;
    }
    let basket: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized));
    BasketCostBasisClient::new(env, &basket).on_share_transfer(
        &env.current_contract_address(),
        from,
        to,
        &amount,
    );
}

fn spend_allowance(env: &Env, from: &Address, spender: &Address, amount: i128) {
    let key = DataKey::Allowance(from.clone(), spender.clone());
    let allowance = env.storage().temporary().get(&key).unwrap_or(0);
    if allowance < amount {
        panic_with_error!(env, Error::InsufficientAllowance);
    }
    env.storage().temporary().set(&key, &(allowance - amount));
}
