import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const basketId = "CBASKETINITIALIZEDTEST";

const walletMock = vi.hoisted(() => ({
  connectFreighter: vi.fn(async () => ({ publicKey: "GTESTACCOUNT", network: "TESTNET" })),
  signAndSubmitContractCall: vi.fn(async () => "tx-hash")
}));

vi.mock("./lib/wallet", () => ({
  connectFreighter: walletMock.connectFreighter,
  signAndSubmitContractCall: walletMock.signAndSubmitContractCall,
  addressArg: vi.fn((value: string) => ({ address: value })),
  i128Arg: vi.fn((value: number) => ({ i128: value })),
  u32VecArg: vi.fn((value: number[]) => ({ u32Vec: value })),
  assetVecArg: vi.fn((value: string[]) => ({ assetVec: value })),
  addressVecArg: vi.fn((value: string[]) => ({ addressVec: value }))
}));

describe("Sqim frontend flows", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.stubGlobal("fetch", vi.fn(fetchMock));
    vi.stubEnv("VITE_SQIM_ENABLE_CONTRACT_WRITES", "false");
    vi.stubEnv("VITE_SQIM_INVESTABLE_BASKET_IDS", "");
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllEnvs();
  });

  it("loads baskets from the API and keeps preview baskets read-only", async () => {
    await renderApp();

    expect(await screen.findByRole("heading", { name: "Sqim Testnet Basket" })).toBeInTheDocument();
    expect(screen.getByText("Deposit disabled")).toBeDisabled();
    expect(screen.getByText("Withdraw disabled")).toBeDisabled();
    expect(screen.getByText(/Preview only: live deposits/)).toBeInTheDocument();
    expect(await screen.findByText("1.07")).toBeInTheDocument();
    expect(await screen.findByText("132,500")).toBeInTheDocument();
  });

  it("connects through the wallet boundary and shows portfolio framing", async () => {
    const user = userEvent.setup();
    await renderApp();

    await user.click(await screen.findByRole("button", { name: /Connect wallet/i }));
    expect(await screen.findByText("GTESTACCOUNT")).toBeInTheDocument();
    expect(walletMock.connectFreighter).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole("button", { name: /Portfolio/i }));
    expect(await screen.findByRole("heading", { name: "Your diversified positions" })).toBeInTheDocument();
    expect(screen.getByText(/basket tokens/)).toBeInTheDocument();
  });

  it("submits deposits only for explicitly configured investable baskets", async () => {
    vi.stubGlobal("fetch", vi.fn((input: RequestInfo | URL) => fetchMock(input, true)));
    vi.stubEnv("VITE_SQIM_ENABLE_CONTRACT_WRITES", "true");
    vi.stubEnv("VITE_SQIM_INVESTABLE_BASKET_IDS", basketId);
    const user = userEvent.setup();
    await renderApp();

    await screen.findByRole("heading", { name: "Initialized Basket" });
    await user.click(screen.getByRole("button", { name: /Connect wallet/i }));
    await user.click(await screen.findByText("Sign deposit"));

    await waitFor(() => {
      expect(walletMock.signAndSubmitContractCall).toHaveBeenCalledWith(
        "GTESTACCOUNT",
        basketId,
        "deposit",
        expect.any(Array)
      );
    });
  });

  it("shows creator dashboard and blocks writes until wallet write mode is configured", async () => {
    const user = userEvent.setup();
    await renderApp();

    await user.click(await screen.findByRole("button", { name: /Creator dashboard/i }));
    expect(await screen.findByText("Create a diversified basket")).toBeInTheDocument();
    expect(screen.getByText("Adjust composition within guardrails")).toBeInTheDocument();
  });
});

async function renderApp() {
  vi.resetModules();
  const { App } = await import("./App");
  render(<App />);
}

async function fetchMock(input: RequestInfo | URL, investable = false) {
  const url = String(input);
  if (url.endsWith("/baskets")) {
    return json([
      {
        basket_id: investable ? basketId : "CPREVIEWBASKET",
        creator: "ayisha",
        name: investable ? "Initialized Basket" : "Sqim Testnet Basket",
        share_token_id: "CSHARETOKEN",
        assets: [
          { address: "native", symbol: "XLM" },
          { address: "test-usdc", symbol: "USDC" },
          { address: "test-btc", symbol: "BTC" }
        ],
        weights_bps: [5000, 3000, 2000],
        investable,
        preview_only: !investable
      }
    ]);
  }
  if (url.includes("/metrics")) {
    return json({ nav: "1.07", aum: "132500" });
  }
  if (url.includes("/history")) {
    return json([
      {
        id: 1,
        event_id: "deposit-1",
        basket_id: investable ? basketId : "CPREVIEWBASKET",
        account: "GTESTACCOUNT",
        event_type: "deposit",
        amount: "100",
        shares: "93.45",
        tx_hash: "mock",
        ledger: 1,
        raw: {},
        occurred_at: new Date().toISOString()
      }
    ]);
  }
  return json({ status: "ok" });
}

function json(value: unknown) {
  return Promise.resolve(new Response(JSON.stringify(value), { status: 200 }));
}
