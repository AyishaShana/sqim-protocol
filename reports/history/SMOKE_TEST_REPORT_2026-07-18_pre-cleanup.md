# Sqim Testnet Smoke Test Report

Test date: 2026-07-15
Network: Stellar testnet only
Result: **PARTIAL - contract and off-chain flows pass; manual Freighter UI signing remains unverified**

No mainnet configuration, contract, or account was touched. This report replaces the failed smoke generation documented earlier on the same date.

## Creator product feature pass - 2026-07-15

The product app now includes the requested four-step creation flow, factory-backed creator pages, wallet-authenticated profile and notification settings, and a non-submitting algorithmic rebalance suggestion. These checks exercised the locally running app against the active testnet contracts and the live Postgres/Redis/API stack.

| Check | Result | Recorded evidence |
| --- | --- | --- |
| Basket creation steps | PASS | Browser completed asset selection, exact 10,000 BPS weights, name/description, and review. The review returned real Soroswap outputs for a 10 XLM three-asset allocation. |
| Factory deployment preparation | PASS | Review targets factory `CD5F...LAUY` and clearly requires the connected wallet before deployment. Existing factory transaction evidence remains in the contract sections below. |
| Creator mapping | PASS | `/creator/GCUG...UPXW` resolved the factory mapping and displayed only `CCRP...G4C`, the basket created by that address. |
| Authenticated profile API | PASS | A five-minute challenge was issued; real Ed25519 verification passed in `TestVerifyProfileSignature`; malformed signatures were rejected by the running API. |
| Profile validation | PASS | Bio/name/avatar limits, HTTP(S) avatar URLs, allowed frequencies, drift bounds, and required notification email are regression-tested. |
| Fixture leakage guard | PASS | `TestFixtureContractIDPatternsCannotMasqueradeAsLiveContracts` passed; live `/baskets` returned two records and zero fixture-pattern IDs. |
| Algorithmic suggestion | PASS | The live Core Four view used 5 price samples and proposed 30/20/25/25 from 40/20/20/20, marked as mechanical output, not investment advice, and drift-cap aware. |
| Unauthorized rebalance UI | PASS | With no creator wallet connected, all sliders, live quote, suggestion apply, and confirm controls were disabled; the suggestion remained inspectable and ignorable. |
| Layout and console | PASS | Desktop document width equalled viewport width (`1265` px), with no horizontal overflow and no application console errors. |
| Fail-closed startup | PASS | Deployment validation and basket reads now run concurrently; a cold live-testnet explorer load completed in `8024` ms and rendered both factory baskets without a failure banner. |
| Frontend regression suite | PASS | 18 tests passed; Oxlint passed; the production Vite build passed; `npm audit` found 0 vulnerabilities. |
| Go and Postgres integration | PASS | All Go packages passed; signed-profile tests passed; `event -> Postgres -> API` passed with the integration build tag. |
| Real extension-approved writes | BLOCKED | The controlled browser has no Freighter/Rabet extension. No profile edit, creation, or rebalance is marked wallet-flow PASS without a real approval and transaction hash. |

Notification preference storage is implemented for `off`, `weekly`, and `on-drift-only` email modes. Email delivery itself remains deliberate future scope and cannot submit transactions. The overall report therefore remains **PARTIAL**: the product surfaces and automated security checks pass, while extension-approved browser writes still require a manual wallet session.

## Browser generation repair - 2026-07-15

The app and all service defaults now use one active testnet manifest:

```text
factory:      CD5FSGAMGJYZRGMXQFT3B3D3SPIF3SOTJMYZXWLE44K2U4TWTSQVLAUY
basket:       CCIR6KR56D65BMVLI6RITJNI64XTPJZXGIP6XJCQKMYF24TNILERYGYF
basket token: CDGXSZKYKVND7ZWSOGHWSCW6IOR5OWRRNV35HU2GNL4HKX3HK3NJIO27
settlement:   CD7FCLIV6BKVWLS3SYRCRA2O44VCP6CMOGAFTWWWWXK4Z73CW2X4VFPU
oracle:       CBVNZE4UO2DSEC6SSCA6S5WMYFZ2NKPIOXQ6DJNG3RQXIGAITKNQCTAS
```

