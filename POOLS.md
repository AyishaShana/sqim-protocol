# Soroswap Testnet Pools

The Core Four basket uses XLM as its deposit, payout, and rebalance base asset. These pools were created and seeded on Soroswap testnet so settlement can execute real constituent trades. Every amount below was decoded from the successful Soroban transaction event XDR.

All assets use seven decimals.

## Initial liquidity

| Pair | XLM seeded | Wrapped asset seeded | LP tokens minted | Seed transaction |
| --- | ---: | ---: | ---: | --- |
| XLM / WETH | `6,000 XLM` (`60,000,000,000` raw) | `0.5924709 WETH` (`5,924,709` raw) | `596,222,565` raw | [`c855cc47...45c1`](https://stellar.expert/explorer/testnet/tx/c855cc47ac4a3c5f697007cc55e12a72227fe139e02892b1abf197200f5445c1) |
| XLM / WBTC | `6,000 XLM` (`60,000,000,000` raw) | `0.0171766 WBTC` (`171,766` raw) | `101,517,274` raw | [`eecaa359...2cf9`](https://stellar.expert/explorer/testnet/tx/eecaa359a3d1b35778181c4dd9f1e01c041e2763872b0db58ad160a7c7f52cf9) |
| XLM / WSOL | `6,000 XLM` (`60,000,000,000` raw) | `14.3815316 WSOL` (`143,815,316` raw) | `2,937,501,163` raw | [`4e6cb77a...6cd3`](https://stellar.expert/explorer/testnet/tx/4e6cb77a4be118575f0ad95aebed2679a44f95d0b46c7d1ec7913e8b10656cd3) |

Soroswap permanently minted an additional `1,000` raw LP units to each pair contract as minimum liquidity. The provider balances above exclude that locked minimum.

## Pair contracts

| Pair | Pair contract | Token order |
| --- | --- | --- |
| XLM / WETH | [`CDJNXIKZ...22AMGZY`](https://stellar.expert/explorer/testnet/contract/CDJNXIKZUNCQDCKVGOPZEKFPDJG2LMNBNQ2N5XE5CGVNCMOAL22AMGZY) | WETH, XLM |
| XLM / WBTC | [`CA654TSP...KDHZZAG`](https://stellar.expert/explorer/testnet/contract/CA654TSPI6WHJPOO2BSVXFIKBH7G3F66SKST6WDXKAT2SG3DDKDHZZAG) | WBTC, XLM |
| XLM / WSOL | [`CALJ4IL3...L6QROIP`](https://stellar.expert/explorer/testnet/contract/CALJ4IL3IBSGORPH2U76HZHYLOHE533TUDJBGR2VWACNRCIROL6QROIP) | WSOL, XLM |

## Pre-top-up reserve readback

Read from each pair's `get_reserves()` view on 2026-07-15 after the settlement smoke tests:

| Pair | Wrapped reserve | XLM reserve |
| --- | ---: | ---: |
| XLM / WETH | `5,879,368` raw WETH | `60,464,166,426` raw XLM |
| XLM / WBTC | `170,450` raw WBTC | `60,465,353,404` raw XLM |
| XLM / WSOL | `142,835,581` raw WSOL | `60,413,104,637` raw XLM |

Reserve changes from the initial values are expected and are direct evidence that deposit, withdrawal, rebalance, and exact-output test swaps moved real pool assets.

## Reproduce the readback

```powershell
stellar contract invoke --network testnet --id CDJNXIKZUNCQDCKVGOPZEKFPDJG2LMNBNQ2N5XE5CGVNCMOAL22AMGZY -- get_reserves
stellar contract invoke --network testnet --id CA654TSPI6WHJPOO2BSVXFIKBH7G3F66SKST6WDXKAT2SG3DDKDHZZAG -- get_reserves
stellar contract invoke --network testnet --id CALJ4IL3IBSGORPH2U76HZHYLOHE533TUDJBGR2VWACNRCIROL6QROIP -- get_reserves
```

These are deliberately shallow testnet pools. Settlement's oracle-relative slippage check must reject a route when the realized pool output falls below the protected minimum; the pools are not suitable for production capital.

## Oracle alignment and depth top-up - 2026-07-18

The original reserves remained funded, but their spot ratios had drifted outside settlement's current `200` bps (2%) oracle tolerance. That made even a small deposit fail with Soroswap error `#507`. Each pool was realigned to approximately 0.5% inside the current oracle price and received another 3,000 XLM of testnet depth.

| Pair | Wrapped asset added | XLM added | Wrapped funding tx | XLM funding tx | LP deposit tx |
| --- | ---: | ---: | --- | --- | --- |
| XLM / WETH | `3,231,621` raw | `30,000,000,000` raw | [`2d8586b2...2eec`](https://stellar.expert/explorer/testnet/tx/2d8586b2e8583c9eb2e8a893988af86fec5fb79455440d1ff5f3ca6c12232eec) | [`6d2742bf...8e53`](https://stellar.expert/explorer/testnet/tx/6d2742bf349ecb95a85c8750a050c6fd13cc4e1a8ab5d4cf9b0f1d5fc0038e53) | [`5932485b...0bb`](https://stellar.expert/explorer/testnet/tx/5932485b34ca318e85e43eae92336821f0058ce583407068cc5c7561a75010bb) |
| XLM / WBTC | `92,211` raw | `30,000,000,000` raw | [`a83a2e7c...9bd8`](https://stellar.expert/explorer/testnet/tx/a83a2e7ca0f9de826560d0d0a853306f0a6e91724f185479b71fd74d2a0a9bd8) | [`21e29c7e...c3ce`](https://stellar.expert/explorer/testnet/tx/21e29c7ed91e8fdc1b2f36bfe96ba666af0ee349791fd783a185f95c3c6bc3ce) | [`0e9e5680...9039`](https://stellar.expert/explorer/testnet/tx/0e9e5680d39800a76d124b5d69a89bc755edaac9a6cc71eb5745b0334c509039) |
| XLM / WSOL | `80,739,632` raw | `30,000,000,000` raw | [`0596ea2d...b80`](https://stellar.expert/explorer/testnet/tx/0596ea2d508572aefa16f34935508dc1bc223479f9f7e7984e2bc0f7e9eaeb80) | [`cdc13653...44b`](https://stellar.expert/explorer/testnet/tx/cdc13653e2b21eb2ddf7c6c9e8ceeb91e00ea72d71d23a53bf21814f1fe8544b) | [`611cf6b7...28c6`](https://stellar.expert/explorer/testnet/tx/611cf6b74f35b8e6493e7f203a222878cbede56d71cc5a721bacfb99a66a28c6) |

LP tokens minted by the top-up were `295,573,501` raw for XLM/WETH, `50,325,670` for XLM/WBTC, and `1,457,472,283` for XLM/WSOL.

Final reserve readback on 2026-07-18 after funding and the active-generation smoke test:

| Pair | Wrapped reserve | XLM reserve |
| --- | ---: | ---: |
| XLM / WETH | `9,070,875` raw WETH | `90,868,393,898` raw XLM |
| XLM / WBTC | `261,514` raw WBTC | `90,868,794,253` raw XLM |
| XLM / WSOL | `222,595,199` raw WSOL | `90,815,729,419` raw XLM |

The pools now clear Core Four deposits throughout the basket's configured 1-100 XLM transaction range. Very small withdrawals remain constrained by seven-decimal integer fee rounding, especially the WBTC leg; the application performs a live reverse-route quote and asks the user to increase the burn amount before opening the wallet when a dust route cannot satisfy the oracle minimum.
