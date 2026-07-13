import { API_URL, DEFAULT_BASKET_CONTRACT_ID } from "../config";
import type { Basket, BasketHistoryEvent, BasketMetrics } from "../types";

const fallbackBasket: Basket = {
  basket_id: DEFAULT_BASKET_CONTRACT_ID,
  creator: "ayisha",
  name: "Sqim Testnet Basket",
  share_token_id: "",
  assets: [
    { address: "native", symbol: "XLM", name: "Stellar Lumens" },
    { address: "test-usdc", symbol: "USDC", name: "Testnet USDC" },
    { address: "test-btc", symbol: "BTC", name: "Testnet BTC" }
  ],
  weights_bps: [5000, 3000, 2000],
  created_at: new Date().toISOString()
};

export async function fetchBaskets(): Promise<Basket[]> {
  const response = await fetch(`${API_URL}/baskets`);
  if (!response.ok) throw new Error("Basket API unavailable");
  const baskets = (await response.json()) as Basket[];
  return baskets.length > 0 ? baskets : [fallbackBasket];
}

export async function fetchBasketHistory(basketID: string): Promise<BasketHistoryEvent[]> {
  const response = await fetch(`${API_URL}/baskets/${basketID}/history?limit=100`);
  if (!response.ok) return [];
  return (await response.json()) as BasketHistoryEvent[];
}

export async function fetchMetrics(basketID: string): Promise<BasketMetrics> {
  const response = await fetch(`${API_URL}/baskets/${basketID}/metrics`);
  if (!response.ok) {
    return { nav: "1.00", aum: "0" };
  }
  return (await response.json()) as BasketMetrics;
}

export function fallbackBaskets(): Basket[] {
  return [fallbackBasket];
}
