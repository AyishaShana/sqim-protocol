export type WalletKind = "freighter" | "rabet";
export type WalletState = { kind: WalletKind; address: string } | null;
export type TxPhase = "idle" | "wallet" | "pending" | "success" | "failed";
export type TxState = { phase: TxPhase; hash?: string; error?: string };

export type BasketSpec = {
  basket: string;
  basket_token: string;
  creator: string;
  name: string;
  assets: { address: string }[];
  target_weights_bps: number[];
};

export type BasketLive = BasketSpec & {
  originalTargetWeights: number[];
  nav: bigint;
  aum: bigint;
  supply: bigint;
  feeBps: number;
  maxDriftBps: number;
  paused: boolean;
  holdings: bigint[];
  prices: bigint[];
  actualWeights: number[];
  asOf: string;
};

export type UserProfile = {
  address: string;
  display_name: string;
  bio: string;
  avatar_url: string;
  notification_frequency: "off" | "weekly" | "on-drift-only";
  drift_threshold_bps: number;
  notification_email: string;
  updated_at?: string;
};

export type PriceSample = { at: number; prices: bigint[] };

export type RebalanceSuggestion = {
  weights: number[];
  method: "mean-reversion";
  sampleCount: number;
  changedAssets: number;
  volatilityAdjusted: boolean;
};

export type Position = {
  balance: bigint;
  trackedShares: bigint;
  averageCostPerShare: bigint;
  currentValue: bigint;
  unrealizedPnl: bigint;
};

export type SwapQuote = {
  side: "sell" | "buy";
  symbol: string;
  amountIn: bigint;
  amountOut: bigint;
  inputSymbol: string;
  outputSymbol: string;
};

export type DepositAllocation = {
  asset: string;
  symbol: string;
  weightBps: number;
  inputAmount: bigint;
  outputAmount: bigint;
  route: "retain" | "swap";
};

export type PoolLive = {
  pair: string;
  symbol: string;
  assetSymbol: string;
  assetReserve: bigint;
  xlmReserve: bigint;
  lpBalance?: bigint;
  error?: string;
};

export type NAVHistoryPoint = {
  at: string;
  nav: string;
  event_type: string;
  tx_hash: string;
  ledger: number;
};

export type NAVHistory = {
  basket_id: string;
  deployed_at: string;
  source: string;
  label: string;
  points: NAVHistoryPoint[];
};

export type AssetHistory = {
  symbol: string;
  name: string;
  product_id: string;
  granularity: string;
  available: boolean;
  first?: string;
  last?: string;
  points: number;
  provider: string;
  source_url: string;
};

export type BacktestResult = {
  series: { at: string; value: number }[];
  total_return: number;
  annualized_volatility: number;
  max_drawdown: number;
  annualized_return: number;
  available_from: string;
  available_to: string;
  available_years: number;
  youngest_constituent: string;
  granularity: string;
  methodology: string;
  provider: string;
  disclaimer: string;
  asset_history: AssetHistory[];
};