Direct testnet readback returned factory basket count `2`, the configured basket/token pair from factory index `0`, token supply `1911503703`, settlement slippage limit `200` bps, and oracle max age `1800` seconds. The browser now runs this validation before rendering product routes.

| Browser/off-chain check | Result | Recorded evidence |
| --- | --- | --- |
| Active deployment startup | PASS | Explorer rendered exactly the two baskets registered by factory `CD5F...LAUY`. |
| Superseded factory startup | PASS, blocked safely | Banner: `Active basket CCIR...YGYF is not registered by factory CDGS...KO2L.` Product actions were replaced by `Testnet deployment check failed`. |
| API generation cleanup | PASS | Before: three superseded baskets. After purge and ledger `3608727` backfill: only `CCIR...YGYF` and `CCRP...G4C`. |
| Fixture ID database guard | PASS | Postgres-backed integration test inserted `CBASKETTESTNET...` and required `AssertNoFixtureIDs` to reject it. |
| Frontend regression suite | PASS | `16 passed`; Oxlint, production build, and npm audit passed with zero vulnerabilities. |
| Freighter connection in controlled browser | BLOCKED | Browser output: `Freighter is not installed in this browser.` |
| Create basket signed through Freighter | BLOCKED | Not run; no extension, no transaction hash. |
| Deposit signed through Freighter and portfolio refresh | BLOCKED | Not run; no extension, no transaction hash. |
| Creator rebalance signed through Freighter | BLOCKED | Not run; no extension, no transaction hash. |
| Unauthorized wallet UI block | BLOCKED for manual wallet evidence | Unit regression passes and submission controls are disabled, but no second real wallet session was available. |
| Cancelled Freighter signature | BLOCKED for manual wallet evidence | Error mapping now returns `You rejected the transaction in your wallet. No changes were made.`, but no real prompt was available to cancel. |

The frontend wallet flow remains **BLOCKED**, not PASS. CLI transaction evidence elsewhere in this report proves the contracts, but does not substitute for the required Freighter browser evidence.

## Final active generation retest - 2026-07-15

The deployment currently used by `/app` is the final generation listed in `DEPLOYMENTS.md`: factory `CD5F...LAUY`, basket `CCIR...YGYF`, transferable token `CDGX...IO27`, settlement `CD7F...VFPU`, and oracle `CBVN...CTAS`.

| Check | Result | Real testnet evidence |
| --- | --- | --- |
| Create basket and token | PASS | `c8cec7b9114584cb632eab88bbb3770d78838bf2c03a1465505a6f67845a4645` |
| Deposit through real pools | PASS | `4f3ec4759a639421a7f347cf0a4a262acc3ea7bdbb8844a31647bc3664e3af39` |
| External basket-token transfer | PASS | `4878bcbdf08aef37fc886d959f6384fd6e7491f2330272e08d8e98874952cdc3` |
| Partial withdrawal and burn | PASS | `8475043e1371cae32348ffc8c480b34ef048c4e88025687425e52178db0f8d1c` |
| Bounded rebalance, constant supply | PASS | `a96b2c29f2eef687e10f96b0f3e8da42a9e3a7ac22d762bd2d9ec84fb3d5f275` |
| Exact-output multihop | PASS | `a47ee73fc8bbf729d6c8b8aea92f9e5c713d6b281164cecb26a193ef41e587a7` |
| Out-of-bounds drift | PASS, rejected | `[5100,1634,1633,1633]` trapped with `Error(WasmVm, InvalidAction)` during simulation |
| Shallow-pool slippage | PASS, rejected | 1,000 XLM -> WSOL trapped with Soroswap `Error(Contract, #507)` during simulation |

The partial withdrawal burned `2,000,000` raw shares, paid `1,476,812` raw XLM, and charged zero fee because the realized position was not profitable. Exact-output multihop moved `36` raw WETH through XLM and returned exactly `1` raw WBTC; both pair reserve deltas matched the transaction.

Final CLI readbacks:

