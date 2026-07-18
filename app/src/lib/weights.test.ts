import { describe, expect, it } from "vitest";
import {
  normalizeWeights,
  rebalanceAuthorization,
  setBalancedWeight,
  sumWeights,
  weightViolations,
} from "./weights";

describe("rebalance guard previews", () => {
  it("accepts the exact on-chain drift boundary", () => {
    expect(
      weightViolations(
        [4000, 2000, 2000, 2000],
        [5000, 1500, 1500, 2000],
        1000,
      ),
    ).toEqual([]);
  });

  it("reports the asset and exact excess beyond the boundary", () => {
    expect(
      weightViolations(
        [4000, 2000, 2000, 2000],
        [5100, 1634, 1633, 1633],
        1000,
      ),
    ).toEqual([{ index: 0, drift: 1100, excess: 100 }]);
  });

  it("uses basis points for the 100 percent invariant", () => {
    expect(sumWeights([4000, 2000, 2000, 2000])).toBe(10_000);
    expect(sumWeights([5100, 1634, 1633, 1633])).toBe(10_000);
  });
});

describe("rebalance authorization", () => {
  const creator = "GCREATOR";
  const rebalancers = ["GREBALANCER1", "GREBALANCER2"];

  it("enables the creator and blocks an unrelated wallet before submission", () => {
    expect(
      rebalanceAuthorization(creator, creator, rebalancers, 2).allowed,
    ).toBe(true);
    const unrelated = rebalanceAuthorization(
      "GUNAUTHORIZED",
      creator,
      rebalancers,
      2,
    );
    expect(unrelated.allowed).toBe(false);
    expect(unrelated.reason).toContain("Only the basket creator");
  });

  it("does not treat one rebalancer key as a quorum", () => {
    const singleSigner = rebalanceAuthorization(
      "GREBALANCER1",
      creator,
      rebalancers,
      2,
    );
    expect(singleSigner.allowed).toBe(false);
    expect(singleSigner.reason).toContain("2-of-2");
  });
});

describe("creator allocation controls", () => {
  it("normalizes manual percentages to exactly 100 percent", () => {
    expect(normalizeWeights([50, 25, 25])).toEqual([5000, 2500, 2500]);
    expect(sumWeights(normalizeWeights([3333, 3333, 3333]))).toBe(10_000);
  });

  it("redistributes an auto-balanced edit across the other assets", () => {
    const next = setBalancedWeight([4000, 2000, 2000, 2000], 0, 5000);
    expect(next).toEqual([5000, 1667, 1667, 1666]);
    expect(sumWeights(next)).toBe(10_000);
  });

  it("recovers from an all-zero manual allocation", () => {
    expect(normalizeWeights([0, 0, 0, 0])).toEqual([2500, 2500, 2500, 2500]);
  });
});
