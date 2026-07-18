# Sqim Final Testnet Smoke Test

> **TESTNET ONLY. No mainnet configuration or real funds were used.**

- Test date: 2026-07-18
- Active generation: [`DEPLOYMENTS.md`](DEPLOYMENTS.md)
- Overall result: **PARTIAL**
- Release verdict: **NOT READY**

This report supersedes earlier root reports. Previous evidence is archived under [`reports/history`](reports/history).

## Summary

| Surface | Result | Evidence |
| --- | --- | --- |
| Rust formatting, lint, tests, WASM build | PASS | Workspace formatting, strict Clippy, 33 contract tests, and all five WASM builds completed locally with no warning after using a no-space Cargo home. |
| Fresh contract generation | PASS | All active IDs resolve on Stellar testnet; deployment transactions are in `DEPLOYMENTS.md`. |
| Real Soroswap settlement | PASS | Deposit, withdrawal, and rebalance changed testnet pool and basket balances. |
| Typed rejection paths | PASS | Drift, quorum, oracle, and slippage failures return contract error codes rather than panic traps. |
| Public frontend read path | PASS | Public Vercel build resolved the final factory, rendered the live basket, and passed fresh direct loads for detail and Risks routes after the SPA rewrite was repaired. |
| Real Freighter write flow | BLOCKED | The controlled browser has no Freighter/Rabet extension. No browser write is marked PASS. |
| Docker Compose stack | BLOCKED | Docker is not installed on this machine. |
| Postgres integration run | BLOCKED | No integration database is running without Docker. |
| Public API/indexer/relayer | BLOCKED | No production hosting target or credentials were supplied for the Go stack. |
| Push and CI | BLOCKED | GitHub CLI is not authenticated. |

## Active readback

The Stellar CLI `testnet` profile returned `Healthy` at ledger `3668625`, protocol `27`, RPC version `27.1.1`, passphrase `Test SDF Network ; September 2015`, and the official Friendbot URL. `stellar contract info meta` resolved all five active Sqim contracts on that network with Soroban SDK `27.0.0` metadata.

| Value | Result |
| --- | --- |
| Factory | `CD4WD6PVISQ6T6PXZ2L4RAWBI3ZVOGNZRYO7RIHOL5XUIF6QOBD26EVM` |
| Basket | `CD7XI7DSE7YNBWI224SUWAF4DWY5DOMLTQZZKF4SOWSLCQC7UB6STUHW` |
| Token | `CAWV5OPADYFZQFA7YCWKKWX3BWFWJAKESTQHEMTGDPXZV7X36CKFFVO4` |
| Final target weights | `[3500,2500,2000,2000]` |
| Final supply | `17,316,371` raw shares |
| Final NAV | `9,827,882` raw XLM per share unit |
| Final AUM | `17,018,326` raw XLM |

## Contract smoke test

### Deployment and basket creation