```text
$ stellar contract invoke --network testnet --id CCIR...YGYF -- nav
"10417353"

$ stellar contract invoke --network testnet --id CCIR...YGYF -- total_basket_value
"999315635"

$ stellar contract invoke --network testnet --id CDGX...IO27 -- total_supply
"959279732"

$ stellar contract invoke --network testnet --id CCIR...YGYF -- target_weights_bps
[4000,2000,2000,2000]
```

The new `/app` browser walk read the live factory and displayed one Core Four basket at NAV `1.0417`, AUM `99.93 XLM`, four live constituent balances, the current 40/20/20/20 mandate, and all three Soroswap pool reserves. The visual rebalance editor was regression-tested for exact-bound acceptance and per-asset excess reporting. Automated app results are `16 passed`, including exact deposit-allocation rounding, deployment-pair validation, wallet rejection messaging, authorization gating, and bounded algorithmic suggestions, with clean Oxlint, zero npm audit vulnerabilities, and a successful production build.

The browser session did not contain an injected Freighter/Rabet extension, so no wallet prompt was fabricated. Real wallet signing remains the only unverified application step and keeps the overall report at PARTIAL.

### Product UI create/deposit retest

After replacing the legacy preview at `localhost:8080` with `/app`, the creator workspace successfully produced a live 10 XLM allocation quote: 4 XLM retained, 2 XLM routed to WETH, 2 XLM to WBTC, and 2 XLM to WSOL. The UI labels this as one atomic action and disables submission when live route outputs are unavailable.

The corresponding contract flow was then executed independently through Stellar CLI to isolate wallet/UI behavior from contract behavior:

```text
create result: CCRPASV5OVVPRLMT6BVW7NEPOQR2XOWZQQGEY7ZB6RTBQ4BYLYNH6G4C
share token:   CDESQ3MIT4IHJ6N3SLLQUQGNLDTS52Q6HJ6YBMF3JDKGZZJV6ZOWP3GM
create tx:     887d8c6f4599556f87032dc79fe2d83f142801e2448160a6a4eb45f077bb361a

deposit input: 100000000 raw XLM (10 XLM)
deposit return:100000000 raw basket shares
deposit tx:    d519ba2b0183668fc51827e15d43b29b624acb1965b55b4f9cd0a0fd40fce1a2

basket XLM:    40000000
basket WETH:   1938
basket WBTC:   56
basket WSOL:   47128
total supply:  100000000
holder balance:100000000
NAV:           9978389
AUM:           99783890
weights:       [4000,2000,2000,2000]
```

The deposit event log contains all three real Soroswap pair swaps and the final token mint in the same successful transaction. Reloading the factory-backed explorer showed both the original basket and `Sqim 10 XLM Verification Basket`; its detail page showed all four custody balances. Result: **PASS** for factory creation, tradable-token deployment, 10 XLM atomic distribution, mint return value, live discovery, and detail rendering.

## Result matrix

| Area | Check | Result |
| --- | --- | --- |
| Environment | CLI profile, RPC health, five contract interfaces | PASS |
| Factory | Create two-asset basket and tradable token | PASS |
| Deposit | First and second deposits at different NAV values | PASS |
| Settlement | Real Soroswap constituent transfers and pair swaps | PASS |
| Token | External SEP-41 transfer and transfer-aware basis | PASS |
| Withdraw | Partial burn, real constituent sale, payout, loss fee = 0 | PASS |
| Authorization | Wrong-signer withdrawal | PASS, rejected |
| Rebalance | Live 2-of-N relayer call and constant supply | PASS |
| Guards | Drift, slippage, and max-size rejection | PASS |
| Oracle | Fresh on-chain 2-of-N fallback submission | PASS |
| Indexer | Factory child discovery and all live events in Postgres | PASS |
| API | Live basket/history/metrics/portfolio endpoints | PASS |
| Frontend | Unit tests and production build | PASS |
| Frontend | Real Freighter create/deposit/rebalance, unauthorized, and cancellation flows | BLOCKED |

## 1. Environment

Installed CLI:

```text
$ stellar --version
stellar 27.0.0 (5a7c5fe76530bf4248477ac812fc757146b98cc4)
```

