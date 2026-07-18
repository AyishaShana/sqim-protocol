import type { PriceSample, RebalanceSuggestion } from "../types";

export function suggestMeanReversion(
  currentTarget: number[],
  originalTarget: number[],
  currentWeights: number[],
  history: PriceSample[],
  maxDriftBps: number,
): RebalanceSuggestion {
  if (
    currentTarget.length === 0 ||
    currentTarget.length !== originalTarget.length ||
    currentTarget.length !== currentWeights.length ||
    currentTarget.reduce((sum, value) => sum + value, 0) !== 10_000 ||
    originalTarget.reduce((sum, value) => sum + value, 0) !== 10_000
  ) {
    throw new Error(
      "Recommendation inputs must contain matching 100% allocations.",
    );
  }

  const desiredTarget =
    history.length >= 3
      ? volatilityAdjustedTarget(originalTarget, history)
      : originalTarget;
  const weights = currentTarget.map((value, index) => {
    const desiredDelta = desiredTarget[index] - value;
    return value + Math.max(-maxDriftBps, Math.min(maxDriftBps, desiredDelta));
  });
  normalizeWithinDrift(weights, currentTarget, maxDriftBps);

  return {
    weights,
    method: "mean-reversion",
    sampleCount: history.length,
    changedAssets: weights.filter(
      (value, index) => value !== currentTarget[index],
    ).length,
    volatilityAdjusted: history.length >= 3,
  };
}

function volatilityAdjustedTarget(
  originalTarget: number[],
  history: PriceSample[],
): number[] {
  const volatilities = originalTarget.map((_, assetIndex) => {
    const returns: number[] = [];
    for (let index = 1; index < history.length; index += 1) {
      const previous = Number(history[index - 1].prices[assetIndex] ?? 0n);
      const current = Number(history[index].prices[assetIndex] ?? 0n);
      if (previous > 0 && current > 0) returns.push(current / previous - 1);
    }
    if (!returns.length) return 0.005;
    const mean =
      returns.reduce((sum, value) => sum + value, 0) / returns.length;
    const variance =
      returns.reduce((sum, value) => sum + (value - mean) ** 2, 0) /
      returns.length;
    return Math.max(0.005, Math.sqrt(variance));
  });
  const scores = originalTarget.map(
    (weight, index) => weight / volatilities[index],
  );
  const scoreTotal = scores.reduce((sum, value) => sum + value, 0);
  const riskTarget = scores.map((score) =>
    Math.round((score * 10_000) / scoreTotal),
  );
  const blended = originalTarget.map((weight, index) =>
    Math.round(weight * 0.75 + riskTarget[index] * 0.25),
  );
  blended[blended.length - 1] +=
    10_000 - blended.reduce((sum, value) => sum + value, 0);
  return blended;
}

function normalizeWithinDrift(
  weights: number[],
  current: number[],
  maxDriftBps: number,
) {
  let remainder = 10_000 - weights.reduce((sum, value) => sum + value, 0);
  while (remainder !== 0) {
    let changed = false;
    for (let index = 0; index < weights.length && remainder !== 0; index += 1) {
      if (remainder > 0) {
        const room = current[index] + maxDriftBps - weights[index];
        if (room <= 0) continue;
        const adjustment = Math.min(room, remainder);
        weights[index] += adjustment;
        remainder -= adjustment;
      } else {
        const room = weights[index] - Math.max(0, current[index] - maxDriftBps);
        if (room <= 0) continue;
        const adjustment = Math.min(room, -remainder);
        weights[index] -= adjustment;
        remainder += adjustment;
      }
      changed = true;
    }
    if (!changed)
      throw new Error("No drift-safe 100% recommendation is available.");
  }
}

export function recordPriceSample(
  basketId: string,
  prices: bigint[],
): PriceSample[] {
  const key = `sqim-price-history:${basketId}`;
  let history: PriceSample[] = [];
  try {
    const stored = JSON.parse(localStorage.getItem(key) || "[]") as {
      at: number;
      prices: string[];
    }[];
    history = stored.map((sample) => ({
      at: sample.at,
      prices: sample.prices.map(BigInt),
    }));
  } catch {
    history = [];
  }
  const now = Date.now();
  if (!history.length || now - history[history.length - 1].at >= 60_000) {
    history.push({ at: now, prices });
  }
  history = history
    .filter((sample) => now - sample.at <= 30 * 24 * 60 * 60 * 1000)
    .slice(-64);
  localStorage.setItem(
    key,
    JSON.stringify(
      history.map((sample) => ({
        at: sample.at,
        prices: sample.prices.map(String),
      })),
    ),
  );
  return history;
}