| Step | Result | Transaction |
| --- | --- | --- |
| Deploy oracle adapter | PASS | [`c6839c74837111745cae496036096604aeb3d023108673cf82260eb96b59bb28`](https://stellar.expert/explorer/testnet/tx/c6839c74837111745cae496036096604aeb3d023108673cf82260eb96b59bb28) |
| Deploy settlement | PASS | [`3ec9376d66f75bf777b6d87976aab8488c2ba8927c4728ee8b5f696356b81372`](https://stellar.expert/explorer/testnet/tx/3ec9376d66f75bf777b6d87976aab8488c2ba8927c4728ee8b5f696356b81372) |
| Deploy factory | PASS | [`7006913480de1c67da80ac374b82c324a3cf478752e2e5fdb27654bfb1a783d7`](https://stellar.expert/explorer/testnet/tx/7006913480de1c67da80ac374b82c324a3cf478752e2e5fdb27654bfb1a783d7) |
| Create four-asset basket and transferable token | PASS | [`34bc705e3905ae10bd49a47a248cd87234c23aa8310e5add585efb97a89b2cf4`](https://stellar.expert/explorer/testnet/tx/34bc705e3905ae10bd49a47a248cd87234c23aa8310e5add585efb97a89b2cf4) |

### Deposit, transfer, and withdrawal

| Step | Result | Actual result | Transaction |
| --- | --- | --- | --- |
| First 1 XLM deposit | PASS | Minted `10,000,000` raw shares. Swaps produced `199` WETH, `5` WBTC, and `4,887` WSOL raw units; basket retained `4,000,000` raw XLM. | [`044c6e7bd9732a8810e8e37c4ed3abf8411b85e5af44629eafd2aab2c2526539`](https://stellar.expert/explorer/testnet/tx/044c6e7bd9732a8810e8e37c4ed3abf8411b85e5af44629eafd2aab2c2526539) |
| Second 1 XLM deposit at different NAV | PASS | Minted `10,316,371` raw shares at changed NAV; AUM became `19,386,662`. | [`73ba857f256c56821af9d57dc60f506707e7b5bd5b51adad732730fc202ef93c`](https://stellar.expert/explorer/testnet/tx/73ba857f256c56821af9d57dc60f506707e7b5bd5b51adad732730fc202ef93c) |
| External SEP-41 transfer | PASS | Sent `4,000,000` raw shares to a second account outside protocol entrypoints. | [`de5c7d0b1b54328e58e2b7837d8563ec8eab8fb88336484f132f418d5319d499`](https://stellar.expert/explorer/testnet/tx/de5c7d0b1b54328e58e2b7837d8563ec8eab8fb88336484f132f418d5319d499) |
| Partial withdrawal | PASS | Burned `3,000,000` raw shares, returned `2,338,985` raw XLM, and charged zero profit fee because the position realized a loss. Reverse WETH and WSOL swaps executed. | [`30be1866031215dc9c8a3001e606df1c8fc13e4d99319aca8b93e96e4f66c6d0`](https://stellar.expert/explorer/testnet/tx/30be1866031215dc9c8a3001e606df1c8fc13e4d99319aca8b93e96e4f66c6d0) |
| Unauthorized withdrawal | PASS, rejected | CLI could not authorize the holder with the wrong signer: `Missing signing key for account`. No transaction was submitted. | None by design |

### Rebalance and safety guards

| Step | Result | Actual result | Transaction |
| --- | --- | --- | --- |
| Creator bounded rebalance | PASS | Weights moved from `[4000,2000,2000,2000]` to `[3000,3000,2000,2000]`; `1,706,006` raw XLM sold for `169` raw WETH; supply stayed `17,316,371`. | [`a79001d2026951fc783e85995742b1cdd1262e07f4e83fd27a11b158b7f437f9`](https://stellar.expert/explorer/testnet/tx/a79001d2026951fc783e85995742b1cdd1262e07f4e83fd27a11b158b7f437f9) |
| On-chain 2-of-2 rebalance | PASS | Multisignature helper moved weights to `[3500,2500,2000,2000]`; supply remained unchanged. | [`5246cda8fcce62495558e7b552866500a65073df918a1f85ae8421fec55d8e7f`](https://stellar.expert/explorer/testnet/tx/5246cda8fcce62495558e7b552866500a65073df918a1f85ae8421fec55d8e7f) |
| Drift above per-call bound | PASS, rejected | Contract returned typed `Error::DriftExceeded` (`#1019`). | None by design |
| Unauthorized rebalance | PASS, rejected | Contract returned typed `Error::QuorumNotMet` (`#1020`). | None by design |
| Insufficient-liquidity withdrawal | PASS, rejected | Router rejection was mapped to typed `Error::SlippageExceeded` (`#4008`); no state change occurred. | None by design |

### Oracle behavior

| Step | Result | Evidence |
| --- | --- | --- |
| Disable primary oracle | PASS, incomplete durable evidence | CLI session recorded hash prefix/suffix `d99831...dedee`; the full hash was not preserved. |
| No fallback | PASS, rejected | Typed `Error::PriceUnavailable` (`#5006`). |
| One fallback signer | PASS, rejected | Typed `Error::QuorumNotMet` (`#5009`). |
| Valid 2-of-2 fallback | PASS | [`f319db4c6dd3e6387f02f3c733d24552ea71dcaa7cd67b367f826a5486f42fd1`](https://stellar.expert/explorer/testnet/tx/f319db4c6dd3e6387f02f3c733d24552ea71dcaa7cd67b367f826a5486f42fd1) |
| Re-enable primary | PASS, incomplete durable evidence | CLI session recorded hash prefix/suffix `66556d74...c561`; the full hash was not preserved. |

The shortened oracle-control hashes are not sufficient durable evidence for a release approval. This is one reason the report remains PARTIAL.

## Off-chain verification

| Check | Result |
| --- | --- |
| Go formatting, vet, unit tests, fixture guard, HTTP handlers, event decoding | PASS |
| Fixture IDs rejected from non-test database writes | PASS |
| API NAV/AUM includes `as_of_ledger`, `as_of`, and source metadata | PASS |
| Docker Compose boot | BLOCKED: Docker executable not installed |
| `go test -tags=integration -v ./integration` | BLOCKED: test reported SKIP because `SQIM_TEST_DATABASE_URL` was not available |
| Public indexer/API/Redis/relayer | BLOCKED: no public service hosting target supplied |

## Frontend verification

| Check | Result | Evidence |
| --- | --- | --- |
| Formatting, Oxlint, tests, production build | PASS | Prettier and Oxlint passed; 24 tests passed; Vite completed with route-level code splitting. |
| Public fresh load | PASS | `https://sqim-protocol-stellar-sigma.vercel.app/explore` resolved the final factory and displayed one live basket, NAV, AUM, composition, testnet banner, and Risks footer link. |
| Deep-link routing | PASS after repair | Initial `/basket/...` and `/risks` requests returned Vercel 404. Rewriting all SPA routes to `/` and redeploying fixed both direct loads. Deployment `dpl_8GgZCRvaG4u79xGgHaJeZLF58ViC`. |
| Startup deployment health check | PASS | Product routes remain behind factory, basket, token, settlement, and oracle read checks. |
| Freighter connect/create/deposit/rebalance/withdraw | BLOCKED | Wallet dialog opens and reports `Freighter is not installed in this browser.` No signed write was possible. |
| Rejected Freighter signature | BLOCKED | Error mapping is tested, but no real prompt was available to reject. |
| Unauthorized wallet UI | AUTOMATED PASS, MANUAL BLOCKED | Controls are disabled by authorization state, but no second extension wallet was available. |

The requested hostname `sqim-protocol-stellar.vercel.app` could not be assigned because Vercel reports that alias as already in use. The verified alias above is public, but the acceptance test is not complete until the intended domain and public API are connected.

## Final status

**Testnet production deployment: NOT READY, blocked on real Freighter evidence, Docker Compose and Postgres integration evidence, public API/indexer/relayer hosting, complete durable oracle-control hashes, GitHub push/CI, and an external professional audit before any mainnet consideration.**
