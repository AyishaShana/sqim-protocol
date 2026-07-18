import { describe, expect, it } from "vitest";
import { suggestMeanReversion } from "./recommendation";

describe("algorithmic rebalance suggestion", () => {
  it("moves toward the original mandate while respecting drift and 100 percent", () => {
    const suggestion = suggestMeanReversion(
      [4000, 2000, 2000, 2000],
      [2500, 2500, 2500, 2500],
      [4100, 1900, 2000, 2000],
      [{ at: 1, prices: [10n, 20n, 30n, 40n] }],
      1000,
    );

    expect(suggestion.weights.reduce((sum, value) => sum + value, 0)).toBe(
      10_000,
    );
    suggestion.weights.forEach((value, index) => {
      expect(
        Math.abs(value - [4000, 2000, 2000, 2000][index]),
      ).toBeLessThanOrEqual(1000);
    });
    expect(suggestion.sampleCount).toBe(1);
    expect(suggestion.volatilityAdjusted).toBe(false);
  });

  it("activates a bounded volatility adjustment when recent history is available", () => {
    const suggestion = suggestMeanReversion(
      [5000, 5000],
      [5000, 5000],
      [5000, 5000],
      [
        { at: 1, prices: [100n, 100n] },
        { at: 2, prices: [101n, 130n] },
        { at: 3, prices: [102n, 90n] },
      ],
      1000,
    );
    expect(suggestion.volatilityAdjusted).toBe(true);
    expect(suggestion.weights[0]).toBeGreaterThan(suggestion.weights[1]);
    expect(suggestion.weights.reduce((sum, value) => sum + value, 0)).toBe(
      10_000,
    );
  });

  it("rejects malformed allocations", () => {
    expect(() =>
      suggestMeanReversion([5000], [5000], [5000], [], 1000),
    ).toThrow("100%");
  });
});
