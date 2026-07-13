# Sqim Contract Threat Review

Scope: `/contracts` only. This is a testnet security hardening pass, not a mainnet audit.

## Mainnet Readiness

Sqim should not hold mainnet user funds until the contracts receive an external professional audit. This pass improves obvious controls and adds regression tests, but it is not a substitute for an independent audit, formal review of economic assumptions, or adversarial testing against real liquidity.

## Risks Reviewed

| Area | Risk found | Mitigation added or verified | Residual risk |
| --- | --- | --- | --- |
| User authorization | `deposit` and `withdraw` must only be callable by the acting user. | Existing `depositor.require_auth()` and `holder.require_auth()` checks verified. Regression coverage still checks unauthorized withdraw failure. | Wallet UX must make the signed action and basket address clear. |
| Rebalance authorization | A single relayer key could move composition if quorum is only off-chain. | Existing on-chain M-of-N signer validation verified. Added regression coverage for timelocked rebalancer set changes. | Signer compromise below threshold is tolerated; threshold compromise is not. |
| Oracle fallback | A single fallback key or off-chain-only quorum would allow price manipulation. | Existing on-chain M-of-N fallback price quorum verified. Added timelocked fallback signer/quorum changes and tests. | Fallback signers can still collude at quorum. Use independent operators before mainnet. |
| Stale oracle data | Stale prices could allow wrong NAV, slippage, or rebalance targets. | Existing primary/fallback staleness checks verified. Tests cover stale fallback failure. | Fresh but manipulated prices remain possible if the oracle/fallback quorum is compromised. |
| Cross-contract calls | Soroban contract-to-contract auth differs from EVM reentrancy. Settlement and token calls need explicit authority boundaries. | Basket uses `authorize_as_current_contract` for settlement and share-token mint/asset-transfer subcalls. Settlement requires basket auth for invest/redeem/rebalance. | Review with auditors for malicious token/router contracts and callback-like composition edge cases. |
| Pause control | No circuit breaker existed for active basket operations. | Added admin-only `pause()`/`unpause()`. Pause halts `deposit`, `withdraw`, and `rebalance`. Tests cover all three paths. | Admin can pause availability. Publish operational policy before launch. |
| Admin parameter changes | Fee, drift, slippage, fallback quorum, and rebalancer settings could previously be changed immediately or lacked setters. | Added 24h timelocks for basket fee, drift, rebalancer set, max transaction size, settlement slippage cap, and oracle fallback config. | 24h may be too short for mainnet governance; consider 48h plus public monitoring. |
| Max trade size | One large deposit, withdrawal, or rebalance could overwhelm available AMM liquidity. | Added per-basket max transaction amount. Deposit uses input amount, withdrawal uses gross value, rebalance sums oracle-valued position deltas. Tests cover all three. | Oracle-valued deltas are an approximation; real route liquidity can still be shallow. |
| Slippage cap bypass | Deposit/withdraw/rebalance swaps must all enforce oracle slippage, not only rebalance. | Settlement already enforces slippage in `invest`, `redeem`, and `rebalance`. Added withdrawal and rebalance regression tests. | Mock settlement does not model all Soroswap routing/path behavior. Test with real pools before launch. |
| Integer overflow / precision | NAV, cost basis, fee, and token supply math used some unchecked additions. | Replaced key additions with checked math in basket value/cost-basis/share-token supply and balance updates. Existing checked multiplication retained. | Fixed-point rounding remains conservative but should be audited for edge cases and dust behavior. |
| Token transferability | Basket shares must be real transferable SEP-41-style tokens, not internal ledger units. | Existing transfer test verifies third-party transfer and withdrawal by recipient. | Transferred holders inherit no off-chain UX context; frontend should explain cost-basis effects. |
| Upgrade paths | No contract upgrade entrypoints currently exist in these contracts. | No upgrade path was added. Timelock requirement documented for any future upgrade function. | Future upgradeability must be designed and audited separately. |

## Test Coverage Added

- Pause blocks deposit, withdraw, and rebalance.
- Fee updates cannot execute before the timelock and can execute after it.
- Rebalancer set changes execute only after timelock.
- Max transaction size blocks large deposit, withdrawal, and rebalance.
- Withdrawal and rebalance swaps fail when simulated slippage exceeds tolerance.
- Settlement slippage cap changes are timelocked.
- Oracle fallback config changes are timelocked.
- Oracle fallback price updates require on-chain quorum.

## Launch Notes

- Keep this deployment testnet-only until audit completion.
- Publish all privileged addresses, timelock settings, fallback signers, and rebalancer quorum parameters before any public deposit window.
- Monitor pending admin actions off-chain so users can exit before delayed changes execute.
- Seed and test realistic Soroswap liquidity before allowing meaningful basket sizes.