The global `testnet` profile was reset to the required RPC and passphrase. A native SAC lookup through that profile returned:

```text
CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC
```

Live RPC health at the end of testing:

```json
{"status":"healthy","latestLedger":3607031,"ledgerRetentionWindow":120960}
```

`stellar contract info interface --network testnet` succeeded for the factory, basket, basket token, settlement, and oracle adapter listed in `DEPLOYMENTS.md`.

Postgres, Redis, indexer, relayer, and API were running locally. `GET http://localhost:8081/health` returned:

```json
{"mode":"live","status":"ok"}
```

Docker is not installed on this workstation, so the equivalent services were run as local processes against the same environment configuration. Docker Compose execution itself was not retested.

## 2. Contract smoke test

### Factory creation

Created `Sqim Live XLM USD Basket` with XLM/SQUSD and weights `[5000,5000]`.

```text
basket: CCPSB5YLGJ4VEYNI7AH444YEYATDM2DBNAMUGN2MC4DUD3ZHER3LFLPM
token:  CA7ENUDZ3DFRVBCQ3ZFF74L6PROXJBIN7HBFDFATLFP64VAUGASBKWAS
tx:     af9aa1c8b7aac599b1ab9e00ec7a80ffefab35e72991d942f86e6b420da7c962
```

Result: **PASS**. Both child interfaces resolve on testnet and the token reports the basket as its mint/burn admin while remaining freely transferable.

### Deposits and real swaps

First deposit:

```text
input:            10000000 XLM base units
returned shares:  10000000
tx:               7d88b871e8bf1789721b92d44bb87291266d7a8733cf8ae775a20a777e78ca90
router input:     5000000 XLM
pair output:      915735 SQUSD
basket retained:  5000000 XLM
event NAV/AUM:    9979550 / 9979550
```

Second deposit at a different NAV:

```text
input:            1000000 XLM base units
returned shares:  1002049
tx:               4bf3c4c250bcfdcaba63e0622e9b263a933fb3551eda4ee44c9e9aff2c81552c
router input:     500000 XLM
pair output:      91523 SQUSD
event NAV/AUM:    9977441 / 10977230
```

A later valid `100000000` deposit also completed:

```text
returned shares:  100250867
tx:               1a0cf76e8aae7c0789f65fed7c67c06be4378398ef917bd17e988720f6bfa6a8
router input:     45000000 XLM
pair output:      8202320 SQUSD
```

Post-test custody readback proves balances are real contract assets, not internal counters:

```text
basket XLM:       59939358
basket SQUSD:     8945601
total supply:     109252916
on-chain NAV:     9927936
on-chain AUM:     108465597
```

Result: **PASS**.

### Tradable token and cost basis

Transferred `4000000` basket-token base units to a second testnet account:

```text
tx: 6fa4c83b5555ba1d55b9d93647bd297d9e3f5cbee37675004afebbf0e2908b9f
recipient shares after transfer: 4000000
recipient tracked basis: 9998137 per share (e7)
```

The transfer emitted both the normal SEP-41 transfer event and the basket basis callback. Result: **PASS**.

### Partial withdrawal

The recipient withdrew `2000000` shares:

```text
returned payout: 1993992 XLM base units
tx:              3fa0d7d45025ccbe6676a9ef7116d87a298210e1db7cf57086de850d8ab7299b
direct XLM:      1097385
SQUSD sold:      165135
swap output:     896607 XLM
fee:             0 (position realized a loss)
shares remaining: 2000000
```

Result: **PASS** for burn, constituent liquidation, payout, and zero fee at a loss. Positive-profit fee arithmetic remains covered by the Rust regression suite rather than this loss-making live position.

### Unauthorized withdrawal

The creator signer attempted to withdraw the recipient's shares:

```text
error: Missing signing key for account GBMVCDI6PXC2UHVKBPO5K7WPT2AXUNPCBHWYRV42J7H6BQVHHDXXSKIM
```

No transaction was submitted and there is no hash. Result: **PASS, rejected**.

### M-of-N rebalance

The Go relayer invoked the testnet-only authorization helper with two distinct configured rebalancer signers. On-chain weights changed from `[5000,5000]` to `[5500,4500]`.

