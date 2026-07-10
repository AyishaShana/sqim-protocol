# Sqim Protocol

**Own Portfolios. Not Positions.**

Sqim is a permissionless Stellar/Soroban protocol for creating diversified on-chain asset baskets. A user deposits once and receives a single liquid basket token that represents proportional exposure to a curated set of underlying assets.

Frontend:

- https://sqim-protocol.vercel.app

## Overview

Sqim turns a basket of assets into a single programmable on-chain asset.

Instead of users manually buying, tracking, and rebalancing many individual positions, a Sqim basket wraps diversified exposure into one transferable basket token. The token is designed to be composable from day one: it can move between wallets and, later, through Soroban DEX/AMM infrastructure.

## What Is Done

### Smart Contracts

- Rust/Soroban Cargo workspace.
- `basket_token` contract for transferable basket shares.
- `factory` contract for deploying basket/token pairs.
- `basket` contract for deposits, withdrawals, NAV views, holdings, and cost basis.
- `oracle_adapter` contract for primary oracle reads and M-of-N fallback pricing.
- `settlement` contract boundary with oracle-based slippage checks.

### Basket Mechanics

- Creator-defined basket name, assets, and weights.
- Weight validation.
- NAV-based minting and burning.
- Per-holder average cost basis.
- Creator withdrawal fee applied only on realized profit.
- Basket token transferability outside the protocol.

### Rebalancing Guards

- Creator-gated rebalance path.
- Authorized rebalancer quorum path.
- Max drift per rebalance call.
- Total basket token supply invariant during rebalance.
- Settlement slippage checks against oracle quote.

### Oracle And Trust Model

- Primary Soroban-native oracle integration path.
- Stale price rejection.
- Fallback price path requiring M-of-N authorized signers.
- No single fallback oracle key assumption.

### Frontend

- Premium static landing page.
- Video-backed hero card.
- Sqim positioning copy.
- Join X CTA.
- Waitlist modal.
- Static Vercel routing config.

## What Is Pending

### Production Rebalancer

The contract supports authorized rebalancers, but the off-chain/automation side still needs to be built:

- strategy runner
- scheduled rebalance bot
- M-of-N signer coordination flow
- monitoring for drift thresholds
- transaction simulation before submission
- alerting for failed or rejected rebalance attempts

### Real Settlement Execution

The current settlement crate is a guarded simulation boundary. The next step is wiring real routes:

- Soroswap router integration
- pool path selection
- multi-hop route support
- exact-in/exact-out handling
- real liquidity seeding for testnet demos
- slippage checks against actual realized swap output

### Oracle Hardening

- Dual-source oracle validation.
- Oracle source failover policy.
- Better typed errors for stale/missing feeds.
- Production-grade signer rotation for fallback publishers.

### Frontend And Waitlist

- Persist waitlist emails through a backend or form provider.
- Add production analytics only after privacy review.
- Add legal/disclaimer copy before public launch.
- Replace local-only waitlist storage.

### Contract Polish

- Replace deprecated event publishing with typed contract events.
- Add more integration tests around multi-asset deposits and withdrawals.
- Add fuzz/property tests for weight and cost-basis math.
- Add deployment scripts for repeatable testnet/mainnet releases.

## Repository Layout

```text
contracts/
  basket/          basket logic, NAV, deposits, withdrawals, rebalancing
  basket_token/    transferable basket share token
  factory/         basket and basket-token deployment flow
  oracle_adapter/  primary oracle and fallback price quorum
  settlement/      settlement boundary and slippage checks

site/
  index.html       landing page
  styles.css       visual design
  app.js           waitlist modal behavior
  assets/          hero video

vercel.json        static deployment routing
```

## Contract Crates

### `basket_token`

Transferable basket share token. It supports transfers, approvals, minting, burning, balances, supply, and metadata.

### `factory`

Deploys basket contracts and their corresponding basket share tokens from creator-defined names, assets, and target weights.

### `basket`

Handles the main user and basket state flows:

- deposit
- withdraw
- NAV
- holdings
- per-holder average cost basis
- profit-only creator withdrawal fee
- rebalancing
- rebalancer quorum checks
- max-drift checks

### `oracle_adapter`

Reads prices from a primary Soroban-native oracle and supports a fallback path that requires M-of-N authorized signers.

### `settlement`

Enforces oracle-based slippage checks for deposit, withdraw, rebalance, and swap paths.

## Development

Install the Rust and Stellar toolchain:

```powershell
rustup update stable
rustup target add wasm32v1-none
cargo install stellar-cli --locked --version 27.0.0
```

Run tests:

```powershell
cargo test --workspace
```

Build contracts:

```powershell
stellar contract build
```

Preview frontend locally:

```powershell
python -m http.server 4177 --directory site
```

## Tests Covered

- first deposit
- second deposit at a different NAV
- partial withdrawal
- full withdrawal
- withdrawal at a loss with zero fee
- unauthorized withdrawal failure
- basket token transferability
- rebalance success path
- unauthorized rebalance failure
- max-drift rebalance failure
- slippage failure
- stale oracle price failure without fallback quorum

## Deployment Notes

The contracts have been tested on Stellar testnet. The current public branch avoids embedding personal local paths, local key aliases, or machine-specific data in the README.

The frontend is configured for Vercel as a static site through `vercel.json`.
