import { describe, expect, it } from "vitest";
import {
  allocateDepositInputs,
  assertActiveFactorySpec,
  enforceOracleQuote,
  formatUnits,
  friendlyError,
  parseUnits,
} from "./stellar";

describe("amount formatting", () => {
  it("round-trips seven-decimal Stellar amounts", () => {
    expect(parseUnits("12.3456789")).toBe(123_456_789n);
    expect(formatUnits(123_456_789n, 7)).toBe("12.3456789");
  });

  it("rejects zero and excess precision", () => {
    expect(() => parseUnits("0")).toThrow("greater than zero");
    expect(() => parseUnits("1.00000001")).toThrow("seven decimals");
  });
});

describe("contract errors", () => {
  it("turns pool protection failures into an actionable message", () => {
    expect(friendlyError(new Error("Error(Contract, #507)"))).toContain(
      "oracle-protected minimum",
    );
  });

  it("does not hide unrecognized failures", () => {
    expect(friendlyError(new Error("RPC unavailable"))).toBe("RPC unavailable");
  });

  it("reports a rejected wallet signature without leaving an ambiguous pending state", () => {
    expect(friendlyError(new Error("User declined the request"))).toBe(
      "You rejected the transaction in your wallet. No changes were made.",
    );
  });

  it("decodes typed settlement and basket failures", () => {
    expect(
      friendlyError(new Error("HostError: Error(Contract, #4008)")),
    ).toContain("slippage limit");
    expect(
      friendlyError(new Error("HostError: Error(Contract, #1019)")),
    ).toContain("maximum drift");
  });
});

describe("oracle-protected quote preview", () => {
  it("accepts a realized route at the configured minimum", () => {
    expect(() =>
      enforceOracleQuote("BTC", 100n, 1_000n, 980n, 200),
    ).not.toThrow();
  });

  it("identifies dust-sized reverse swaps before wallet signing", () => {
    expect(() => enforceOracleQuote("BTC", 29n, 1_000n, 970n, 200)).toThrow(
      "BTC redemption is dust-sized",
    );
  });
});

describe("deployment manifest", () => {
  const spec = {
    basket: "CACTIVE",
    basket_token: "CTOKEN",
    creator: "GCREATOR",
    name: "Active",
    assets: [],
    target_weights_bps: [],
  };

  it("accepts only the basket-token pair registered by the factory", () => {
    expect(assertActiveFactorySpec([spec], "CACTIVE", "CTOKEN")).toBe(spec);
    expect(() => assertActiveFactorySpec([spec], "CSTALE", "CTOKEN")).toThrow(
      "not registered",
    );
    expect(() => assertActiveFactorySpec([spec], "CACTIVE", "CWRONG")).toThrow(
      "not configured token",
    );
  });
});

describe("deposit allocation", () => {
  it("allocates every stroop and gives the final asset the rounding remainder", () => {
    const allocations = allocateDepositInputs(
      [4_000, 2_000, 2_000, 2_000],
      100_000_001n,
    );

    expect(allocations).toEqual([
      40_000_000n,
      20_000_000n,
      20_000_000n,
      20_000_001n,
    ]);
    expect(allocations.reduce((sum, amount) => sum + amount, 0n)).toBe(
      100_000_001n,
    );
  });

  it("rejects an incomplete target allocation", () => {
    expect(() => allocateDepositInputs([5_000, 4_999], 100_000_000n)).toThrow(
      "total 100%",
    );
  });
});