```text
tx:             1c9a9f73d0d1362d853407e3df28ded1f8dba8098e54cd364902384cc00afcdd
supply before:  11002049
supply after:   11002049
database status: submitted
```

The helper signs each Soroban authorization entry and rejects any non-testnet passphrase. Signer secrets are environment-only and are neither logged nor stored in the repository. Result: **PASS**.

### Guard rejections

Drift request `[7000,3000]` from `[5500,4500]`, with a 1000 bps per-asset limit:

```text
HostError: Error(WasmVm, InvalidAction)
VM call trapped: UnreachableCodeReached
diagnostic function: rebalance
```

Direct swap with `500` bps requested slippage against a `100` bps contract maximum:

```text
HostError: Error(WasmVm, InvalidAction)
VM call trapped: UnreachableCodeReached
diagnostic function: swap
```

Deposit `1000000001` against a configured maximum of `1000000000`:

```text
HostError: Error(WasmVm, InvalidAction)
VM call trapped: UnreachableCodeReached
diagnostic function: deposit
```

All three failed during simulation, so no transaction hashes exist. Result: **PASS, rejected**.

### Oracle fallback quorum

A fresh XLM fallback price was submitted with both configured oracle signers:

```json
{
  "hash": "28a4d095e7e145843856c52f13cb713277cff2b84e06411132c09eac4a032435",
  "sendStatus": "PENDING",
  "status": "SUCCESS"
}
```

The stale-without-quorum path is covered by an on-chain contract unit test and traps rather than returning stale data. Result: **PASS**.

## 3. Off-chain smoke test

The indexer discovered the factory-created basket/token and wrote the live sequence into the fresh `sqim_verified` database:

```text
basket-created | 1
deposit        | 3
rebalance      | 2
basis          | 1
transfer       | 1
withdraw       | 1
```

The final large deposit appears in history with transaction hash `1a0cf76e...fa6a8`, amount `10`, shares `10.0250867`, event NAV `0.9936935`, and event AUM `10.8563913`.

Live API responses:

```text
GET /health                                      200 {"mode":"live","status":"ok"}
GET /baskets                                     200, one live basket
GET /baskets/{id}                                200, XLM/SQUSD at 55/45
GET /baskets/{id}/history?limit=100              200, exact chain hashes
GET /baskets/{id}/metrics                        200 {"nav":"0.9936935","aum":"10.8563913"}
GET /portfolio/{recipient}                       200 {"shares":"0.2"}
```

The metrics endpoint is event-time cached data. The read-only on-chain NAV can move after the last basket event as oracle prices update; a production freshness policy/periodic backend refresh is still required if the UI labels this value real-time.

Relayer database evidence:

```text
status:      submitted
tx_hash:     1c9a9f73d0d1362d853407e3df28ded1f8dba8098e54cd364902384cc00afcdd
old weights: [5000,5000]
new weights: [5500,4500]
```

Result: **PASS** for indexer -> Postgres -> Redis/API and guarded relayer submission.

## 4. Frontend smoke test

Automated frontend verification:

```text
Test Files  2 passed (2)
Tests       6 passed (6)
vite build: 1761 modules transformed, built successfully
```

The app configuration defaults to the final factory/basket, testnet RPC, live API on `localhost:8081`, and contract writes enabled. The API responds correctly to the browser-facing routes. Wallet unit tests cover wallet absence, rejected signing, successful submission, and argument encoding.

The real Freighter approval flow is **BLOCKED/UNVERIFIED**, not passed. Browser control failed to initialize with `Cannot redefine property: process`, and no wallet approval or private key was fabricated. A wallet owner must still complete create -> deposit -> portfolio refresh -> withdraw and an intentional rejection in a Freighter-enabled browser.

## 5. Regression suites

```text
cargo test --workspace
  basket:          22 passed
  oracle_adapter:   4 passed
  settlement:       3 passed
  total:           29 passed, 0 failed

go test ./...
  all packages passed

go test -tags=integration ./integration
  simulated event -> Postgres -> API passed

npm test && npm run build
  6 tests passed; production build passed
```

