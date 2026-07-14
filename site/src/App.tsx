import { useEffect, useMemo, useState } from "react";
import {
  Activity,
  ArrowDownToLine,
  ArrowUpRight,
  Boxes,
  CircleDollarSign,
  RefreshCcw,
  ShieldCheck,
  Sparkles
} from "lucide-react";
import {
  API_URL,
  ENABLE_CONTRACT_WRITES,
  FACTORY_CONTRACT_ID,
  INVESTABLE_BASKET_IDS,
  NETWORK_PASSPHRASE,
  RPC_URL
} from "./config";
import { fetchBasketHistory, fetchBaskets, fetchMetrics, fallbackBaskets } from "./lib/api";
import { compactAddress, formatNumber, formatPercent, formatToken, toNumber } from "./lib/format";
import { positionFromHistory, previewWithdrawal } from "./lib/portfolio";
import {
  addressArg,
  addressVecArg,
  assetVecArg,
  connectFreighter,
  i128Arg,
  signAndSubmitContractCall,
  u32VecArg
} from "./lib/wallet";
import type { Basket, BasketAsset, BasketHistoryEvent, BasketMetrics, WalletState } from "./types";
import { WalletButton } from "./components/WalletButton";
import { WeightBreakdown } from "./components/WeightBreakdown";
import { PerformanceChart } from "./components/PerformanceChart";

type Screen = "explore" | "portfolio" | "creator";

const emptyWallet: WalletState = { connected: false, publicKey: "", network: "" };

export function App() {
  const [screen, setScreen] = useState<Screen>("explore");
  const [baskets, setBaskets] = useState<Basket[]>([]);
  const [selectedID, setSelectedID] = useState("");
  const [metrics, setMetrics] = useState<Record<string, BasketMetrics>>({});
  const [history, setHistory] = useState<Record<string, BasketHistoryEvent[]>>({});
  const [wallet, setWallet] = useState<WalletState>(emptyWallet);
  const [busyWallet, setBusyWallet] = useState(false);
  const [notice, setNotice] = useState("");
  const [apiOnline, setApiOnline] = useState(true);

  useEffect(() => {
    void loadBaskets();
  }, []);

  useEffect(() => {
    for (const basket of baskets) {
      void refreshBasket(basket.basket_id);
    }
  }, [baskets]);

  const selected = baskets.find((basket) => basket.basket_id === selectedID) ?? baskets[0];

  async function loadBaskets() {
    try {
      const loaded = await fetchBaskets();
      setBaskets(loaded);
      setSelectedID(loaded[0]?.basket_id ?? "");
      setApiOnline(true);
    } catch {
      const fallback = fallbackBaskets();
      setBaskets(fallback);
      setSelectedID(fallback[0].basket_id);
      setApiOnline(false);
    }
  }

  async function refreshBasket(basketID: string) {
    const [nextMetrics, nextHistory] = await Promise.all([
      fetchMetrics(basketID),
      fetchBasketHistory(basketID)
    ]);
    setMetrics((current) => ({ ...current, [basketID]: nextMetrics }));
    setHistory((current) => ({ ...current, [basketID]: nextHistory }));
  }

  async function connectWallet() {
    setBusyWallet(true);
    setNotice("");
    try {
      const connected = await connectFreighter();
      setWallet({ connected: true, publicKey: connected.publicKey, network: connected.network });
      setNotice("Wallet connected. Private keys stay inside Freighter.");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Wallet connection failed.");
    } finally {
      setBusyWallet(false);
    }
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <a className="brand" href="#">
          <span>S</span>
          Sqim
        </a>
        <nav>
          <button className={screen === "explore" ? "active" : ""} onClick={() => setScreen("explore")}>
            <Boxes size={18} /> Basket explorer
          </button>
          <button className={screen === "portfolio" ? "active" : ""} onClick={() => setScreen("portfolio")}>
            <Activity size={18} /> Portfolio
          </button>
          <button className={screen === "creator" ? "active" : ""} onClick={() => setScreen("creator")}>
            <Sparkles size={18} /> Creator dashboard
          </button>
        </nav>
        <div className="network-card">
          <ShieldCheck size={18} />
          <span>Testnet configuration</span>
          <em>{NETWORK_PASSPHRASE}</em>
        </div>
      </aside>

      <main>
        <header className="topbar">
          <div>
            <span className="eyebrow">Diversified exposure, one basket token</span>
            <h1>{titleFor(screen)}</h1>
          </div>
          <WalletButton wallet={wallet} onConnect={connectWallet} busy={busyWallet} />
        </header>

        {!apiOnline && (
          <div className="notice warning">
            Local API at {API_URL} is not reachable, so the explorer is showing the deployed testnet basket
            placeholder. Basket lists and history are still designed to come from the API, not direct chain
            polling.
          </div>
        )}
        {notice && <div className="notice">{notice}</div>}

        {screen === "explore" && selected && (
          <Explorer
            baskets={baskets}
            selected={selected}
            metrics={metrics[selected.basket_id]}
            history={history[selected.basket_id] ?? []}
            onSelect={setSelectedID}
            onRefresh={() => refreshBasket(selected.basket_id)}
            wallet={wallet}
            onNotice={setNotice}
          />
        )}
        {screen === "portfolio" && (
          <Portfolio
            baskets={baskets}
            metrics={metrics}
            history={history}
            wallet={wallet}
            onConnect={connectWallet}
          />
        )}
        {screen === "creator" && (
          <CreatorDashboard wallet={wallet} baskets={baskets} onNotice={setNotice} />
        )}
      </main>
    </div>
  );
}

