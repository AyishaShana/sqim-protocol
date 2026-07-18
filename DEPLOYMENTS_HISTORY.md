# Testnet Deployment History

This file is historical evidence only. No runtime code, service, script, or environment example may reference these generations.

## Superseded active generation

| Component | Contract ID |
| --- | --- |
| Factory | `CD5FSGAMGJYZRGMXQFT3B3D3SPIF3SOTJMYZXWLE44K2U4TWTSQVLAUY` |
| Basket | `CCIR6KR56D65BMVLI6RITJNI64XTPJZXGIP6XJCQKMYF24TNILERYGYF` |
| Basket token | `CDGXSZKYKVND7ZWSOGHWSCW6IOR5OWRRNV35HU2GNL4HKX3HK3NJIO27` |
| Settlement | `CD7FCLIV6BKVWLS3SYRCRA2O44VCP6CMOGAFTWWWWXK4Z73CW2X4VFPU` |
| Oracle adapter | `CBVNZE4UO2DSEC6SSCA6S5WMYFZ2NKPIOXQ6DJNG3RQXIGAITKNQCTAS` |

## Discarded verification generations

Two short-lived 2026-07-18 deployments were used to isolate Soroswap budget behavior. They were never promoted to runtime configuration.

| Purpose | Factory | Settlement | Basket | Token |
| --- | --- | --- | --- | --- |
| Typed-error verification | `CBT2...H6EZ` | `CCWN...SWGS` | `CDLR...6SKT` | `CBIZ...VZ4V` |
| Router prequote experiment | `CANCD...RSYK` | `CA4R...OJ2` | `CBRCL...TZUS` | `CBCW...U6WF` |

The router prequote design was rejected because the extra call could exceed Soroban transaction budget. The active settlement executes the router through its typed `try_` client and maps downstream router rejection to Sqim's typed `SlippageExceeded` error.