Non-blocking warnings remain for deprecated Soroban `events().publish` calls and a frontend bundle above 500 kB.

## Blocker disposition

The earlier critical blockers are fixed and retested:

1. Settlement now executes real Soroswap transfers and swaps.
2. Basket valuation uses actual custody balances.
3. Cost basis follows external basket-token transfers.
4. Indexer RPC filters are chunked and factory children are discovered dynamically.
5. Postgres, Redis, indexer, API, and relayer run in live mode.
6. Relayer submissions carry actual on-chain 2-of-N authorization.
7. Frontend uses the live API and final testnet IDs.

Remaining before public production/mainnet:

1. Complete the manual Freighter UI smoke flow.
2. Add periodic backend NAV/AUM refresh semantics or clearly label event-time values.
3. Replace opaque panic traps with typed contract errors.
4. Migrate deprecated event publication and code-split the frontend bundle.
5. Run the stack through Docker Compose in an environment with Docker installed.
6. Obtain an independent professional smart-contract and protocol audit.

## Launch decision

The corrected generation is suitable for continued controlled testnet testing. It is **not approved for mainnet or public production** while the real Freighter flow and independent audit remain incomplete. This internal pass is not a substitute for a professional audit of contracts that will hold user funds.

## Core Four follow-up retest - 2026-07-15

| Check | Result | Evidence |
| --- | --- | --- |
| Create four-asset basket | PASS | `351080d0068d0f25745feae79b55ed2fe8ee3e01bf68152729b70f5c12e746c0` |
| Deposit 100 XLM | PASS | `3ebdc861d8c7a585710cc977b63f672a99781a8902096f70c36bdf9e749b5cf2` |
| Four assets in custody | PASS | XLM `250000000`, WETH `24510`, WBTC `710`, WSOL `594961` immediately after deposit |
| Three constituent pools used | PASS | Deposit event log contains XLM/WETH, XLM/WBTC and XLM/WSOL swaps |
| Bounded rebalance | PASS | `d740db2a3b3fbe949f27c48bcbc0496f946375636b3be4c093db5bfdc1aeb07d` |
| Supply unchanged | PASS | `total_supply` returned `1000000000` before and after rebalance |
| Drift cap rejection | PASS | `41/25/25/9` simulation rejected with `HostError: Error(WasmVm, InvalidAction)` before submission |
| Indexer to API | PASS | API returns live create, deposit and rebalance events plus NAV `0.9949182`, AUM `99.491827` |
| Frontend automated suite | PASS | 11 tests passed; TypeScript and Vite production build passed |

The frontend now exposes the four live oracle prices, current target weights, indexed rebalance activity, all three testnet pool reserves, and connected-wallet LP balances. Browser-controlled Freighter approval remains a manual test requirement because the local browser automation bridge failed to initialize; this is still not marked as a passed wallet smoke flow.

## Active budget-fixed generation - 2026-07-15

This section supersedes the contract IDs and Core Four proof in the preceding follow-up. The settlement ABI now returns the exact oracle snapshot used for its slippage checks, allowing the basket to calculate current-NAV minting without repeating four oracle calls and exceeding Soroban's resource budget.

| Check | Result | Evidence |
| --- | --- | --- |
| Deploy active factory, settlement, basket, and token | PASS | Factory `CDGS...KO2L`, settlement `CCD7...2P4N`, basket `CAVC...A653`, token `CBPH...QUHC` |
| First 1 XLM deposit | PASS | `5c54e60f9ea5048534e8491c5186af1f9c9d4a81893908dc3e2c54a832dbbb22`; minted 1 share and used all three pools |
| Second deposit at current NAV | PASS | `71f19b57933d119ade461b166fc647c8357ad609780a39fdc02b1d53af011237`; minted 1.0101056 shares at NAV 0.9849976 |
| Bounded rebalance | PASS | `88987ee6ba1fd7e6c2924076aada8b7dafd6bc246719fc1a7793f790998045bb`; target moved to 30/25/25/20 |
| Supply unchanged by rebalance | PASS | `total_supply` remained 20,101,056 raw units |
| Drift cap | PASS, rejected | `41/25/25/9` trapped during simulation with `HostError: Error(WasmVm, InvalidAction)`; no transaction hash |
| Large-trade slippage guard | PASS, rejected | 100 XLM simulation failed with Soroswap `Error(Contract, #507)`; no transaction hash |
| Basket custody | PASS | XLM 6,001,818; WETH 488; WBTC 14; WSOL 9,474 raw units |
| Live LP positions | PASS | Ayisha balances: 596,222,565 XLM/WETH; 101,517,274 XLM/WBTC; 2,937,501,163 XLM/WSOL raw LP units |
| Indexer and API | PASS | API contains create, both deposit hashes, and rebalance; cached NAV 0.9850807 and AUM 1.9801164 |

