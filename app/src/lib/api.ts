import { API_URL } from "../config";
import type {
  AssetHistory,
  BacktestResult,
  NAVHistory,
  UserProfile,
} from "../types";

type ProfileChallenge = { nonce: string; message: string; expires_at: string };

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_URL}${path}`, {
    ...init,
    headers: { "content-type": "application/json", ...(init?.headers ?? {}) },
  });
  const body = await response.json().catch(() => ({}));
  if (!response.ok)
    throw new Error(body.error || `Sqim API returned ${response.status}.`);
  return body as T;
}

export function loadProfile(address: string): Promise<UserProfile> {
  return request(`/profiles/${encodeURIComponent(address)}`);
}

export function requestProfileChallenge(
  address: string,
): Promise<ProfileChallenge> {
  return request(`/profiles/${encodeURIComponent(address)}/challenge`, {
    method: "POST",
  });
}

export function saveProfile(
  profile: UserProfile,
  nonce: string,
  signature: string,
): Promise<UserProfile> {
  return request(`/profiles/${encodeURIComponent(profile.address)}`, {
    method: "PUT",
    body: JSON.stringify({
      ...profile,
      nonce,
      signature,
    }),
  });
}

export function loadNAVHistory(basketID: string): Promise<NAVHistory> {
  return request(`/baskets/${encodeURIComponent(basketID)}/nav-history`);
}

export function loadHistoricalAssets(): Promise<AssetHistory[]> {
  return request("/backtesting/assets");
}

export function runBacktest(
  assets: string[],
  weightsBPS: number[],
  from?: string,
): Promise<BacktestResult> {
  return request("/backtesting/run", {
    method: "POST",
    body: JSON.stringify({
      assets,
      weights_bps: weightsBPS,
      from,
      granularity: "daily",
    }),
  });
}