function Explorer(props: {
  baskets: Basket[];
  selected: Basket;
  metrics?: BasketMetrics;
  history: BasketHistoryEvent[];
  onSelect: (id: string) => void;
  onRefresh: () => void;
  wallet: WalletState;
  onNotice: (message: string) => void;
}) {
  const { selected, metrics } = props;
  const nav = toNumber(metrics?.nav, 1);
  const aum = toNumber(metrics?.aum, 0);
  const assets = normalizedAssets(selected);
  const weights = normalizedWeights(selected, assets.length);

  return (
    <section className="grid explorer-grid">
      <div className="panel basket-list">
        <div className="panel-head">
          <span>Live baskets</span>
          <button onClick={props.onRefresh}>
            <RefreshCcw size={16} /> Refresh
          </button>
        </div>
        {props.baskets.map((basket) => (
          <button
            className={basket.basket_id === selected.basket_id ? "basket-row selected" : "basket-row"}
            key={basket.basket_id}
            onClick={() => props.onSelect(basket.basket_id)}
          >
            <strong>{basket.name || "Sqim Basket"}</strong>
            <span>{compactAddress(basket.basket_id)}</span>
          </button>
        ))}
      </div>

      <div className="panel hero-panel">
        <span className="eyebrow">Basket detail</span>
        <h2>{selected.name || "Diversified Basket"}</h2>
        <p>
          One liquid basket token representing proportional exposure to the underlying assets. The interface
          frames this as diversified ownership, not active trading.
        </p>
        <div className="stats-grid">
          <Metric label="NAV" value={formatNumber(nav, { minimumFractionDigits: 2 })} />
          <Metric label="AUM" value={formatNumber(aum)} />
          <Metric label="Assets" value={String(assets.length)} />
        </div>
        <WeightBreakdown assets={assets} weights={weights} />
      </div>

      <DepositWithdrawPanel
        basket={selected}
        nav={nav}
        history={props.history}
        wallet={props.wallet}
        onNotice={props.onNotice}
        onRefresh={props.onRefresh}
      />

      <PerformanceChart history={props.history} />
    </section>
  );
}