Latest direct contract readback returned NAV `0.9838214` and AUM `1.9775851`. The API's small difference is an event-time snapshot from the rebalance ledger, not a claim of tick-level oracle freshness. The frontend labels and presents live oracle prices separately.

The attached historical `#507` trace referencing basket `CCPS...LFLPM` is from a superseded two-asset deployment and a large 100 XLM request. The active UI no longer targets that contract and presents `#507` as an actionable liquidity/slippage error.

Final local verification for this generation:

```text
cargo fmt --all -- --check                 PASS
cargo test --workspace                     PASS (29 tests)
stellar contract build                     PASS (5 contracts)
go test ./...                              PASS
go test -tags=integration ./integration    PASS
npm test -- --run                          PASS (11 tests)
npm run build                              PASS
GET localhost:8080                         200
GET localhost:8081/health                  200
GET localhost:8081/baskets                 200
GET active basket /metrics                 200
```

Postgres, Redis, indexer, relayer, API, and Vite were all running locally at final readback. The production frontend bundle remains above Vite's 500 kB advisory threshold; route-level code splitting is a performance follow-up, not a correctness failure.

The final route check also found and fixed test-data pollution: the Postgres integration test had left `CBASKETTESTNET` in `basket_configs`, causing the explorer to select a preview fixture before the live basket. The test now uses a unique fixture ID and removes its event/config rows on completion. After deleting the stale row and rerunning the test, `/baskets` returns `CAVC...A653` first and no test fixture remains.

## Historical performance and backtesting - 2026-07-16

This pass did not submit contract transactions or change testnet deployment IDs. It verified the new off-chain historical pipeline and browser UI against the already active testnet baskets.

| Check | Result | Executed evidence |
| --- | --- | --- |
| Public source ingestion | PASS | `go run ./backtester --sync-only` fetched completed Binance daily spot candles and wrote five Parquet datasets. |
| Minute granularity | PASS | `--granularity minute --from 2026-07-14` wrote `xlm-minute.parquet` with 2,880 real minute candles through 2026-07-15; the temporary verification file was removed after readback. |
| Honest coverage | PASS | BTC/ETH: 2017-08-17, 3,255 points; XLM: 2018-05-31, 2,968; USDC: 2018-12-15, 2,605; SOL: 2020-08-11, 2,165. All ended at the last completed UTC day, 2026-07-15. |
| No fabricated young-asset history | PASS | Core Four starts at SOL's first shared candle, `2020-08-11`; unsupported asset tests return an unavailable-data error. |
| Backtester health | PASS | `GET localhost:8090/health` returned `status=ok`, `provider=binance-public-data`, `quote_currency=USDT`. |
| API proxy | PASS | `GET /backtesting/assets` returned five real coverage records; `POST /backtesting/run` returned 2,165 shared Core Four observations. |
| Core Four maximum-window model | PASS | 40/20/20/20 result: total return `9.6672241286`, annualized volatility `0.7629273314`, max drawdown `-0.8374792887`; these are simulations, not a Sqim live track record. |
| Live NAV separation | PASS | `GET /baskets/{id}/nav-history` is backed only by indexed post-deployment Soroban events; the Postgres integration test asserted the event reaches this endpoint. |
| Browser basket detail | PASS | Running app showed separate green `On-chain NAV since deployment` and marigold `Historical target-weight model` cards, `5.9 years available`, and `Limited by SOL, from Aug 2020`. |
| Browser creator tool | PASS | Create flow reached review, `Run backtest` returned a 5-year result with return, volatility, drawdown, youngest-asset disclosure, and the mandatory simulation disclaimer. |
| Browser integrity | PASS | One historical section/result card, no horizontal overflow at 855px viewport, and no console errors. |
| Backtester mutation surface | PASS | HTTP `POST /sync` returned 404; refresh is available only through the explicit `--sync-only` operations command. |

