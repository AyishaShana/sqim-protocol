# Sqim

**Own Portfolios. Not Positions.**

Sqim is a permissionless Stellar protocol for diversified on-chain asset baskets. A user deposits once and receives one liquid, transferable token representing proportional exposure to a creator-defined set of assets.

> **TESTNET ONLY**
>
> This repository and its deployed contracts use valueless test assets. Sqim is unaudited, is not approved for mainnet, and must not receive real funds.

| Resource | Link |
| --- | --- |
| Public testnet app | [sqim-protocol-stellar-sigma.vercel.app](https://sqim-protocol-stellar-sigma.vercel.app) |
| Active contracts | [DEPLOYMENTS.md](DEPLOYMENTS.md) |
| Final verification | [SMOKE_TEST_REPORT.md](SMOKE_TEST_REPORT.md) |
| Readiness verdict | [PRODUCTION_READINESS.md](PRODUCTION_READINESS.md) |
| Threat model | [THREATS.md](THREATS.md) |
| Pool evidence | [POOLS.md](POOLS.md) |

## Status

**Testnet production deployment: NOT READY.** The contracts, public read-only application, and CI are live, but real Freighter browser evidence, Docker/Postgres integration evidence, public backend hosting, preferred-domain ownership, and an independent audit remain open. The exact blockers are maintained in [`PRODUCTION_READINESS.md`](PRODUCTION_READINESS.md).

## What is implemented

### Basket protocol

- Creator-defined basket names, asset sets, target weights, fees, drift limits, transaction limits, and rebalancer quorum.
- Factory deployment of one basket contract and one transferable SEP-41-style basket token per basket.
- Read-only NAV and AUM views based on oracle-valued constituent balances.
- Deposits that mint shares at current NAV and route capital into target constituents.
- Withdrawals that burn shares, unwind proportional constituents, and charge creator fees only on realized profit.
- Per-holder weighted average cost basis, including basis movement when basket tokens transfer.
- Creator or on-chain M-of-N rebalancer authorization.
- Pausing, max transaction size, bounded per-call drift, and timelocked risk-setting changes.
- Typed contract errors and typed events across basket, token, factory, settlement, and oracle contracts.

### Real settlement

- Soroswap testnet router integration with single-hop and configured multi-hop routes.
- Exact-in execution for deposit and rebalance paths.
- Exact-out execution for withdrawal requirements.
- Oracle-relative minimum/maximum amounts on every trade path.
- Downstream router rejection mapped to typed Sqim errors without a silent simulation fallback.
- Testnet XLM/WETH, XLM/WBTC, and XLM/WSOL pools documented in [`POOLS.md`](POOLS.md).

### Oracle model

- Reflector testnet oracle as the primary source.
- Configured maximum price age and explicit stale/missing-price failure.
- On-chain fallback submissions requiring M-of-N authorized signatures.
- No single fallback key can publish a valid price unilaterally.

### Off-chain stack

- Go indexer for factory, deposit, withdrawal, transfer, and rebalance events.
- Postgres tables for basket configuration, events, NAV history, creator profiles, and preferences.
- Go REST API for discovery, details, portfolios, profiles, historical series, and backtests.
- Redis cache for current NAV/AUM, including `as_of_ledger`, timestamp, and source metadata.
- Rule-based relayer supporting calendar and drift strategies through an M-of-N signer set.
- Historical data/backtesting service with Parquet-oriented storage and explicit simulated-data labels.
- Fixture-ID guards that reject test basket patterns in non-test database state.

### Product application

- React and TypeScript application in [`app`](app), separate from the removed legacy preview.
- Freighter and Rabet wallet adapters; all signing remains client-side.
- Basket explorer with visual composition, NAV, AUM, and freshness context.
- Basket detail with deposit, withdraw, fee preview, transaction lifecycle, and history.
- Portfolio and liquidity views for diversified positions and transferable basket shares.
- Multi-step basket creation: assets, weights, identity, review, and signed factory deployment.
- Manual percentage entry, sliders, equal-weight normalization, and live 100% validation.
- Visual before/after rebalance editor, per-asset drift diagnostics, quote preview, and authorization gating.
- Creator profiles, signature-protected editing, and rebalance-notification preferences.
- Mechanical rebalance suggestions labeled **algorithmic suggestion, not investment advice**.
- Separate presentation of live on-chain NAV history and simulated historical performance.
- Persistent TESTNET ONLY banner and a Risks page linked from every route.
- Route-level code splitting and a fail-closed startup contract health check.

## Architecture

```text
app (React/TypeScript)
  |-- client-side wallet signing ----> Stellar testnet / Soroban
  |-- basket reads -------------------> API
                                        |
services/indexer --> Postgres <---------+--> services/api --> Redis
services/relayer --> M-of-N rebalance --+--> basket --> settlement --> Soroswap
services/backtester --> historical files +--> API
```

The API/indexer is the intended source of truth for discovery and event history. The public Go backend is not yet hosted, so production readiness remains blocked even though the frontend's startup health check reads contract metadata directly to prevent address drift.

## Workspace

```text
contracts/
  basket/          Deposit, withdrawal, NAV, cost basis, rebalance, safety controls
  basket_token/    Transferable basket shares
  factory/         Basket and token deployment, creator lookup
  settlement/      Soroswap routes and oracle-protected execution
  oracle_adapter/  Reflector reads and M-of-N fallback prices
services/
  indexer/         Soroban event ingestion
  api/             REST API
  relayer/         Rule-based authorized rebalance process
  backtester/      Historical price ingestion and simulation
  internal/        Shared config, database, cache, event, and HTTP packages
app/               Product application
config/            Canonical testnet deployment manifest
scripts/           Deployment and multisignature helpers
reports/history/   Superseded smoke-test reports
```

## Contract flows

### Create

`factory.create_basket` validates exact 10,000 basis-point weights, deploys a basket and transferable token, initializes both, and records creator-to-basket lookup state.

### Deposit

1. The depositor authorizes the transaction.
2. The basket checks pause state and maximum transaction size.
3. Current NAV determines shares to mint.
4. Settlement executes oracle-protected exact-in swaps into constituents.
5. The token mints transferable shares and cost basis is updated.

### Withdraw

1. The holder authorizes their own withdrawal.
2. Shares and proportional constituent balances are calculated with checked arithmetic.
3. Settlement unwinds constituents using oracle-protected routes.
4. Profit is measured against that holder's average cost basis.
5. Creator fee is charged only when realized profit is positive.
6. Shares burn and net XLM returns to the holder.

### Rebalance

1. The creator authorizes directly, or configured rebalancers satisfy the on-chain quorum.
2. Every asset's requested change is checked against max drift per call.
3. Required sells and buys execute through settlement with oracle slippage protection.
4. Target weights update only after successful swaps.
5. Basket token supply remains unchanged.

## Error model

Contracts expose stable typed ranges so clients can render actionable messages:

| Contract | Range | Examples |
| --- | --- | --- |
| Basket | `1001-1022` | paused, amount limit, drift exceeded, quorum not met |
| Basket token | `2001-2006` | unauthorized, invalid amount, insufficient balance |
| Factory | `3001-3010` | invalid weights, invalid config, initialization state |
| Settlement | `4001-4014` | missing route, slippage exceeded, timelock state |
| Oracle adapter | `5001-5012` | stale price, unavailable price, invalid quorum |

## Local development

### Requirements

- Rust stable with `wasm32v1-none`
- Stellar CLI 23 or newer
- Go matching [`services/go.mod`](services/go.mod)
- Node.js 20 or newer
- Docker Desktop for the full local service stack

Copy environment templates without committing local values:

```powershell
Copy-Item .env.testnet.example .env
Copy-Item app/.env.example app/.env.local
```

Start the complete service stack:

```powershell
docker compose up --build
```

Start the application:

```powershell
npm --prefix app ci
npm --prefix app run dev
```

Default local endpoints:

| Service | URL |
| --- | --- |
| App | `http://localhost:8080` |
| API | `http://localhost:8081` |
| Postgres | `localhost:5432` |
| Redis | `localhost:6379` |

## Verification

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
stellar contract build

Push-Location services
gofmt -w .
go vet ./...
go test ./...
go test -tags=integration ./integration
Pop-Location

npm ci
npm run check
npm test
npm run build
```

The integration-tagged Go test requires a reachable Postgres database. A skipped integration test is not a production-readiness pass.

## Deployment

Contracts, testnet only:

```powershell
.\scripts\deploy-testnet.ps1
```

Frontend:

```powershell
.\scripts\deploy-frontend.ps1
```

Both scripts are designed for clean-checkout use. The contract script rejects any network passphrase other than Stellar testnet. No script changes mainnet configuration.

## Known gaps

- Real Freighter create/deposit/rebalance/withdraw evidence in a browser with the extension installed.
- Public deployment of Postgres, Redis, indexer, API, and relayer.
- Docker Compose and integration-test evidence on a Docker-capable machine.
- Durable outbound email provider for waitlist and drift notifications.
- Intended production-domain ownership and routing.
- Independent professional contract audit.
- Mainnet checklist and cutover, deliberately out of scope.

## Security

Review [`THREATS.md`](THREATS.md) before operating the testnet deployment. Report vulnerabilities privately to the repository owner. Do not test suspected exploits with mainnet assets.

## License

No open-source license has been granted yet. All rights are reserved unless a license is added explicitly.
