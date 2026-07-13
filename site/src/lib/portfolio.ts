import { WITHDRAWAL_FEE_BPS } from "../config";
import type { Basket, BasketHistoryEvent, PortfolioPosition } from "../types";
import { toNumber } from "./format";

export function positionFromHistory(
  basket: Basket,
  history: BasketHistoryEvent[],
  account: string,
  nav: number
): PortfolioPosition {
  let shares = 0;
  let costBasis = 0;
  const normalized = account.toLowerCase();

  for (const event of [...history].reverse()) {
    if (event.account.toLowerCase() !== normalized) continue;
    const eventShares = toNumber(event.shares);
    const eventAmount = toNumber(event.amount);
    if (event.event_type === "deposit") {
      shares += eventShares;
      costBasis += eventAmount;
    }
    if (event.event_type === "withdraw") {
      const burned = eventShares > 0 ? eventShares : eventAmount / Math.max(nav, 1);
      const ratio = shares > 0 ? Math.min(1, burned / shares) : 0;
      shares -= burned;
      costBasis -= costBasis * ratio;
    }
  }

  const currentValue = shares * nav;
  return {
    basket,
    shares,
    costBasis,
    currentValue,
    unrealizedPnL: currentValue - costBasis
  };
}

export function previewWithdrawal(position: PortfolioPosition, burnShares: number, nav: number) {
  const gross = burnShares * nav;
  const averageCost = position.shares > 0 ? position.costBasis / position.shares : nav;
  const costBasis = burnShares * averageCost;
  const profit = Math.max(0, gross - costBasis);
  const fee = (profit * WITHDRAWAL_FEE_BPS) / 10_000;
  return {
    gross,
    profit,
    fee,
    net: Math.max(0, gross - fee)
  };
}
