# Sqim Protocol

**Own Portfolios. Not Positions.**

Sqim is a permissionless Stellar/Soroban protocol for creating diversified on-chain asset baskets. A user deposits once and receives a single liquid basket token that represents proportional exposure to a curated set of underlying assets.

Frontend:

- https://sqim-protocol-stellar.vercel.app

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

### Off-Chain Services

- `services/indexer` polls Soroban RPC contract events and writes normalized basket activity into Postgres.
- `services/api` exposes frontend REST reads from Postgres, with Redis used for hot-path NAV/AUM cache keys.
- `services/relayer` runs reviewed rule-based rebalance strategies only, dry-run by default, and rejects live mode without an M-of-N signer quorum.
- `docker-compose.yml` runs Postgres, Redis, indexer, API, and relayer for local development.

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

- React + TypeScript app for basket discovery, deposit, withdraw, portfolio, and creator workflows.
- Freighter wallet integration for client-side signing.
- Basket explorer backed by the API service, not direct chain polling for basket lists or history.
- UI copy centered on diversified exposure and one-token basket ownership.
- Static Vercel routing config for the built app.

## What Is Pending

### Production Rebalancer Hardening

The v1 off-chain stack includes a lean rule-based relayer. Production hardening still needs:

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
  src/             React + TypeScript frontend
  assets/          hero and product media
  package.json     frontend scripts and wallet dependencies
  assets/          hero video

services/
  api/             REST API for frontend reads
  indexer/         Soroban RPC event indexer
  relayer/         reviewed rule-based rebalance process
  db/schema.sql    Postgres schema

vercel.json        static deployment routing
docker-compose.yml local service stack
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
npm install --prefix site
npm run dev
```

Preview the frontend on port `8080` with the lightweight mock API on `8081`:

```powershell
npm --prefix site run mock-api
npm --prefix site run dev:8080
```

Frontend environment:

```powershell
$env:VITE_SQIM_API_URL="http://localhost:8081"
$env:VITE_SOROBAN_RPC_URL="https://soroban-testnet.stellar.org"
$env:VITE_SOROBAN_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
$env:VITE_SQIM_ENABLE_CONTRACT_WRITES="false"
```

Wallet support:

- v1 ships with Freighter signing through `@stellar/freighter-api`.
- Stellar Wallets Kit is the planned abstraction for additional wallets such as xBull, Albedo, Hana, Lobstr through WalletConnect, Rabet, Ledger, and Trezor.
- The frontend never handles private keys. It builds transactions, sends XDR to the connected wallet, and submits only signed transactions.
- Local preview baskets are read-only by default. Enable writes only after the API returns a factory-created initialized basket with `investable: true`, and set `VITE_SQIM_INVESTABLE_BASKET_IDS` plus `VITE_SQIM_ENABLE_CONTRACT_WRITES=true`.

Run the local off-chain stack:

```powershell
docker compose up --build
```

Switch RPC networks without code changes:

```powershell
$env:SOROBAN_RPC_URL="https://soroban-testnet.stellar.org"
$env:SOROBAN_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
docker compose up --build
```

API examples:

```powershell
curl http://localhost:8081/health
curl http://localhost:8081/baskets
curl http://localhost:8081/baskets/<basket-contract-id>/history
curl http://localhost:8081/baskets/<basket-contract-id>/metrics
```

Run the Postgres-backed integration test:

```powershell
cd services
$env:SQIM_TEST_DATABASE_URL="postgres://sqim:sqim@localhost:5432/sqim?sslmode=disable"
go test -tags=integration ./integration
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

## Testnet Contracts

Deployed on Stellar testnet under the `ayisha` development identity.

| Contract | Testnet Contract ID | Explorer |
| --- | --- | --- |
| `factory` | `CA74FW7KGZQ2N7X3DO5CRDX7KMGX5LKA5GNIZ7WHX7ZFZAR54NI5MAXM` | [View](https://stellar.expert/explorer/testnet/contract/CA74FW7KGZQ2N7X3DO5CRDX7KMGX5LKA5GNIZ7WHX7ZFZAR54NI5MAXM) |
| `basket` | `CC7XPFDPZEMRRHY3NJ7WPB5RDMWIXZMHNULKQALJGIWTXUXDK7JVPG4A` | [View](https://stellar.expert/explorer/testnet/contract/CC7XPFDPZEMRRHY3NJ7WPB5RDMWIXZMHNULKQALJGIWTXUXDK7JVPG4A) |
| `basket_token` | `CD3V4GJ3QJPR6JAWEGJNAEGZ4JRLSGEWAMP2TZIYNO2JXMHTZNBBE3KL` | [View](https://stellar.expert/explorer/testnet/contract/CD3V4GJ3QJPR6JAWEGJNAEGZ4JRLSGEWAMP2TZIYNO2JXMHTZNBBE3KL) |
| `settlement` | `CDJSQKCPKM5RACK2P5VHW4KC4AEIBO2SHKH5FOGR2YB2P2DBOIAS6D5A` | [View](https://stellar.expert/explorer/testnet/contract/CDJSQKCPKM5RACK2P5VHW4KC4AEIBO2SHKH5FOGR2YB2P2DBOIAS6D5A) |
| `oracle_adapter` | `CDYAEPQS4ITHYNOSXZ4UIF2XX4HL6HOJBEO7TVFDUHJMVAOIBJ3CYP7C` | [View](https://stellar.expert/explorer/testnet/contract/CDYAEPQS4ITHYNOSXZ4UIF2XX4HL6HOJBEO7TVFDUHJMVAOIBJ3CYP7C) |

WASM hashes:

| Contract | WASM Hash |
| --- | --- |
| `factory` | `a89c74c49879941b7cd3d1dac1d2f57037e7de6174a2d91b6b5631f3da4589c6` |
| `basket` | `6929446d0f76a62f578617a5fb0f2d4e65e8315093757d71de15a2d5acb5923d` |
| `basket_token` | `06dc92eb01c63173dadcd2a9211b529446231993bfb589401b83f934b1ca089d` |
| `settlement` | `8e7e3fcac382fb683dc07381cb99b59aba1a1de0634f6be4a9151eacebb309ed` |
| `oracle_adapter` | `d328a7774ac82fc425f7d91336f899a4f4943bf6054936b751cdf570f9fdb160` |

## Deployment Notes

The contracts have been built, tested, and deployed on Stellar testnet. The public README intentionally includes only the Ayisha deployment identity, public contract IDs, and public explorer links.

The frontend is configured for Vercel as a static site through `vercel.json`.
