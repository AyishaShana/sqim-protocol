export type BasketAsset = {
  address: string;
  symbol?: string;
  name?: string;
};

export type Basket = {
  basket_id: string;
  creator: string;
  name: string;
  share_token_id: string;
  assets: BasketAsset[] | unknown;
  weights_bps: number[] | unknown;
  strategy?: unknown;
  created_at?: string;
};

export type BasketMetrics = {
  nav: string;
  aum: string;
};

export type BasketHistoryEvent = {
  id: number;
  event_id: string;
  basket_id: string;
  account: string;
  event_type: "deposit" | "withdraw" | "rebalance" | "basket-created" | string;
  amount: string;
  shares: string;
  tx_hash: string;
  ledger: number;
  raw: unknown;
  occurred_at: string;
};

export type WalletState = {
  connected: boolean;
  publicKey: string;
  network: string;
};

export type PortfolioPosition = {
  basket: Basket;
  shares: number;
  currentValue: number;
  costBasis: number;
  unrealizedPnL: number;
};
