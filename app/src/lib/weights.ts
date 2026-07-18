export type WeightViolation = {
  index: number;
  drift: number;
  excess: number;
};

export function sumWeights(weights: number[]): number {
  return weights.reduce((total, value) => total + value, 0);
}

export function normalizeWeights(weights: number[], total = 10_000): number[] {
  if (!weights.length) return [];
  const clean = weights.map((value) => Math.max(0, Math.round(value)));
  const current = sumWeights(clean);
  if (current === 0) {
    const equal = Math.floor(total / clean.length);
    return clean.map((_, index) =>
      index === clean.length - 1 ? total - equal * (clean.length - 1) : equal,
    );
  }
  let allocated = 0;
  return clean.map((value, index) => {
    if (index === clean.length - 1) return total - allocated;
    const normalized = Math.round((value * total) / current);
    allocated += normalized;
    return normalized;
  });
}

export function setBalancedWeight(
  weights: number[],
  changedIndex: number,
  nextValue: number,
): number[] {
  if (!weights.length || changedIndex < 0 || changedIndex >= weights.length)
    return weights;
  if (weights.length === 1) return [10_000];
  const changed = Math.max(0, Math.min(10_000, Math.round(nextValue)));
  const remainder = 10_000 - changed;
  const otherTotal = weights.reduce(
    (total, value, index) =>
      index === changedIndex ? total : total + Math.max(0, value),
    0,
  );
  const next = weights.map((value, index) => {
    if (index === changedIndex) return changed;
    return otherTotal > 0
      ? (Math.max(0, value) * remainder) / otherTotal
      : remainder / (weights.length - 1);
  });
  const normalizedOthers = normalizeWeights(
    next.filter((_, index) => index !== changedIndex),
    remainder,
  );
  let cursor = 0;
  return next.map((_, index) =>
    index === changedIndex ? changed : normalizedOthers[cursor++],
  );
}

export function weightViolations(
  current: number[],
  proposed: number[],
  maxDriftBps: number,
): WeightViolation[] {
  return proposed.flatMap((value, index) => {
    const drift = Math.abs(value - (current[index] ?? 0));
    return drift > maxDriftBps
      ? [{ index, drift, excess: drift - maxDriftBps }]
      : [];
  });
}

export function rebalanceAuthorization(
  walletAddress: string | undefined,
  creator: string,
  rebalancers: readonly string[],
  threshold: number,
): { allowed: boolean; reason: string } {
  if (!walletAddress) {
    return {
      allowed: false,
      reason: "Connect the creator wallet to propose a rebalance.",
    };
  }
  if (walletAddress === creator) {
    return { allowed: true, reason: "" };
  }
  if (rebalancers.includes(walletAddress)) {
    return {
      allowed: false,
      reason: `This address is one rebalancer, but the basket requires ${threshold}-of-${rebalancers.length} on-chain signatures. Use the multisig relayer flow.`,
    };
  }
  return {
    allowed: false,
    reason:
      "Only the basket creator or configured rebalancer quorum can rebalance this basket.",
  };
}
