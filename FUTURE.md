# Future Scope

These are deliberate v2 items, not missing v1 work.

## Per-Strategy Containers

Sqim v1 uses a small rule-based relayer with reviewed strategies. Running arbitrary strategy code in separate containers is intentionally out of scope until there is a clearer security model for sandboxing, permissions, resource limits, and review.

## Backtesting Service

Backtesting is useful, but it is not needed for the first production support loop. Strategy simulation can start as offline research until real basket usage proves which strategy surfaces need product support.

## Notification Service

Alerts, email, webhooks, and push notifications are deferred. The v1 API and indexer expose the data needed to build notifications later without adding another always-on service now.

## Service Split

Metrics, discovery, user profile reads, and public REST endpoints stay in the single API service for v1. They can be separated only when load or ownership boundaries justify the extra operational cost.
