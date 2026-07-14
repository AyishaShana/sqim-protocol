import { API_URL, DEFAULT_BASKET_CONTRACT_ID } from "../config";
import type { Basket, BasketHistoryEvent, BasketMetrics } from "../types";

const fallbackBasket: Basket = {
  basket_id: DEFAULT_BASKET_CONTRACT_ID,
  creator: "ayisha",
  name: "Sqim Testnet Basket",
  share_token_id: "CARGKEM34YZ4DCNMSSLMOPMIAAJGAF5CKGAAMJWV7E2AI5QFFGPGYEBM",
  assets: [
    {
      address: "CASNUWD2Z4RCUUY4LZEO7XRKHGMNRR55K7BMMUUH6PVIMYZF4IMU5RS4",
      symbol: "SQTA",
      name: "Sqim Test Asset"
    }
  ],
  weights_bps: [10000],
  investable: true,
  preview_only: false,
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
