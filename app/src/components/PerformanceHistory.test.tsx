import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ASSETS } from "../config";
import type { BacktestResult, BasketLive, NAVHistory } from "../types";
import { CreatorBacktest, HistoricalPerformance } from "./PerformanceHistory";

const loadNAVHistory = vi.fn();
const runBacktest = vi.fn();

vi.mock("../lib/api", () => ({
  loadNAVHistory: (...args: unknown[]) => loadNAVHistory(...args),
  runBacktest: (...args: unknown[]) => runBacktest(...args),
}));

const navHistory: NAVHistory = {
  basket_id: "CBASKET",
  deployed_at: "2026-07-01T00:00:00Z",
  source: "Soroban events indexed by Sqim",
  label: "Live on-chain basket NAV since deployment",
  points: [
    {
      at: "2026-07-01T00:00:00Z",
      nav: "1",
      event_type: "deposit",
      tx_hash: "a",
      ledger: 1,
    },
    {
      at: "2026-07-02T00:00:00Z",
      nav: "1.02",
      event_type: "rebalance",
      tx_hash: "b",
      ledger: 2,
    },
  ],
};

const simulation: BacktestResult = {
  series: [
    { at: "2021-06-17T00:00:00Z", value: 100 },
    { at: "2026-07-15T00:00:00Z", value: 155 },
  ],
  total_return: 0.55,
  annualized_volatility: 0.4,
  max_drawdown: -0.5,
  annualized_return: 0.09,
  available_from: "2021-06-17T00:00:00Z",
  available_to: "2026-07-15T00:00:00Z",
  available_years: 5.08,
  youngest_constituent: "SOL",
  granularity: "daily",
  methodology: "Daily target-weight rebalancing on common real close candles.",
  provider: "Binance Public Data",
  disclaimer:
    "Simulated using historical prices, not a live basket track record. Results exclude fees, slippage, taxes, and liquidity constraints and are not investment advice.",
  asset_history: [
    {
      symbol: "XLM",
      name: "Stellar Lumens",
      product_id: "XLMUSDT",
      granularity: "daily",
      available: true,
      first: "2018-05-31T00:00:00Z",
      last: "2026-07-15T00:00:00Z",
      points: 2970,
      provider: "Binance Public Data",
      source_url: "https://data.binance.vision",
    },
    {
      symbol: "SOL",
      name: "Solana",
      product_id: "SOLUSDT",
      granularity: "daily",
      available: true,
      first: "2020-08-11T00:00:00Z",
      last: "2026-07-15T00:00:00Z",
      points: 2166,
      provider: "Binance Public Data",
      source_url: "https://data.binance.vision",
    },
  ],
};

const basket: BasketLive = {
  asOf: "2026-07-18T00:00:00Z",
  basket: "CBASKET",
  basket_token: "CTOKEN",
  creator: "GCREATOR",
  name: "Core Four",
  assets: ASSETS.map((asset) => ({ address: asset.address })),
  target_weights_bps: [4000, 2000, 2000, 2000],
  originalTargetWeights: [4000, 2000, 2000, 2000],
  nav: 10_000_000n,
  aum: 100_000_000n,
  supply: 100_000_000n,
  feeBps: 100,
  maxDriftBps: 1000,
  paused: false,
  holdings: [1n, 1n, 1n, 1n],
  prices: [1n, 1n, 1n, 1n],
  actualWeights: [4000, 2000, 2000, 2000],
};

describe("historical performance boundaries", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    loadNAVHistory.mockResolvedValue(navHistory);
    runBacktest.mockImplementation(
      (_assets: string[], _weights: number[], from?: string) =>
        Promise.resolve(
          from
            ? {
                ...simulation,
                available_from: `${from}T00:00:00Z`,
                series: [
                  { at: `${from}T00:00:00Z`, value: 100 },
                  simulation.series[1],
                ],
              }
            : simulation,
        ),
    );
  });

  afterEach(() => cleanup());

  it("renders live NAV and simulated history as separately labelled records", async () => {
    render(<HistoricalPerformance basket={basket} />);
    expect(
      await screen.findByText("On-chain NAV since deployment"),
    ).toBeTruthy();
    expect(screen.getByText("Historical target-weight model")).toBeTruthy();
    expect(
      screen.getByText(
        /Simulated using historical prices, not a live basket track record/,
      ),
    ).toBeTruthy();
    expect(
      screen.getByText(/Simulated values are never appended/),
    ).toBeTruthy();
  });

  it("runs the creator simulation without presenting it as deployment history", async () => {
    render(
      <CreatorBacktest
        assets={ASSETS.map((asset) => asset.address)}
        weights={[4000, 2000, 2000, 2000]}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Run backtest" }));
    expect(await screen.findByText("5.1 years available")).toBeTruthy();
    expect(screen.getByText(/SOL is youngest; selected window/)).toBeTruthy();
    expect(runBacktest).toHaveBeenCalledTimes(1);
  });
});
