# Sqim Testnet Deployment

> **TESTNET ONLY. These contracts use valueless test assets. Do not send mainnet funds.**

This is the canonical current deployment manifest. Runtime configuration is mirrored in [`config/testnet.json`](config/testnet.json). Superseded generations are recorded only in [`DEPLOYMENTS_HISTORY.md`](DEPLOYMENTS_HISTORY.md).

- Network: Stellar testnet
- RPC: configured through `SOROBAN_RPC_URL`
- Passphrase: configured through `SOROBAN_NETWORK_PASSPHRASE`
- Deployment date: 2026-07-18
- Initial ledger: `3667948`

## Active contracts

| Component | Contract ID |
| --- | --- |
| Factory | [`CD4WD6PVISQ6T6PXZ2L4RAWBI3ZVOGNZRYO7RIHOL5XUIF6QOBD26EVM`](https://stellar.expert/explorer/testnet/contract/CD4WD6PVISQ6T6PXZ2L4RAWBI3ZVOGNZRYO7RIHOL5XUIF6QOBD26EVM) |
| Basket | [`CD7XI7DSE7YNBWI224SUWAF4DWY5DOMLTQZZKF4SOWSLCQC7UB6STUHW`](https://stellar.expert/explorer/testnet/contract/CD7XI7DSE7YNBWI224SUWAF4DWY5DOMLTQZZKF4SOWSLCQC7UB6STUHW) |
| Transferable basket token | [`CAWV5OPADYFZQFA7YCWKKWX3BWFWJAKESTQHEMTGDPXZV7X36CKFFVO4`](https://stellar.expert/explorer/testnet/contract/CAWV5OPADYFZQFA7YCWKKWX3BWFWJAKESTQHEMTGDPXZV7X36CKFFVO4) |
| Settlement | [`CAK6F4PFOS2LU2FWPVGHI6AALYWBEHIHQBSF7XM2JQJRBAIU3QDSTD4L`](https://stellar.expert/explorer/testnet/contract/CAK6F4PFOS2LU2FWPVGHI6AALYWBEHIHQBSF7XM2JQJRBAIU3QDSTD4L) |
| Oracle adapter | [`CAUPYKDDPXL4NGRPYKHYIDND74FCMXEBGG2DW2BJLCO7LR7O3MEDZ22E`](https://stellar.expert/explorer/testnet/contract/CAUPYKDDPXL4NGRPYKHYIDND74FCMXEBGG2DW2BJLCO7LR7O3MEDZ22E) |
| Reflector primary oracle | [`CCYOZJCOPG34LLQQ7N24YXBM7LL62R7ONMZ3G6WZAAYPB5OYKOMJRN63`](https://stellar.expert/explorer/testnet/contract/CCYOZJCOPG34LLQQ7N24YXBM7LL62R7ONMZ3G6WZAAYPB5OYKOMJRN63) |
| Soroswap router | [`CCJUD55AG6W5HAI5LRVNKAE5WDP5XGZBUDS5WNTIVDU7O264UZZE7BRD`](https://stellar.expert/explorer/testnet/contract/CCJUD55AG6W5HAI5LRVNKAE5WDP5XGZBUDS5WNTIVDU7O264UZZE7BRD) |

## Assets and pools

| Asset | Token contract | XLM pool |
| --- | --- | --- |
| XLM | [`CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC`](https://stellar.expert/explorer/testnet/contract/CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC) | Native leg |
| Test WETH | [`CBM3JP2PSBUSM23TUEENQRNYOT72XATPK5ACGUOZEYUWISLZGLXT5C75`](https://stellar.expert/explorer/testnet/contract/CBM3JP2PSBUSM23TUEENQRNYOT72XATPK5ACGUOZEYUWISLZGLXT5C75) | [`CDJNXIKZUNCQDCKVGOPZEKFPDJG2LMNBNQ2N5XE5CGVNCMOAL22AMGZY`](https://stellar.expert/explorer/testnet/contract/CDJNXIKZUNCQDCKVGOPZEKFPDJG2LMNBNQ2N5XE5CGVNCMOAL22AMGZY) |
| Test WBTC | [`CCIMS6WN2MEBVWEGOYET46ZVBXDZU4AGDFMKETDAQUVRUM4IV5U7NEBW`](https://stellar.expert/explorer/testnet/contract/CCIMS6WN2MEBVWEGOYET46ZVBXDZU4AGDFMKETDAQUVRUM4IV5U7NEBW) | [`CA654TSPI6WHJPOO2BSVXFIKBH7G3F66SKST6WDXKAT2SG3DDKDHZZAG`](https://stellar.expert/explorer/testnet/contract/CA654TSPI6WHJPOO2BSVXFIKBH7G3F66SKST6WDXKAT2SG3DDKDHZZAG) |
| Test WSOL | [`CAGDBMJRJFITACJEQQ5YP2QDZ4K6RW4W5LGIESSU7FGBA2XDYJ7CXWVV`](https://stellar.expert/explorer/testnet/contract/CAGDBMJRJFITACJEQQ5YP2QDZ4K6RW4W5LGIESSU7FGBA2XDYJ7CXWVV) | [`CALJ4IL3IBSGORPH2U76HZHYLOHE533TUDJBGR2VWACNRCIROL6QROIP`](https://stellar.expert/explorer/testnet/contract/CALJ4IL3IBSGORPH2U76HZHYLOHE533TUDJBGR2VWACNRCIROL6QROIP) |

Wrapped test assets are not claims on mainnet ETH, BTC, or SOL. Pool seed and reserve evidence is in [`POOLS.md`](POOLS.md).

## WASM hashes

| Contract | Hash |
| --- | --- |
| Basket | `8ab6c816f5ac4fd5c1574415286e41acc97ca2040551179513ee096a975dcded` |
| Basket token | `9283ea8a389f38f888ec9f71d8601a21516f2a58299275d13105c8ec47d314e6` |
| Oracle adapter | `61989e4792a648067699a876977583e746827349b89446bf4bc2f0b541d8518` |
| Settlement | `9db5839c3870c2ac6912b41a57599d8f02d57baa2728714f430a3e6fe79b359a` |
| Factory | `4af23e5d802ced76c20c064a676c88753b112c8c35a09766c5b5b00d0ea12c63` |

## Deployment transactions

| Action | Transaction |
| --- | --- |
| Deploy oracle adapter | [`c6839c74837111745cae496036096604aeb3d023108673cf82260eb96b59bb28`](https://stellar.expert/explorer/testnet/tx/c6839c74837111745cae496036096604aeb3d023108673cf82260eb96b59bb28) |
| Initialize oracle adapter | [`07d12afccfd696f556b669b870e37da7fa13e73d7b9732528bd6c954064beb14`](https://stellar.expert/explorer/testnet/tx/07d12afccfd696f556b669b870e37da7fa13e73d7b9732528bd6c954064beb14) |
| Deploy settlement | [`3ec9376d66f75bf777b6d87976aab8488c2ba8927c4728ee8b5f696356b81372`](https://stellar.expert/explorer/testnet/tx/3ec9376d66f75bf777b6d87976aab8488c2ba8927c4728ee8b5f696356b81372) |
| Initialize settlement | [`4c81799090c196a7ef48ba126ef83bab6277f9b3598af9f6839d76303bdbe3b1`](https://stellar.expert/explorer/testnet/tx/4c81799090c196a7ef48ba126ef83bab6277f9b3598af9f6839d76303bdbe3b1) |
| Deploy factory | [`7006913480de1c67da80ac374b82c324a3cf478752e2e5fdb27654bfb1a783d7`](https://stellar.expert/explorer/testnet/tx/7006913480de1c67da80ac374b82c324a3cf478752e2e5fdb27654bfb1a783d7) |
| Initialize factory | [`77e97655afaa26d85d8ce083280f31769ad68e891f032832fdca7936fe197c50`](https://stellar.expert/explorer/testnet/tx/77e97655afaa26d85d8ce083280f31769ad68e891f032832fdca7936fe197c50) |
| Create basket and token | [`34bc705e3905ae10bd49a47a248cd87234c23aa8310e5add585efb97a89b2cf4`](https://stellar.expert/explorer/testnet/tx/34bc705e3905ae10bd49a47a248cd87234c23aa8310e5add585efb97a89b2cf4) |

## Reproduce

From a clean checkout, set the required testnet environment values and run:

```powershell
.\scripts\deploy-testnet.ps1
```

The script rejects non-testnet passphrases, builds and deploys the Sqim contracts, initializes the stack, creates the four-asset basket, and updates `config/testnet.json`. Private keys are read from the configured Stellar CLI identity and are never written to this repository.