function DepositWithdrawPanel({
  basket,
  nav,
  history,
  wallet,
  onNotice,
  onRefresh
}: {
  basket: Basket;
  nav: number;
  history: BasketHistoryEvent[];
  wallet: WalletState;
  onNotice: (message: string) => void;
  onRefresh: () => void;
}) {
  const [depositAmount, setDepositAmount] = useState("100");
  const [burnShares, setBurnShares] = useState("10");
  const [submitting, setSubmitting] = useState(false);
  const amount = toNumber(depositAmount);
  const shares = nav > 0 ? amount / nav : 0;
  const position = positionFromHistory(basket, history, wallet.publicKey, nav);
  const withdrawPreview = previewWithdrawal(position, toNumber(burnShares), nav);
  const canWrite = isInvestableBasket(basket);

  async function submitDeposit() {
    await submit("deposit", [addressArg(wallet.publicKey), i128Arg(amount)]);
  }

  async function submitWithdraw() {
    await submit("withdraw", [addressArg(wallet.publicKey), i128Arg(toNumber(burnShares))]);
  }

  async function submit(method: string, args: unknown[]) {
    if (!canWrite) {
      onNotice(
        "This basket is a read-only preview. Configure VITE_SQIM_INVESTABLE_BASKET_IDS with a factory-created, initialized basket and set VITE_SQIM_ENABLE_CONTRACT_WRITES=true before submitting deposits or withdrawals."
      );
      return;
    }
    if (!wallet.connected) {
      onNotice("Connect Freighter before signing. Sqim never touches private keys.");
      return;
    }
    setSubmitting(true);
    try {
      const hash = await signAndSubmitContractCall(wallet.publicKey, basket.basket_id, method, args);
      onNotice(`Submitted ${method}. Transaction hash: ${hash}`);
      window.setTimeout(onRefresh, 3500);
    } catch (error) {
      onNotice(error instanceof Error ? error.message : `${method} failed.`);
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="panel action-panel">
      <div className="flow-card">
        <ArrowDownToLine size={20} />
        <h3>Deposit into diversified exposure</h3>
        <label>
          Deposit amount
          <input value={depositAmount} onChange={(event) => setDepositAmount(event.target.value)} inputMode="decimal" />
        </label>
        <PreviewLine label="Estimated basket tokens" value={formatToken(shares)} />
        {!canWrite && <p className="read-only-note">Preview only: live deposits require an initialized basket.</p>}
        <button onClick={submitDeposit} disabled={submitting || !canWrite}>
          {canWrite ? "Sign deposit" : "Deposit disabled"}
        </button>
      </div>
      <div className="flow-card subtle">
        <ArrowUpRight size={20} />
        <h3>Withdraw basket shares</h3>
        <label>
          Basket tokens to burn
          <input value={burnShares} onChange={(event) => setBurnShares(event.target.value)} inputMode="decimal" />
        </label>
        <PreviewLine label="Gross payout estimate" value={formatToken(toNumber(burnShares) * nav)} />
        <PreviewLine label="Profit-only fee estimate" value={formatToken(withdrawPreview.fee)} />
        <PreviewLine label="Estimated net payout" value={formatToken(withdrawPreview.net)} />
        {!canWrite && <p className="read-only-note">Preview only: live withdrawals require an initialized basket.</p>}
        <button onClick={submitWithdraw} disabled={submitting || !canWrite}>
          {canWrite ? "Sign withdraw" : "Withdraw disabled"}
        </button>
      </div>
    </div>
  );
}

function Portfolio({
  baskets,
  metrics,
  history,
  wallet,
  onConnect
}: {
  baskets: Basket[];
  metrics: Record<string, BasketMetrics>;
  history: Record<string, BasketHistoryEvent[]>;
  wallet: WalletState;
  onConnect: () => void;
}) {
  const positions = useMemo(
    () =>
      baskets.map((basket) =>
        positionFromHistory(
          basket,
          history[basket.basket_id] ?? [],
          wallet.publicKey,
          toNumber(metrics[basket.basket_id]?.nav, 1)
        )
      ),
    [baskets, history, metrics, wallet.publicKey]
  );
  const totalValue = positions.reduce((sum, position) => sum + position.currentValue, 0);
  const totalPnL = positions.reduce((sum, position) => sum + position.unrealizedPnL, 0);

  if (!wallet.connected) {
    return (
      <section className="panel empty-state">
        <CircleDollarSign size={36} />
        <h2>Connect to see your diversified positions</h2>
        <p>Your wallet signs locally. The app reads indexed basket history from the Sqim API.</p>
        <button onClick={onConnect}>Connect Freighter</button>
      </section>
    );
  }

  return (
    <section className="grid portfolio-grid">
      <Metric label="Current value" value={formatNumber(totalValue)} />
      <Metric label="Unrealized P&L" value={formatNumber(totalPnL)} tone={totalPnL >= 0 ? "positive" : "negative"} />
      <div className="panel positions-panel">
        <div className="panel-head">
          <span>Your diversified positions</span>
          <em>{compactAddress(wallet.publicKey)}</em>
        </div>
        {positions.map((position) => {
          const nav = toNumber(metrics[position.basket.basket_id]?.nav, 1);
          const withdrawal = previewWithdrawal(position, Math.min(position.shares, position.shares * 0.25), nav);
          return (
            <div className="position-row" key={position.basket.basket_id}>
              <div>
                <strong>{position.basket.name || "Sqim Basket"}</strong>
                <span>{formatToken(position.shares)} basket tokens</span>
              </div>
              <div>
                <em>{formatNumber(position.currentValue)}</em>
                <small className={position.unrealizedPnL >= 0 ? "positive-text" : "negative-text"}>
                  {formatNumber(position.unrealizedPnL)} unrealized
                </small>
              </div>
              <div className="fee-note">
                Fee preview: {formatToken(withdrawal.fee)} on profit only, net {formatToken(withdrawal.net)}
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}

function CreatorDashboard({
  wallet,
  baskets,
  onNotice
}: {
  wallet: WalletState;
  baskets: Basket[];
  onNotice: (message: string) => void;
}) {
  const [name, setName] = useState("Balanced Testnet Basket");
  const [assetText, setAssetText] = useState("CASNUWD2Z4RCUUY4LZEO7XRKHGMNRR55K7BMMUUH6PVIMYZF4IMU5RS4");
  const [weights, setWeights] = useState("10000");
  const [rebalanceWeights, setRebalanceWeights] = useState("10000");
  const [selectedBasket, setSelectedBasket] = useState(baskets[0]?.basket_id ?? "");
  const feeRevenue = 0;

  async function createBasket() {
    if (!ENABLE_CONTRACT_WRITES) {
      return onNotice(
        "Contract writes are disabled for local preview. Set VITE_SQIM_ENABLE_CONTRACT_WRITES=true only after configuring initialized testnet contracts."
      );
    }
    if (!wallet.connected) return onNotice("Connect Freighter before creating a basket.");
    const assetAddresses = assetText.split(",").map((part) => part.trim()).filter(Boolean);
    const parsedWeights = weights.split(",").map((part) => Number(part.trim()));
    try {
      const hash = await signAndSubmitContractCall(wallet.publicKey, FACTORY_CONTRACT_ID, "create_basket", [
        addressArg(wallet.publicKey),
        name,
        assetVecArg(assetAddresses),
        u32VecArg(parsedWeights)
      ]);
      onNotice(`Create basket submitted: ${hash}`);
    } catch (error) {
      onNotice(error instanceof Error ? error.message : "Create basket failed.");
    }
  }

  async function rebalance() {
    if (!ENABLE_CONTRACT_WRITES || !isInvestableBasket({ basket_id: selectedBasket } as Basket)) {
      return onNotice(
        "Manual rebalance is disabled for preview baskets. Use a factory-created initialized basket and enable contract writes first."
      );
    }
    if (!wallet.connected) return onNotice("Connect Freighter before rebalancing.");
    const parsedWeights = rebalanceWeights.split(",").map((part) => Number(part.trim()));
    try {
      const hash = await signAndSubmitContractCall(wallet.publicKey, selectedBasket, "rebalance", [
        addressArg(wallet.publicKey),
        u32VecArg(parsedWeights),
        addressVecArg([wallet.publicKey])
      ]);
      onNotice(`Manual rebalance submitted: ${hash}`);
    } catch (error) {
      onNotice(error instanceof Error ? error.message : "Manual rebalance failed.");
    }
  }

  return (
    <section className="grid creator-grid">
      <div className="panel">
        <span className="eyebrow">Creator tools</span>
        <h2>Create a diversified basket</h2>
        <p>Define assets and target weights. The resulting basket token is designed as a transferable share.</p>
        <label>
          Basket name
          <input value={name} onChange={(event) => setName(event.target.value)} />
        </label>
        <label>
          Asset contract IDs
          <textarea value={assetText} onChange={(event) => setAssetText(event.target.value)} />
        </label>
        <label>
          Target weights in BPS
          <input value={weights} onChange={(event) => setWeights(event.target.value)} />
        </label>
        <button onClick={createBasket}>Sign create basket</button>
      </div>

      <div className="panel">
        <span className="eyebrow">Manual rebalance</span>
        <h2>Adjust composition within guardrails</h2>
        <p>Use this for manually managed baskets. Algorithmic baskets should use the M-of-N relayer path.</p>
        <label>
          Basket
          <select value={selectedBasket} onChange={(event) => setSelectedBasket(event.target.value)}>
            {baskets.map((basket) => (
              <option key={basket.basket_id} value={basket.basket_id}>
                {basket.name || compactAddress(basket.basket_id)}
              </option>
            ))}
          </select>
        </label>
        <label>
          New weights in BPS
          <input value={rebalanceWeights} onChange={(event) => setRebalanceWeights(event.target.value)} />
        </label>
        <button onClick={rebalance}>Sign manual rebalance</button>
      </div>

      <div className="panel revenue-panel">
        <span className="eyebrow">Creator revenue</span>
        <h2>{formatNumber(feeRevenue)}</h2>
        <p>Withdrawal fees are shown only when realized profit exists. Loss withdrawals preview a zero fee.</p>
      </div>
    </section>
  );
}

function Metric({ label, value, tone }: { label: string; value: string; tone?: "positive" | "negative" }) {
  return (
    <div className={`metric ${tone ?? ""}`}>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function PreviewLine({ label, value }: { label: string; value: string }) {
  return (
    <div className="preview-line">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function normalizedAssets(basket: Basket): BasketAsset[] {
  if (Array.isArray(basket.assets) && basket.assets.length > 0) {
    return basket.assets.map((asset, index) => {
      if (typeof asset === "object" && asset !== null && "address" in asset) {
        return asset as BasketAsset;
      }
      return { address: String(asset), symbol: `Asset ${index + 1}` };
    });
  }
  return [
    { address: "native", symbol: "XLM" },
    { address: "test-usdc", symbol: "USDC" },
    { address: "test-btc", symbol: "BTC" }
  ];
}

function normalizedWeights(basket: Basket, assetCount: number): number[] {
  if (Array.isArray(basket.weights_bps) && basket.weights_bps.length > 0) {
    return basket.weights_bps.map(Number);
  }
  return Array.from({ length: assetCount }, (_, index) => (index === 0 ? 5000 : index === 1 ? 3000 : 2000));
}

function isInvestableBasket(basket: Basket) {
  return (
    ENABLE_CONTRACT_WRITES &&
    basket.investable === true &&
    INVESTABLE_BASKET_IDS.includes(basket.basket_id)
  );
}

function titleFor(screen: Screen) {
  if (screen === "portfolio") return "Your diversified positions";
  if (screen === "creator") return "Creator dashboard";
  return "Basket explorer";
}
