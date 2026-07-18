# Future Scope

These are deliberate v2 items, not missing v1 work.

## Per-Strategy Containers

Sqim v1 uses a small rule-based relayer with reviewed strategies. Running arbitrary strategy code in separate containers is intentionally out of scope until there is a clearer security model for sandboxing, permissions, resource limits, and review.

## Advanced Backtesting

The v1 backtester now stores real public candles in Parquet, restricts each simulation to dates shared by every constituent, and reports return, volatility, and drawdown. Execution-aware modeling remains v2: historical Soroswap liquidity, route-by-route slippage, gas/resource fees, withdrawal fees, configurable rebalance schedules, taxes, and strategy parameter sweeps are deliberately excluded. The current product says so on every result rather than presenting a price-only simulation as a realizable track record.

## Notification Service

Creators can already store wallet-authenticated weekly or drift-only email preferences in the v1 API, reusing the same email field shape as the waitlist capture. Delivery workers, email sending, webhooks, and push notifications remain deferred; preferences never authorize a rebalance or any other on-chain action.

## Service Split

Metrics, discovery, user profile reads, and public REST endpoints stay in the single API service for v1. They can be separated only when load or ownership boundaries justify the extra operational cost.
