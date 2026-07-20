# Production Readiness

> Scope: Stellar testnet only. No real assets, no real funds, and no mainnet configuration.

## Live and verified

- A final contract generation is deployed on Stellar testnet and recorded in [`DEPLOYMENTS.md`](DEPLOYMENTS.md).
- The factory created a four-asset basket and a normal transferable basket token.
- Deposits, a second-NAV deposit, external token transfer, partial withdrawal, creator rebalance, and 2-of-2 rebalance executed on testnet with transaction evidence.
- Settlement executes real Soroswap exact-in/exact-out routes and maps router failures to typed Sqim errors.
- Drift, slippage, transaction-size, pause, timelock, authorization, oracle age, and fallback-quorum controls are enforced on-chain and regression-tested.
- The public frontend reads the active generation, fails closed on deployment drift, and persistently labels every route TESTNET ONLY.
- Historical simulations are visibly separate from live on-chain history; algorithmic suggestions explicitly state that they are mechanical output, not advice.
- NAV and AUM API responses carry ledger and timestamp freshness metadata.
- GitHub `main` is pushed under the Ayisha account, and contracts, services, application, and fixture-guard jobs passed without annotations in [Actions run 29717367088](https://github.com/AyishaShana/sqim-protocol/actions/runs/29717367088).

See [`SMOKE_TEST_REPORT.md`](SMOKE_TEST_REPORT.md) for exact hashes and rejection output.

## Not complete

- Real Freighter extension approval has not been exercised in the available controlled browser.
- Docker is not installed on this machine, so the final Compose and Postgres integration run is not evidenced.
- Indexer, API, Redis, and relayer are not deployed to a public service host.
- The preferred Vercel alias is owned by another project; the verified build currently uses the available project alias.
- Waitlist/profile email preferences are durably stored only when Postgres is deployed; outbound email delivery is not implemented.
- The contracts have not received an independent professional audit.
- Mainnet deployment and mainnet configuration remain deliberately untouched.

## Audit gate

Sqim holds user-directed assets and makes oracle-dependent cross-contract swaps. **An independent professional smart-contract audit is required before mainnet.** This engineering pass and its tests are not a substitute for that audit.

## Status

**Testnet production deployment: NOT READY, blocked on real Freighter public-flow evidence, Docker/Postgres integration evidence, public backend hosting, intended-domain ownership, durable email delivery, complete oracle-control evidence, and independent audit.**
