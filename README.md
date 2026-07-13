# Sqim Protocol

**Own Portfolios. Not Positions.**

Sqim is a permissionless Stellar/Soroban protocol for creating diversified on-chain asset baskets. A user deposits once and receives a single liquid basket token that represents proportional exposure to a curated set of underlying assets.

Frontend:

- https://sqim-protocol-stellar.vercel.app

## Overview

Sqim turns a basket of assets into a single programmable on-chain asset.

Instead of users manually buying, tracking, and rebalancing many individual positions, a Sqim basket wraps diversified exposure into one transferable basket token. The token is designed to be composable from day one: it can move between wallets and, later, through Soroban DEX/AMM infrastructure.

## Production Readiness

Current status: **testnet preview, not mainnet production**.

The frontend, mock API, and hardened contracts build and pass local smoke checks. The current contracts include pause controls, timelocked sensitive admin changes, max transaction-size guards, M-of-N fallback oracle enforcement, M-of-N rebalancer enforcement, and slippage checks across deposit, withdraw, and rebalance paths.

Before any mainnet launch, Sqim still needs an external professional smart-contract audit, real Soroswap settlement integration, real testnet liquidity validation, typed event cleanup, and repeatable deployment scripts. See `THREATS.md` for the current risk register.

Latest verification on July 13, 2026:

- Stellar CLI `27.0.0` verified from `~/.cargo/bin/stellar.exe`, with the `testnet` network profile available. The older Program Files CLI on this Windows machine is `23.1.4` and cannot parse the current hardened basket/factory WASM specs.
- Frontend route smoke passed on `http://localhost:8080`.
- API route smoke passed on `http://localhost:8081`.
- Frontend tests, production build, Soroban contract tests, and release WASM build passed.
- Fresh hardened contracts deployed on Stellar testnet and manually exercised through deposit, external basket-token transfer, and third-party withdrawal.
- Go service tests and Docker Compose validation require Go and Docker to be installed in the local environment.

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
- Convert the manual hardened testnet deployment flow into a repeatable deployment script.

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

Run the local smoke suite:

```powershell
npm run smoke
```

The smoke script checks:

- frontend route on `http://localhost:8080`
- mock/API routes on `http://localhost:8081`
- frontend tests and production build
- Soroban contract tests and WASM build
- optional Go and Docker checks when those tools are installed

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
- pause blocking deposit, withdraw, and rebalance
- timelocked withdrawal-fee and rebalancer-set changes
- max transaction-size guards
- withdrawal and rebalance slippage guard regressions
- timelocked settlement slippage-cap changes
- timelocked oracle fallback signer/quorum changes
- on-chain fallback oracle quorum enforcement

## Testnet Contracts

Fresh hardened preview contracts deployed on Stellar testnet under the `ayisha` development identity on July 13, 2026. This is still a testnet preview, not a mainnet launch approval.

| Contract | Testnet Contract ID | Explorer |
| --- | --- | --- |
| `factory` | `CBYWTMUFK6DXO4CN4QZASWXAK7BXGJLPWDQNA3CNBOMRCX7GUGTNNKPZ` | [View](https://stellar.expert/explorer/testnet/contract/CBYWTMUFK6DXO4CN4QZASWXAK7BXGJLPWDQNA3CNBOMRCX7GUGTNNKPZ) |
| `basket` | `CABCGGFYGPWYNRPJIXFN6YHGER7YHY4CH4GWHQZUHAFEO7A6EJNS64VZ` | [View](https://stellar.expert/explorer/testnet/contract/CABCGGFYGPWYNRPJIXFN6YHGER7YHY4CH4GWHQZUHAFEO7A6EJNS64VZ) |
| `basket_token` | `CARGKEM34YZ4DCNMSSLMOPMIAAJGAF5CKGAAMJWV7E2AI5QFFGPGYEBM` | [View](https://stellar.expert/explorer/testnet/contract/CARGKEM34YZ4DCNMSSLMOPMIAAJGAF5CKGAAMJWV7E2AI5QFFGPGYEBM) |
| `settlement` | `CDNKWO64BEB4GJ4EAVSIHE3IVCGYI25UZK6DYD3CGSSRJBJGN63N3K7U` | [View](https://stellar.expert/explorer/testnet/contract/CDNKWO64BEB4GJ4EAVSIHE3IVCGYI25UZK6DYD3CGSSRJBJGN63N3K7U) |
| `oracle_adapter` | `CDMZIC6FKPT6B6LYPAZVXBV2COP5BZSAM4VIIY67OGGGVPNBW7QWAKNR` | [View](https://stellar.expert/explorer/testnet/contract/CDMZIC6FKPT6B6LYPAZVXBV2COP5BZSAM4VIIY67OGGGVPNBW7QWAKNR) |
| `mock_deposit_asset` | `CASNUWD2Z4RCUUY4LZEO7XRKHGMNRR55K7BMMUUH6PVIMYZF4IMU5RS4` | [View](https://stellar.expert/explorer/testnet/contract/CASNUWD2Z4RCUUY4LZEO7XRKHGMNRR55K7BMMUUH6PVIMYZF4IMU5RS4) |

Manual testnet proof:

| Flow | Result | Transaction |
| --- | --- | --- |
| Deposit `10000000` mock asset into basket | Minted `10000000` basket tokens | [View](https://stellar.expert/explorer/testnet/tx/b608fa1aedef563e71f9ec49d3a101f2f62341c9b80bc22dc368ebfb6c893af7) |
| Transfer `4000000` basket tokens to third party | Transfer event emitted by basket token | [View](https://stellar.expert/explorer/testnet/tx/b8d55581d89f658ccc9b0c9542ef9c318e8e792ffc91f65beb28f5757eb05636) |
| Withdraw `2000000` basket tokens from third-party holder | Returned `2000000` mock asset, fee `0` | [View](https://stellar.expert/explorer/testnet/tx/cdb90bc129705eeedbac3979768273f3a308f6ae2d103d2ac5ae332db97d1aed) |

Current hardened local WASM hashes:

| Contract | WASM Hash |
| --- | --- |
| `factory` | `57b6a0771da160747c4ba8182354b69701b1bcb820188320cfc45902678ca213` |
| `basket` | `7bb0afef51a8410ede6f9f3c241521abf2ff5e1f358ba9ecfd6ef2a2e23bfc43` |
| `basket_token` | `3aecf7efc53ac4098af30e06d96c6583dc5584983526358473e89a6e1c4cd08d` |
| `settlement` | `569ff92714f3c8dec30305f5c1a287c7ddab73d82998294a1eb0be1c095582fc` |
| `oracle_adapter` | `ac24d7a2d51562f8b0f3e1af9100566ddd2bbb5b182e193f161ae6c224aaabcd` |

## Deployment Notes

The contracts have been built and tested locally against the testnet-oriented configuration. Do not treat the legacy preview contract IDs as production-safe hardened deployments. Mainnet deployment is intentionally out of scope until audit completion.

The frontend is configured for Vercel as a static site through `vercel.json`.