Executed gates:

```text
go test ./... -count=1                         PASS
go test -tags=integration ./integration        PASS
npm test -- --run                              PASS (18 tests)
npm run lint                                   PASS
npm run build                                  PASS
npm audit --omit=dev                           PASS (0 vulnerabilities)
```

The Vite production bundle still reports the pre-existing chunk-size advisory (`1,295.13 kB`, `353.75 kB` gzip). Historical results explicitly exclude fees, slippage, taxes, liquidity constraints, and market impact. They must not be presented as a live basket track record or investment advice.

## Liquidity blocker retest - 2026-07-18

The browser's Soroswap `#507` deposit failure was reproduced against the active Core Four basket. The pools were funded, but their reserve ratios had moved 2.5%-4.1% away from the oracle while settlement allowed only `200` bps. The failure was therefore caused by oracle-relative price drift, not an empty pool.

Each XLM pair received another 3,000 XLM and the wrapped amount needed to place its spot ratio approximately 0.5% inside the current oracle price. All nine funding and LP-deposit transaction hashes are recorded in [POOLS.md](POOLS.md).

| Check | Result | Evidence |
| --- | --- | --- |
| API/indexer live mode | PASS | `GET /health` returned `mode=live`, `status=ok`. |
| 1 XLM deposit | PASS | `166c2b7c9554848c0a19725fccf23cd05f63a729fdf90592d3c06656276a1772`; 0.9710789 SQIMB minted. |
| 10 XLM deposit | PASS | `a3ec7efde1d22c80fa376bfa2ec446026c067f47ced594385d34ccbfac5d74ec`; 9.7132238 SQIMB minted. |
| 50 XLM deposit | PASS | `c3b3a4f8ee7197d3c5615828e829d54692c874f73ce5b8cca8352849d8220f75`; 48.5729557 SQIMB minted. |
| 100 XLM configured maximum | PASS | `b48d2c695827f4df13170620ad2a8f008abaa5fd232f3c20834ddff29733d303`; 97.1467041 SQIMB minted. |
| Final post-funding deposit | PASS | `3128d28cc4dfe491c98b6f1cf90c1b549956d3db2c538a7670d4425bb712929d`; indexer recorded NAV 1.0286026 and AUM 265.9209609. |
| Real pool movement | PASS | Latest reserves: WETH/XLM `[9071782,90859258877]`, WBTC/XLM `[261534,90860794253]`, WSOL/XLM `[222613305,90808316106]`. |
| Dust withdrawal preflight | PASS in UI, contract constraint remains | Live reverse-route quoting now blocks a below-minimum or dust withdrawal before wallet signing and identifies the affected route. |
| Bounded rebalance retest | BLOCKED | The current four-asset rebalance simulation exceeds Soroban's resource budget. Liquidity funding does not bypass or fix this separate contract-budget limit. |

Stellar CLI v23 submitted the write invocations used for the 1/10/50/100 XLM range checks. The table reports them as real transactions rather than simulations; the independent indexer entries and balance changes confirm submission.

Frontend verification after the error-handling change:

```text
npm test -- --run    PASS (20 tests)
npm run lint         PASS
npm run build        PASS
```

Deposit liquidity is no longer a blocker within the basket's configured transaction range. The system remains testnet-only and is not production/mainnet ready while the four-asset rebalance resource-budget failure and manual Freighter browser evidence remain unresolved.

Direct browser readback at `localhost:8080` showed the funded 1 XLM deposit preview resolving all four allocations. In withdrawal mode, 1 SQIMB reported `BTC redemption is dust-sized` with the payout marked unavailable, while 8 SQIMB returned a live gross and net route quote of `8.1834 XLM`. No transaction was submitted during these browser preview checks.
