# Sqim App

The authenticated Sqim product interface. The marketing and waitlist experience remains in `/site`; this app reads the active testnet factory and submits contract calls directly from a connected wallet.

## Product flows

- Explore every basket created by the configured factory, including live composition, NAV, and AUM.
- Inspect a basket and preview deposits or withdrawals at its current on-chain NAV.
- Sign deposits, withdrawals, rebalances, and basket creation with Freighter or Rabet. Private keys never enter the app.
- View connected-wallet basket positions, current value, average cost, and unrealized P&L.
- Inspect live Soroswap reserves and the connected wallet's LP-token balance for every Core Four pool.
- Compare current and proposed rebalance weights visually, validate the 100% total and per-asset drift bound, and inspect live Soroswap route quotes before signing.
- Build a basket in four deliberate steps: choose at least two supported assets, set an exact 100% allocation, name and describe the mandate, then review a live 10 XLM routing preview before signing the factory deployment.
- Open creator profiles resolved from the factory's creator-to-basket mapping. Profile and notification edits require a fresh wallet-signed challenge; unauthenticated writes are rejected by the API.
- Review a clearly labelled algorithmic suggestion that blends mean reversion toward the original mandate with recent observed volatility. It can pre-fill the editor, but it never submits a transaction and is explicitly not investment advice.
- Store creator rebalance-notification preferences for off/weekly/on-drift-only email reminders. Delivery is intentionally a later service concern; no preference can trigger an on-chain action.
- Compare indexed on-chain NAV since deployment with a separate, clearly labelled historical simulation. The two series are never blended into one track record.
- Backtest a proposed creator allocation over 1Y, 3Y, 5Y, or maximum available history and inspect return, annualized volatility, and drawdown before deployment.
- Follow every write through wallet approval, network submission, confirmation, or an honest failure state with a testnet transaction link.

## Run locally

```bash
cd app
npm install
npm run dev
```

Open `http://127.0.0.1:8080`. Set the wallet network to Stellar Testnet before connecting.

```bash
npm test
npm run lint
npm run build
```

## Configuration

Copy values from `.env.example` into `.env.local` when testing a new deployment. Defaults point to the current Sqim testnet factory, active basket/token pair, settlement, and oracle contracts. The API default is `http://localhost:8081`; port `8080` is reserved for the product app. `VITE_*` values are public client configuration and must never contain secret keys.

On startup the app reads factory metadata, verifies the active basket/token mapping, and resolves the basket, token, settlement, and oracle views. A mismatch replaces the product routes with a blocking error screen, so a stale generation cannot submit transactions. Basket discovery begins at the factory, and signed writes are reported as successful only after testnet confirmation.

Creator profile writes use `POST /profiles/{address}/challenge` followed by a wallet-signed `PUT /profiles/{address}`. Challenges expire after five minutes and are single-use. Freighter message signing is supported directly; Rabet is supported when its installed version exposes `signMessage`.

Historical simulations are served through the API from the separate Go backtester on `http://localhost:8090`. The service stores real Binance USDT-quoted daily candles in Parquet and starts each basket at the youngest constituent's first shared date. Unsupported Stellar-native assets return an explicit unavailable-data error. Every simulated chart is labelled as historical-price research, not a live Sqim basket track record.
