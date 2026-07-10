#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Env, MuxedAddress, String,
};

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
            panic!("basket token already initialized");
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Name, &name);
        env.storage().instance().set(&DataKey::Symbol, &symbol);
        env.storage().instance().set(&DataKey::Decimals, &decimals);
        env.storage().instance().set(&DataKey::TotalSupply, &0_i128);
        env.storage().instance().set(&DataKey::Initialized, &true);
    }

    pub fn admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).unwrap()
    }

    pub fn name(env: Env) -> String {
        env.storage().instance().get(&DataKey::Name).unwrap()
    }

    pub fn symbol(env: Env) -> String {
        env.storage().instance().get(&DataKey::Symbol).unwrap()
    }

    pub fn decimals(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::Decimals).unwrap()
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
        check_nonnegative(amount);
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

        env.events().publish(
            (symbol_short!("approve"), from, spender),
            (amount, expiration_ledger),
        );
    }

    pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
        check_nonnegative(amount);
        from.require_auth();

        let to_address = to.address();
        move_balance(&env, &from, &to_address, amount);
        env.events()
            .publish((symbol_short!("transfer"), from, to_address), amount);
    }

    pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
        check_nonnegative(amount);
        spender.require_auth();
        spend_allowance(&env, &from, &spender, amount);

        move_balance(&env, &from, &to, amount);
        env.events()
            .publish((symbol_short!("transfer"), from, to), amount);
    }

    pub fn burn(env: Env, from: Address, amount: i128) {
        check_nonnegative(amount);
        from.require_auth();

        spend_balance(&env, &from, amount);
        write_total_supply(&env, read_total_supply(&env) - amount);
        env.events().publish((symbol_short!("burn"), from), amount);
    }

    pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
        check_nonnegative(amount);
        spender.require_auth();
        spend_allowance(&env, &from, &spender, amount);

        spend_balance(&env, &from, amount);
        write_total_supply(&env, read_total_supply(&env) - amount);
        env.events().publish((symbol_short!("burn"), from), amount);
    }

    pub fn mint(env: Env, to: Address, amount: i128) {
        check_nonnegative(amount);
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        write_balance(&env, &to, read_balance(&env, &to) + amount);
        write_total_supply(&env, read_total_supply(&env) + amount);
        env.events().publish((symbol_short!("mint"), to), amount);
    }
}

fn check_nonnegative(amount: i128) {
    if amount < 0 {
        panic!("negative basket token amount");
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
        panic!("negative basket token supply");
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
        panic!("insufficient basket token balance");
    }
    write_balance(env, id, balance - amount);
}

fn move_balance(env: &Env, from: &Address, to: &Address, amount: i128) {
    spend_balance(env, from, amount);
    write_balance(env, to, read_balance(env, to) + amount);
}

fn spend_allowance(env: &Env, from: &Address, spender: &Address, amount: i128) {
    let key = DataKey::Allowance(from.clone(), spender.clone());
    let allowance = env.storage().temporary().get(&key).unwrap_or(0);
    if allowance < amount {
        panic!("insufficient basket token allowance");
    }
    env.storage().temporary().set(&key, &(allowance - amount));
}
