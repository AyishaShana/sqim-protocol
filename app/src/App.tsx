import { useEffect, useState, type ReactNode } from "react";
import {
  BrowserRouter,
  Link,
  NavLink,
  Navigate,
  Route,
  Routes,
  useLocation,
  useNavigate,
  useParams,
} from "react-router-dom";
import {
  Activity,
  ArrowDownToLine,
  ArrowLeft,
  ArrowRight,
  ArrowUpFromLine,
  Bell,
  Check,
  CircleAlert,
  Droplets,
  ExternalLink,
  Layers3,
  LoaderCircle,
  Plus,
  RefreshCw,
  Scale,
  ShieldCheck,
  SlidersHorizontal,
  Sparkles,
  UserRound,
  Wallet,
  X,
} from "lucide-react";
import {
  CreatorBacktest,
  HistoricalPerformance,
} from "./components/PerformanceHistory";
import {
  ASSETS,
  FACTORY_ID,
  REBALANCERS,
  REBALANCER_THRESHOLD,
  assetMeta,
  basketPresentation,
} from "./config";
import { loadProfile, requestProfileChallenge, saveProfile } from "./lib/api";
import { recordPriceSample, suggestMeanReversion } from "./lib/recommendation";
import {
  addressArg,
  addressVecArg,
  assetVecArg,
  connectWallet,
  formatUnits,
  friendlyError,
  i128Arg,
  loadBaskets,
  loadCreatorBasketAddresses,
  loadLiquidity,
  loadPosition,
  parseUnits,
  quoteDepositAllocation,
  quoteRebalance,
  quoteWithdrawalPayout,
  signWalletMessage,
  stringArg,
  submitContractCall,
  u32VecArg,
  validateDeployment,
} from "./lib/stellar";
import {
  normalizeWeights,
  rebalanceAuthorization,
  setBalancedWeight,
  sumWeights,
  weightViolations,
} from "./lib/weights";
import type {
  BasketLive,
  DepositAllocation,
  PoolLive,
  Position,
  PriceSample,
  RebalanceSuggestion,
  SwapQuote,
  TxState,
  UserProfile,
  WalletKind,
  WalletState,
} from "./types";

const IDLE_TX: TxState = { phase: "idle" };

export function App() {
  const [baskets, setBaskets] = useState<BasketLive[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState("");
  const [deploymentState, setDeploymentState] = useState<
    "checking" | "ready" | "failed"
  >("checking");
  const [deploymentError, setDeploymentError] = useState("");
  const [wallet, setWallet] = useState<WalletState>(null);
  const [walletOpen, setWalletOpen] = useState(false);

  async function refresh() {
    setLoading(true);
    try {
      setBaskets(await loadBaskets());
      setLoadError("");
    } catch (error) {
      setLoadError(friendlyError(error));
    } finally {
      setLoading(false);
    }
  }

  async function start() {
    setDeploymentState("checking");
    setDeploymentError("");
    setLoading(true);
    try {
      const [, loadedBaskets] = await Promise.all([
        validateDeployment(),
        loadBaskets(),
      ]);
      setBaskets(loadedBaskets);
      setLoadError("");
      setLoading(false);
      setDeploymentState("ready");
    } catch (error) {
      setBaskets([]);
      setLoading(false);
      setDeploymentState("failed");
      setDeploymentError(
        error instanceof Error ? error.message : String(error),
      );
    }
  }

  useEffect(() => {
    void start();
  }, []);

  async function chooseWallet(kind: WalletKind) {
    const address = await connectWallet(kind);
    const next = { kind, address } as const;
    setWallet(next);
    setWalletOpen(false);
  }

  return (
    <BrowserRouter>
      <ScrollToTop />
      <div className="app-shell">
        <header className="topbar">
          <Link to="/" className="brand" aria-label="Sqim explorer">
            <span className="brand-mark" aria-hidden="true">
              <i />
              <i />
              <i />
            </span>
            <span>Sqim</span>
          </Link>
          <nav className="primary-nav" aria-label="Primary navigation">
            <NavLink to="/explore">
              <Layers3 size={17} /> Explore
            </NavLink>
            <NavLink to="/portfolio">
              <Activity size={17} /> Portfolio
            </NavLink>
            <NavLink to="/liquidity">
              <Droplets size={17} /> My LP
            </NavLink>
            <NavLink to="/create">
              <Plus size={17} /> Create
            </NavLink>
            {wallet && (
              <NavLink to={`/creator/${wallet.address}`}>
                <UserRound size={17} /> Profile
              </NavLink>
            )}
          </nav>
          <div className="top-actions">
            <span className="network-pill">
              <i /> Stellar testnet
            </span>
            <button
              className="wallet-button"
              onClick={() => setWalletOpen(true)}
            >
              <Wallet size={17} />{" "}
              {wallet ? compact(wallet.address) : "Connect wallet"}
            </button>
          </div>
        </header>
        <div className="testnet-banner" role="status">
          <ShieldCheck size={14} />
          <strong>TESTNET ONLY</strong>
          <span>
            Demo assets have no real value. Do not send mainnet funds.
          </span>
        </div>

        {(deploymentError || loadError) && (
          <div className="global-error">
            <CircleAlert size={17} /> {deploymentError || loadError}
            <button onClick={deploymentState === "failed" ? start : refresh}>
              <RefreshCw size={15} /> Retry
            </button>
          </div>
        )}
        <main>
          {deploymentState !== "ready" ? (
            <DeploymentGate state={deploymentState} retry={start} />
          ) : (
            <Routes>
              <Route path="/" element={<Navigate to="/explore" replace />} />
              <Route
                path="/explore"
                element={
                  <Explorer
                    baskets={baskets}
                    loading={loading}
                    refresh={refresh}
                  />
                }
              />
              <Route
                path="/basket/:id"
                element={
                  <BasketDetail
                    baskets={baskets}
                    wallet={wallet}
                    onConnect={() => setWalletOpen(true)}
                    refresh={refresh}
                  />
                }
              />
              <Route
                path="/portfolio"
                element={
                  <Portfolio
                    baskets={baskets}
                    wallet={wallet}
                    onConnect={() => setWalletOpen(true)}
                  />
                }
              />
              <Route
                path="/liquidity"
                element={
                  <Liquidity
                    wallet={wallet}
                    onConnect={() => setWalletOpen(true)}
                  />
                }
              />
              <Route
                path="/create"
                element={
                  <CreateBasket
                    wallet={wallet}
                    onConnect={() => setWalletOpen(true)}
                    refresh={refresh}
                  />
                }
              />
              <Route
                path="/creator/:address"
                element={
                  <CreatorProfile
                    baskets={baskets}
                    wallet={wallet}
                    onConnect={() => setWalletOpen(true)}
                  />
                }
              />
              <Route path="/risks" element={<Risks />} />
            </Routes>
          )}
        </main>
        <footer>
          <span>Sqim protocol</span>
          <span>Testnet only. No real assets or funds.</span>
          <Link to="/risks">Risks</Link>
          <a
            href={`https://stellar.expert/explorer/testnet/contract/${FACTORY_ID}`}
            target="_blank"
            rel="noreferrer"
          >
            Factory <ExternalLink size={13} />
          </a>
        </footer>
      </div>
      {walletOpen && (
        <WalletDialog
          wallet={wallet}
          close={() => setWalletOpen(false)}
          choose={chooseWallet}
          disconnect={() => {
            setWallet(null);
            setWalletOpen(false);
          }}
        />
      )}
    </BrowserRouter>
  );
}

function Risks() {
  return (
    <div className="page risks-page">
      <PageHead
        eyebrow="Launch disclosure"
        title="Testnet risk, stated plainly."
        copy="Sqim is an unaudited testnet build for technical evaluation. Demo assets have no real value and no mainnet funds should be sent to these contracts."
      />
      <section className="risk-grid">
        <article>
          <ShieldCheck size={20} />
          <h2>Smart contract risk</h2>
          <p>
            The contracts have automated tests and on-chain guards, but they
            have not completed an independent professional audit. Mainnet
            remains blocked.
          </p>
        </article>
        <article>
          <Activity size={20} />
          <h2>Oracle risk</h2>
          <p>
            Primary prices can become stale or unavailable. The fallback
            requires an on-chain M-of-N quorum, but signer compromise and bad
            source data remain residual risks.
          </p>
        </article>
        <article>
          <Scale size={20} />
          <h2>Liquidity and MEV</h2>
          <p>
            Soroswap execution is checked against oracle prices and capped for
            slippage. Thin testnet pools can still reject otherwise valid
            deposits, withdrawals, or rebalances.
          </p>
        </article>
        <article>
          <RefreshCw size={20} />
          <h2>Testnet instability</h2>
          <p>
            Testnet contracts, accounts, RPC history, and liquidity can be reset
            without notice. Displayed NAV and AUM are timestamped reads, not
            tick-level market data.
          </p>
        </article>
        <article>
          <Bell size={20} />
          <h2>Email status</h2>
          <p>
            Signed creator notification preferences are persisted in Postgres.
            Email delivery and a durable marketing waitlist intake are not
            enabled in this testnet app.
          </p>
        </article>
      </section>
    </div>
  );
}

function ScrollToTop() {
  const { pathname } = useLocation();
  useEffect(() => {
    window.scrollTo({ top: 0, left: 0, behavior: "instant" });
  }, [pathname]);
  return null;
}

function DeploymentGate({
  state,
  retry,
}: {
  state: "checking" | "ready" | "failed";
  retry(): Promise<void>;
}) {
  if (state === "checking")
    return <Loading label="Verifying the active testnet deployment" />;
  return (
    <div className="empty deployment-failure">
      <CircleAlert size={27} />
      <h2>Testnet deployment check failed</h2>
      <p>
        Sqim blocked contract actions because the configured generation could
        not be verified. No transaction can be submitted until every address is
        healthy.
      </p>
      <button className="primary-button" onClick={retry}>
        <RefreshCw size={15} /> Retry verification
      </button>
    </div>
  );
}

function Explorer({
  baskets,
  loading,
  refresh,
}: {
  baskets: BasketLive[];
  loading: boolean;
  refresh(): Promise<void>;
}) {
  return (
    <div className="page">
      <PageHead
        eyebrow="Basket explorer"
        title="Diversified exposure, in one asset."
        copy="Browse live baskets deployed by the Sqim factory. Composition and values are read from testnet."
        action={
          <button
            className="icon-button"
            onClick={refresh}
            title="Refresh testnet data"
          >
            <RefreshCw size={18} className={loading ? "spin" : ""} />
          </button>
        }
      />
      <section className="stats-band">
        <Metric label="Live baskets" value={String(baskets.length)} />
        <Metric
          label="Combined AUM"
          value={`${formatUnits(
            baskets.reduce((sum, basket) => sum + basket.aum, 0n),
            2,
          )} XLM`}
        />
        <Metric label="Network" value="Testnet" />
        <Metric label="Settlement" value="Soroswap" />
      </section>
      {loading && baskets.length === 0 ? (
        <Loading label="Reading factory and basket state" />
      ) : (
        <section className="basket-grid">
          {baskets.map((basket) => (
            <BasketCard basket={basket} key={basket.basket} />
          ))}
          {!baskets.length && (
            <Empty
              title="No factory baskets yet"
              copy="Create the first basket from the creator workspace."
            />
          )}
        </section>
      )}
    </div>
  );
}

function BasketCard({ basket }: { basket: BasketLive }) {
  const presentation = basketPresentation(
    basket.basket,
    basket.name,
    basket.assets.length,
  );
  return (
    <Link to={`/basket/${basket.basket}`} className="basket-card">
      <div className="card-top">
        <span className="basket-icon">
          <Layers3 size={22} />
        </span>
        <span className={basket.paused ? "status paused" : "status"}>
          <i />
          {basket.paused
            ? "Paused"
            : presentation.suggested
              ? "Suggested"
              : "Live"}
        </span>
      </div>
      <div>
        <span className="strategy-label">{presentation.strategy}</span>
        <h2>{presentation.name}</h2>
        <p>{presentation.shortDescription}</p>
      </div>
      <WeightBar
        assets={basket.assets.map((item) => item.address)}
        weights={
          basket.actualWeights.length
            ? basket.actualWeights
            : basket.target_weights_bps
        }
      />
      <div className="legend">
        {basket.assets.map((item, index) => (
          <span key={item.address}>
            <i style={{ background: assetMeta(item.address).color }} />
            {assetMeta(item.address).symbol}{" "}
            {bpsLabel(
              basket.actualWeights[index] || basket.target_weights_bps[index],
            )}
          </span>
        ))}
      </div>
      <div className="card-metrics">
        <Metric label="NAV" value={`${formatUnits(basket.nav, 4)} XLM`} />
        <Metric label="AUM" value={`${formatUnits(basket.aum, 2)} XLM`} />
      </div>
      <div className="card-link">
        View basket <ArrowRight size={17} />
      </div>
    </Link>
  );
}

function BasketDetail({
  baskets,
  wallet,
  onConnect,
  refresh,
}: {
  baskets: BasketLive[];
  wallet: WalletState;
  onConnect(): void;
  refresh(): Promise<void>;
}) {
  const { id } = useParams();
  const basket = baskets.find((item) => item.basket === id);
  const [mode, setMode] = useState<"deposit" | "withdraw" | "rebalance">(
    "deposit",
  );
  const [position, setPosition] = useState<Position | null>(null);
  const [amount, setAmount] = useState("1");
  const [tx, setTx] = useState<TxState>(IDLE_TX);
  const [weights, setWeights] = useState<number[]>(
    basket?.target_weights_bps ?? [],
  );
  const [quotes, setQuotes] = useState<SwapQuote[]>([]);
  const [quoteState, setQuoteState] = useState<
    "idle" | "loading" | "ready" | "failed"
  >("idle");
  const [depositPlan, setDepositPlan] = useState<DepositAllocation[]>([]);
  const [depositPlanState, setDepositPlanState] = useState<
    "idle" | "loading" | "ready" | "failed"
  >("idle");
  const [withdrawalPayout, setWithdrawalPayout] = useState<bigint>(0n);
  const [withdrawalQuoteState, setWithdrawalQuoteState] = useState<
    "idle" | "loading" | "ready" | "failed"
  >("idle");
  const [withdrawalQuoteError, setWithdrawalQuoteError] = useState("");
  const [priceHistory, setPriceHistory] = useState<PriceSample[]>([]);
  const [suggestionIgnored, setSuggestionIgnored] = useState(false);
  const presentation = basket
    ? basketPresentation(basket.basket, basket.name, basket.assets.length)
    : null;

  useEffect(() => {
    if (basket) setWeights(basket.target_weights_bps);
  }, [basket]);
  useEffect(() => {
    if (!basket) return;
    setPriceHistory(recordPriceSample(basket.basket, basket.prices));
  }, [basket]);
  useEffect(() => {
    if (!basket || !wallet) {
      setPosition(null);
      return;
    }
    void loadPosition(basket, wallet.address)
      .then(setPosition)
      .catch(() => setPosition(null));
  }, [basket, wallet]);
  useEffect(() => {
    if (!basket || mode !== "deposit") return;
    const units = safeUnits(amount);
    if (units <= 0n) {
      setDepositPlan([]);
      setDepositPlanState("idle");
      return;
    }
    let active = true;
    setDepositPlanState("loading");
    const timer = window.setTimeout(() => {
      void quoteDepositAllocation(
        basket.assets.map((asset) => asset.address),
        basket.target_weights_bps,
        units,
      )
        .then((plan) => {
          if (active) {
            setDepositPlan(plan);
            setDepositPlanState("ready");
          }
        })
        .catch(() => {
          if (active) {
            setDepositPlan([]);
            setDepositPlanState("failed");
          }
        });
    }, 350);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [amount, basket, mode]);
  useEffect(() => {
    if (!basket || mode !== "withdraw") return;
    const units = safeUnits(amount);
    if (units <= 0n) {
      setWithdrawalPayout(0n);
      setWithdrawalQuoteState("idle");
      setWithdrawalQuoteError("");
      return;
    }
    let active = true;
    setWithdrawalQuoteState("loading");
    setWithdrawalQuoteError("");
    const timer = window.setTimeout(() => {
      void quoteWithdrawalPayout(basket, units)
        .then((payout) => {
          if (active) {
            setWithdrawalPayout(payout);
            setWithdrawalQuoteState("ready");
          }
        })
        .catch((error) => {
          if (active) {
            setWithdrawalPayout(0n);
            setWithdrawalQuoteState("failed");
            setWithdrawalQuoteError(friendlyError(error));
          }
        });
    }, 350);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [amount, basket, mode]);

  if (!basket)
    return (
      <div className="page">
        <Loading label="Loading basket" />
      </div>
    );
  const activeBasket = basket;
  const rebalanceAccess = rebalanceAuthorization(
    wallet?.address,
    basket.creator,
    REBALANCERS,
    REBALANCER_THRESHOLD,
  );
  const canRebalance = rebalanceAccess.allowed;
  const sum = sumWeights(weights);
  const violations = weightViolations(
    basket.target_weights_bps,
    weights,
    basket.maxDriftBps,
  );
  const validWeights = sum === 10_000 && violations.length === 0;
  const amountUnits = safeUnits(amount);
  const depositShares =
    basket.nav > 0n ? (amountUnits * 10_000_000n) / basket.nav : 0n;
  const burnUnits = amountUnits;
  const gross =
    withdrawalQuoteState === "ready"
      ? withdrawalPayout
      : (burnUnits * basket.nav) / 10_000_000n;
  const averageCost = position?.averageCostPerShare ?? basket.nav;
  const profit = maxBigInt(0n, gross - (burnUnits * averageCost) / 10_000_000n);
  const fee = (profit * BigInt(basket.feeBps)) / 10_000n;
  const suggestion = suggestMeanReversion(
    basket.target_weights_bps,
    basket.originalTargetWeights,
    basket.actualWeights,
    priceHistory,
    basket.maxDriftBps,
  );

  function changeWeights(next: number[]) {
    setWeights(next);
    setQuotes([]);
    setQuoteState("idle");
  }

  async function run(method: string, args: any[]) {
    if (!wallet) return onConnect();
    setTx({ phase: "wallet" });
    try {
      const hash = await submitContractCall(
        wallet,
        activeBasket.basket,
        method,
        args,
        (submitted) => setTx({ phase: "pending", hash: submitted }),
      );
      setTx({ phase: "success", hash });
      await refresh();
    } catch (error) {
      setTx({ phase: "failed", error: friendlyError(error) });
    }
  }

  async function refreshQuote(next = weights) {
    if (!validWeights) return;
    setQuoteState("loading");
    try {
      setQuotes(await quoteRebalance(activeBasket, next));
      setQuoteState("ready");
    } catch {
      setQuotes([]);
      setQuoteState("failed");
    }
  }

  return (
    <div className="page detail-page">
      <div className="breadcrumb">
        <Link to="/explore">Explorer</Link>
        <span>/</span>
        <span>{presentation!.name}</span>
      </div>
      <section className="detail-hero">
        <div>
          <span className="eyebrow">
            {presentation!.suggested
              ? "Suggested testnet basket"
              : "Live testnet basket"}
          </span>
          <h1>{presentation!.name}</h1>
          <p>{presentation!.shortDescription}</p>
        </div>
        <div className="hero-metrics">
          <Metric label="NAV" value={`${formatUnits(basket.nav, 4)} XLM`} />
          <Metric label="AUM" value={`${formatUnits(basket.aum, 2)} XLM`} />
          <Metric label="Supply" value={formatUnits(basket.supply, 2)} />
        </div>
      </section>
      <section className="composition-panel">
        <div className="section-title">
          <div>
            <span className="eyebrow">Current composition</span>
            <h2>What one SQIMB represents</h2>
          </div>
          <span className="timestamp">
            <i /> Live on-chain state
          </span>
        </div>
        <WeightBar
          assets={basket.assets.map((asset) => asset.address)}
          weights={basket.actualWeights}
          large
        />
        <div className="asset-breakdown">
          {basket.assets.map((asset, index) => (
            <div key={asset.address}>
              <i style={{ background: assetMeta(asset.address).color }} />
              <span>
                <strong>{assetMeta(asset.address).symbol}</strong>
                <small>{assetMeta(asset.address).name}</small>
              </span>
              <b>{bpsLabel(basket.actualWeights[index])}</b>
              <em>{formatUnits(basket.holdings[index], 7)}</em>
            </div>
          ))}
        </div>
      </section>
      <StrategyHistory basket={basket} />
      <HistoricalPerformance basket={basket} />
      <div className="workspace-grid">
        <section className="action-panel">
          <div className="segmented">
            <button
              className={mode === "deposit" ? "active" : ""}
              onClick={() => {
                setMode("deposit");
                setTx(IDLE_TX);
              }}
            >
              <ArrowDownToLine size={16} /> Deposit
            </button>
            <button
              className={mode === "withdraw" ? "active" : ""}
              onClick={() => {
                setMode("withdraw");
                setTx(IDLE_TX);
              }}
            >
              <ArrowUpFromLine size={16} /> Withdraw
            </button>
            <button
              className={mode === "rebalance" ? "active" : ""}
              onClick={() => {
                setMode("rebalance");
                setTx(IDLE_TX);
              }}
            >
              <SlidersHorizontal size={16} /> Rebalance
            </button>
          </div>
          {mode === "deposit" && (
            <ActionForm
              title="Add diversified exposure"
              label="Deposit amount"
              suffix="XLM"
              amount={amount}
              setAmount={setAmount}
              rows={[
                ["Current NAV", `${formatUnits(basket.nav, 4)} XLM`],
                ["SQIMB to receive", formatUnits(depositShares, 4)],
                ["Execution", "One atomic basket deposit"],
              ]}
              disabled={
                basket.paused ||
                amountUnits <= 0n ||
                depositPlanState !== "ready"
              }
              button="Confirm deposit"
              onSubmit={() =>
                run("deposit", [
                  addressArg(wallet?.address || basket.creator),
                  i128Arg(amount),
                ])
              }
              wallet={wallet}
              onConnect={onConnect}
            >
              <AllocationPreview
                plan={depositPlan}
                state={depositPlanState}
                amount={amount}
              />
            </ActionForm>
          )}
          {mode === "withdraw" && (
            <ActionForm
              title="Redeem basket shares"
              label="Shares to burn"
              suffix="SQIMB"
              amount={amount}
              setAmount={setAmount}
              rows={[
                [
                  "Live gross payout",
                  withdrawalQuoteState === "ready"
                    ? `${formatUnits(gross, 4)} XLM`
                    : withdrawalQuoteState === "failed"
                      ? "Unavailable"
                      : "Checking routes",
                ],
                [
                  "Profit-only fee",
                  withdrawalQuoteState === "ready"
                    ? `${formatUnits(fee, 4)} XLM`
                    : "-",
                ],
                [
                  "Net preview",
                  withdrawalQuoteState === "ready"
                    ? `${formatUnits(gross - fee, 4)} XLM`
                    : "-",
                ],
              ]}
              disabled={
                basket.paused ||
                !position ||
                burnUnits <= 0n ||
                burnUnits > position.balance ||
                withdrawalQuoteState !== "ready"
              }
              button="Confirm withdrawal"
              onSubmit={() =>
                run("withdraw", [
                  addressArg(wallet?.address || basket.creator),
                  i128Arg(amount),
                ])
              }
              wallet={wallet}
              onConnect={onConnect}
              note={
                withdrawalQuoteState === "failed"
                  ? withdrawalQuoteError
                  : position
                    ? `Available ${formatUnits(position.balance, 4)} SQIMB`
                    : "Connect to read your transferable share balance."
              }
              noteError={withdrawalQuoteState === "failed"}
            />
          )}
          {mode === "rebalance" && (
            <RebalanceEditor
              basket={basket}
              weights={weights}
              setWeights={changeWeights}
              sum={sum}
              violations={violations}
              quotes={quotes}
              quoteState={quoteState}
              refreshQuote={refreshQuote}
              canRebalance={canRebalance}
              disabledReason={rebalanceAccess.reason}
              wallet={wallet}
              onConnect={onConnect}
              suggestion={suggestionIgnored ? null : suggestion}
              ignoreSuggestion={() => setSuggestionIgnored(true)}
              submit={() =>
                run("rebalance", [
                  addressArg(wallet!.address),
                  u32VecArg(weights),
                  addressVecArg([]),
                ])
              }
            />
          )}
          <TxStatus tx={tx} />
        </section>
        <aside className="position-panel">
          <span className="eyebrow">Your position</span>
          <h2>
            {position
              ? `${formatUnits(position.balance, 4)} SQIMB`
              : "Connect wallet"}
          </h2>
          <div className="position-value">
            <span>Current value</span>
            <strong>
              {position ? `${formatUnits(position.currentValue, 4)} XLM` : "-"}
            </strong>
          </div>
          <div className="position-value">
            <span>Unrealized P&amp;L</span>
            <strong
              className={
                position && position.unrealizedPnl < 0n
                  ? "negative"
                  : "positive"
              }
            >
              {position
                ? `${position.unrealizedPnl >= 0n ? "+" : ""}${formatUnits(position.unrealizedPnl, 4)} XLM`
                : "-"}
            </strong>
          </div>
          <div className="contract-note">
            <ShieldCheck size={18} />
            <span>
              Wallet signs locally. Sqim never receives your private key.
            </span>
          </div>
          <Link className="text-link" to={`/creator/${basket.creator}`}>
            <UserRound size={14} /> Creator profile
          </Link>
          <a
            className="text-link"
            href={`https://stellar.expert/explorer/testnet/contract/${basket.basket}`}
            target="_blank"
            rel="noreferrer"
          >
            View contract <ExternalLink size={14} />
          </a>
        </aside>
      </div>
    </div>
  );
}

function RebalanceEditor({
  basket,
  weights,
  setWeights,
  sum,
  violations,
  quotes,
  quoteState,
  refreshQuote,
  canRebalance,
  disabledReason,
  wallet,
  onConnect,
  submit,
  suggestion,
  ignoreSuggestion,
}: any) {
  return (
    <div className="rebalance-editor">
      <div className="action-heading">
        <span className="eyebrow">Visual rebalance</span>
        <h2>Move the mandate, deliberately.</h2>
        <p>
          Compare the current mandate with the proposed target before any swap
          is signed.
        </p>
      </div>
      {disabledReason && (
        <div className="authorization-note">
          <ShieldCheck size={18} />
          <span>{disabledReason}</span>
        </div>
      )}
      {suggestion && (
        <AlgorithmicSuggestion
          suggestion={suggestion}
          basket={basket}
          canApply={canRebalance}
          apply={() => setWeights(suggestion.weights)}
          ignore={ignoreSuggestion}
        />
      )}
      <div className="before-after">
        <div>
          <small>Current</small>
          <WeightBar
            assets={basket.assets.map((asset: any) => asset.address)}
            weights={basket.target_weights_bps}
          />
        </div>
        <ArrowRight size={20} />
        <div>
          <small>New target</small>
          <WeightBar
            assets={basket.assets.map((asset: any) => asset.address)}
            weights={weights}
          />
        </div>
      </div>
      <div className="slider-list">
        {basket.assets.map((asset: any, index: number) => {
          const violation = violations.find(
            (item: any) => item.index === index,
          );
          const delta = weights[index] - basket.target_weights_bps[index];
          return (
            <label
              className={violation ? "slider-row invalid" : "slider-row"}
              key={asset.address}
            >
              <span>
                <i style={{ background: assetMeta(asset.address).color }} />
                <strong>{assetMeta(asset.address).symbol}</strong>
                <em>
                  {delta === 0
                    ? "No change"
                    : `${delta > 0 ? "+" : ""}${(delta / 100).toFixed(1)}%`}
                </em>
              </span>
              <input
                disabled={!canRebalance}
                type="range"
                min="0"
                max="10000"
                step="50"
                value={weights[index]}
                onChange={(event) => {
                  const next = [...weights];
                  next[index] = Number(event.target.value);
                  setWeights(next);
                }}
              />
              <b>{(weights[index] / 100).toFixed(1)}%</b>
              {violation && (
                <small>
                  Over the {(basket.maxDriftBps / 100).toFixed(0)}% limit by{" "}
                  {((violation.drift - basket.maxDriftBps) / 100).toFixed(1)}%
                </small>
              )}
            </label>
          );
        })}
      </div>
      <div
        className={
          sum === 10000 ? "validation-row valid" : "validation-row invalid"
        }
      >
        <span>
          {sum === 10000 ? <Check size={16} /> : <CircleAlert size={16} />}
          {sum === 10000
            ? "Targets total 100%"
            : `Targets total ${(sum / 100).toFixed(1)}%`}
        </span>
        <span>
          {violations.length === 0 ? (
            <>
              <Check size={16} /> Within per-asset drift
            </>
          ) : (
            <>
              <CircleAlert size={16} /> {violations.length} over limit
            </>
          )}
        </span>
      </div>
      <div className="swap-plan">
        <div className="swap-plan-head">
          <div>
            <span className="eyebrow">Settlement preview</span>
            <h3>Swaps this change triggers</h3>
          </div>
          <button
            className="secondary-button"
            disabled={
              !canRebalance ||
              sum !== 10000 ||
              violations.length > 0 ||
              quoteState === "loading"
            }
            onClick={() => refreshQuote()}
          >
            <RefreshCw
              size={15}
              className={quoteState === "loading" ? "spin" : ""}
            />{" "}
            Live quote
          </button>
        </div>
        {quoteState === "idle" && (
          <p>Validate the weights, then request live Soroswap route outputs.</p>
        )}
        {quoteState === "loading" && (
          <p>Reading the same pools settlement will route through...</p>
        )}
        {quoteState === "failed" && (
          <p className="negative">
            Live route quote failed. No transaction has been submitted.
          </p>
        )}
        {quoteState === "ready" && quotes.length === 0 && (
          <p>No material swaps are required at the current holdings.</p>
        )}
        {quotes.map((quote: SwapQuote, index: number) => (
          <div className="swap-row" key={`${quote.symbol}-${index}`}>
            <span className={quote.side}>{quote.side}</span>
            <strong>
              {formatUnits(quote.amountIn, 5)} {quote.inputSymbol}
            </strong>
            <ArrowRight size={15} />
            <strong>
              {formatUnits(quote.amountOut, 5)} {quote.outputSymbol}
            </strong>
            <small>live pool quote</small>
          </div>
        ))}
      </div>
      <button
        className="primary-button full"
        disabled={
          !canRebalance ||
          sum !== 10000 ||
          violations.length > 0 ||
          quoteState !== "ready"
        }
        onClick={wallet ? submit : onConnect}
      >
        <SlidersHorizontal size={17} /> Confirm rebalance
      </button>
    </div>
  );
}

function StrategyHistory({ basket }: { basket: BasketLive }) {
  const presentation = basketPresentation(
    basket.basket,
    basket.name,
    basket.assets.length,
  );
  const maxObservedDrift = Math.max(
    0,
    ...basket.actualWeights.map((weight, index) =>
      Math.abs(weight - (basket.target_weights_bps[index] ?? 0)),
    ),
  );
  return (
    <section className="strategy-history">
      <div className="strategy-intro">
        <div>
          <span className="eyebrow">
            {presentation.suggested
              ? "Why Sqim suggests it"
              : "Creator mandate"}
          </span>
          <h2>{presentation.strategy}</h2>
          <p>{presentation.rationale}</p>
        </div>
        <span
          className={
            presentation.suggested ? "suggested-badge" : "creator-badge"
          }
        >
          <Sparkles size={14} />
          {presentation.suggested
            ? "Curated testnet basket"
            : "Creator-defined"}
        </span>
      </div>
      <div className="strategy-results">
        <div>
          <span>Current NAV</span>
          <strong>{formatUnits(basket.nav, 4)} XLM</strong>
          <small>Testnet RPC read {timeLabel(basket.asOf)}</small>
        </div>
        <div>
          <span>Assets under management</span>
          <strong>{formatUnits(basket.aum, 2)} XLM</strong>
          <small>Oracle-valued at {timeLabel(basket.asOf)}</small>
        </div>
        <div>
          <span>Largest current drift</span>
          <strong>{bpsLabel(maxObservedDrift)}</strong>
          <small>Against target weights</small>
        </div>
        <div>
          <span>Transferable supply</span>
          <strong>{formatUnits(basket.supply, 2)}</strong>
          <small>SQIMB outstanding</small>
        </div>
      </div>
      <div className="strategy-timeline">
        <div>
          <i />
          <span>
            <strong>Origin</strong>
            <small>{presentation.inception}</small>
          </span>
        </div>
        <div>
          <i />
          <span>
            <strong>Allocation logic</strong>
            <small>{presentation.rationale}</small>
          </span>
        </div>
        <div>
          <i />
          <span>
            <strong>Rebalance guardrail</strong>
            <small>{presentation.rebalancePolicy}</small>
          </span>
        </div>
      </div>
    </section>
  );
}

function AlgorithmicSuggestion({
  suggestion,
  basket,
  canApply,
  apply,
  ignore,
}: {
  suggestion: RebalanceSuggestion;
  basket: BasketLive;
  canApply: boolean;
  apply(): void;
  ignore(): void;
}) {
  return (
    <section className="algorithm-card">
      <div className="algorithm-head">
        <span>
          <Sparkles size={16} /> Algorithmic suggestion
        </span>
        <small>Mechanical output</small>
      </div>
      <h3>Mean-revert toward the original mandate</h3>
      <p>
        This calculation compares current composition with the basket's original
        target and recent local price observations. It is not investment advice
        or a recommendation to buy or sell.
      </p>
      <div className="suggested-weights">
        {basket.assets.map((asset, index) => (
          <div key={asset.address}>
            <span>{assetMeta(asset.address).symbol}</span>
            <strong>{bpsLabel(suggestion.weights[index])}</strong>
            <small>
              {signedBps(
                suggestion.weights[index] - basket.target_weights_bps[index],
              )}
            </small>
          </div>
        ))}
      </div>
      <div className="algorithm-meta">
        <span>
          {suggestion.sampleCount} price sample
          {suggestion.sampleCount === 1 ? "" : "s"}
        </span>
        <span>
          {suggestion.volatilityAdjusted
            ? "Volatility adjustment active"
            : "Building volatility history"}
        </span>
        <span>{suggestion.changedAssets} assets adjusted</span>
        <span>Drift-cap aware</span>
      </div>
      <div className="algorithm-actions">
        <button className="secondary-button" onClick={ignore}>
          Ignore
        </button>
        <button className="primary-button" disabled={!canApply} onClick={apply}>
          <Sparkles size={15} /> {canApply ? "Apply and edit" : "Creator only"}
        </button>
      </div>
    </section>
  );
}

function Portfolio({
  baskets,
  wallet,
  onConnect,
}: {
  baskets: BasketLive[];
  wallet: WalletState;
  onConnect(): void;
}) {
  const [positions, setPositions] = useState<Map<string, Position>>(new Map());
  const [loading, setLoading] = useState(false);
  useEffect(() => {
    if (!wallet) {
      setPositions(new Map());
      return;
    }
    setLoading(true);
    Promise.all(
      baskets.map(
        async (basket) =>
          [basket.basket, await loadPosition(basket, wallet.address)] as const,
      ),
    )
      .then((items) => setPositions(new Map(items)))
      .finally(() => setLoading(false));
  }, [wallet, baskets]);
  const active = baskets.filter(
    (basket) => (positions.get(basket.basket)?.balance ?? 0n) > 0n,
  );
  const total = active.reduce(
    (sum, basket) => sum + (positions.get(basket.basket)?.currentValue ?? 0n),
    0n,
  );
  const pnl = active.reduce(
    (sum, basket) => sum + (positions.get(basket.basket)?.unrealizedPnl ?? 0n),
    0n,
  );
  return (
    <div className="page">
      <PageHead
        eyebrow="Portfolio"
        title="Your diversified positions."
        copy="Basket shares held by the connected wallet, valued at current on-chain NAV."
      />
      {!wallet ? (
        <ConnectEmpty onConnect={onConnect} />
      ) : (
        <>
          <section className="portfolio-summary">
            <div>
              <span>Total basket value</span>
              <strong>{formatUnits(total, 4)} XLM</strong>
            </div>
            <div>
              <span>Unrealized P&amp;L</span>
              <strong className={pnl < 0n ? "negative" : "positive"}>
                {pnl >= 0n ? "+" : ""}
                {formatUnits(pnl, 4)} XLM
              </strong>
            </div>
            <div>
              <span>Positions</span>
              <strong>{active.length}</strong>
            </div>
          </section>
          {loading ? (
            <Loading label="Reading transferable balances" />
          ) : (
            <section className="position-list">
              {active.map((basket, index) => (
                <PositionRow
                  key={basket.basket}
                  basket={basket}
                  position={positions.get(basket.basket)!}
                  index={index}
                />
              ))}
              {!active.length && (
                <Empty
                  title="No basket shares yet"
                  copy="Choose a basket to build your first diversified position."
                />
              )}
            </section>
          )}
        </>
      )}
    </div>
  );
}

function PositionRow({
  basket,
  position,
  index,
}: {
  basket: BasketLive;
  position: Position;
  index: number;
}) {
  const presentation = basketPresentation(
    basket.basket,
    basket.name,
    basket.assets.length,
  );
  return (
    <article
      className="position-row"
      style={{ animationDelay: `${index * 90}ms` }}
    >
      <div className="position-name">
        <span className="basket-icon">
          <Layers3 size={20} />
        </span>
        <div>
          <span className="strategy-label">{presentation.strategy}</span>
          <h2>{presentation.name}</h2>
          <p>{formatUnits(position.balance, 4)} transferable SQIMB</p>
        </div>
      </div>
      <div className="position-composition">
        <WeightBar
          assets={basket.assets.map((asset) => asset.address)}
          weights={basket.actualWeights}
        />
        <div className="mini-legend">
          {basket.assets.map((asset, assetIndex) => (
            <span key={asset.address}>
              <i style={{ background: assetMeta(asset.address).color }} />
              {assetMeta(asset.address).symbol}{" "}
              {bpsLabel(basket.actualWeights[assetIndex])}
            </span>
          ))}
        </div>
      </div>
      <div className="position-metric">
        <span>Current value</span>
        <strong>{formatUnits(position.currentValue, 4)} XLM</strong>
      </div>
      <div className="position-metric">
        <span>Unrealized P&amp;L</span>
        <strong
          className={position.unrealizedPnl < 0n ? "negative" : "positive"}
        >
          {position.unrealizedPnl >= 0n ? "+" : ""}
          {formatUnits(position.unrealizedPnl, 4)} XLM
        </strong>
      </div>
      <Link to={`/basket/${basket.basket}`} className="position-action">
        Manage position <ArrowRight size={15} />
      </Link>
    </article>
  );
}

function Liquidity({
  wallet,
  onConnect,
}: {
  wallet: WalletState;
  onConnect(): void;
}) {
  const [pools, setPools] = useState<PoolLive[]>([]);
  const [loading, setLoading] = useState(true);
  async function refresh() {
    setLoading(true);
    setPools(await loadLiquidity(wallet?.address));
    setLoading(false);
  }
  useEffect(() => {
    let active = true;
    setLoading(true);
    void loadLiquidity(wallet?.address).then((next) => {
      if (active) {
        setPools(next);
        setLoading(false);
      }
    });
    return () => {
      active = false;
    };
  }, [wallet]);
  const totalXlm = pools.reduce((sum, pool) => sum + pool.xlmReserve, 0n);
  const funded = pools.filter((pool) => (pool.lpBalance ?? 0n) > 0n).length;
  return (
    <div className="page">
      <PageHead
        eyebrow="Soroswap liquidity"
        title="Your liquidity, pool by pool."
        copy="Track the real testnet reserves backing Core Four routes and the LP tokens held by your connected wallet."
        action={
          <button
            className="icon-button"
            onClick={refresh}
            title="Refresh pool state"
          >
            <RefreshCw size={18} className={loading ? "spin" : ""} />
          </button>
        }
      />
      <section className="stats-band lp-stats">
        <Metric label="Core Four pools" value={String(pools.length || 3)} />
        <Metric
          label="Combined XLM side"
          value={loading ? "Reading" : `${formatUnits(totalXlm, 2)} XLM`}
        />
        <Metric
          label="Your funded positions"
          value={wallet ? String(funded) : "Connect wallet"}
        />
        <Metric label="Venue" value="Soroswap" />
      </section>
      {loading && pools.length === 0 ? (
        <Loading label="Reading live pool reserves" />
      ) : (
        <section className="pool-grid">
          {pools.map((pool) => (
            <article className="pool-card" key={pool.pair}>
              <div className="pool-card-head">
                <span className="basket-icon">
                  <Droplets size={20} />
                </span>
                <div>
                  <h2>{pool.symbol}</h2>
                  <p>Soroswap testnet pair</p>
                </div>
              </div>
              <div className="pool-reserve">
                <span>{pool.assetSymbol} reserve</span>
                <strong>{formatUnits(pool.assetReserve, 7)}</strong>
              </div>
              <div className="pool-reserve">
                <span>XLM reserve</span>
                <strong>{formatUnits(pool.xlmReserve, 4)}</strong>
              </div>
              <div className="pool-reserve lp-balance">
                <span>Your LP balance</span>
                <strong>
                  {wallet
                    ? formatUnits(pool.lpBalance ?? 0n, 7)
                    : "Connect wallet"}
                </strong>
              </div>
              {!wallet && (
                <button className="secondary-button full" onClick={onConnect}>
                  <Wallet size={15} /> Read my LP tokens
                </button>
              )}
              {pool.error && (
                <p className="negative pool-error">{pool.error}</p>
              )}
              <a
                className="text-link"
                href={`https://stellar.expert/explorer/testnet/contract/${pool.pair}`}
                target="_blank"
                rel="noreferrer"
              >
                View pair contract <ExternalLink size={13} />
              </a>
            </article>
          ))}
        </section>
      )}
    </div>
  );
}

function CreatorProfile({
  baskets,
  wallet,
  onConnect,
}: {
  baskets: BasketLive[];
  wallet: WalletState;
  onConnect(): void;
}) {
  const { address = "" } = useParams();
  const [profile, setProfile] = useState<UserProfile>({
    address,
    display_name: "",
    bio: "",
    avatar_url: "",
    notification_frequency: "off",
    drift_threshold_bps: 500,
    notification_email: "",
  });
  const [creatorBasketIDs, setCreatorBasketIDs] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [editing, setEditing] = useState(false);
  const [status, setStatus] = useState<
    "idle" | "signing" | "saving" | "saved" | "failed"
  >("idle");
  const [error, setError] = useState("");
  const ownProfile = wallet?.address === address;
  const creatorBaskets = baskets.filter((basket) =>
    creatorBasketIDs.includes(basket.basket),
  );

  useEffect(() => {
    let active = true;
    setLoading(true);
    Promise.all([loadProfile(address), loadCreatorBasketAddresses(address)])
      .then(([nextProfile, ids]) => {
        if (active) {
          setProfile(nextProfile);
          setCreatorBasketIDs(ids);
          setError("");
        }
      })
      .catch((reason) => {
        if (active) setError(friendlyError(reason));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [address]);

  async function persist() {
    if (!wallet || !ownProfile) return onConnect();
    setStatus("signing");
    setError("");
    try {
      const challenge = await requestProfileChallenge(address);
      const signature = await signWalletMessage(wallet, challenge.message);
      setStatus("saving");
      const saved = await saveProfile(profile, challenge.nonce, signature);
      setProfile(saved);
      setEditing(false);
      setStatus("saved");
    } catch (reason) {
      setStatus("failed");
      setError(friendlyError(reason));
    }
  }

  if (loading)
    return (
      <div className="page">
        <Loading label="Loading creator profile" />
      </div>
    );
  const initial = (profile.display_name || address).slice(0, 1).toUpperCase();
  return (
    <div className="page creator-profile-page">
      <section className="profile-hero">
        <div className="profile-avatar">
          {profile.avatar_url ? (
            <img src={profile.avatar_url} alt="" />
          ) : (
            initial
          )}
        </div>
        <div>
          <span className="eyebrow">Basket creator</span>
          <h1>{profile.display_name || compact(address, 10)}</h1>
          <p>{profile.bio || "This creator has not added a bio yet."}</p>
          <code>{address}</code>
        </div>
        <div className="profile-actions">
          {ownProfile ? (
            <button
              className="primary-button"
              onClick={() => setEditing((value) => !value)}
            >
              <UserRound size={16} />{" "}
              {editing ? "Close editor" : "Edit profile"}
            </button>
          ) : !wallet ? (
            <button className="secondary-button" onClick={onConnect}>
              <Wallet size={15} /> Connect wallet
            </button>
          ) : null}
        </div>
      </section>
      {error && (
        <div className="global-error">
          <CircleAlert size={16} />
          {error}
        </div>
      )}
      {editing && ownProfile && (
        <section className="profile-editor">
          <div>
            <span className="eyebrow">Public identity</span>
            <h2>Creator profile</h2>
            <p>
              Your wallet signs a one-time challenge before the API accepts
              these changes.
            </p>
          </div>
          <div className="profile-fields">
            <label>
              <span>Display name</span>
              <input
                maxLength={64}
                value={profile.display_name}
                onChange={(event) =>
                  setProfile({ ...profile, display_name: event.target.value })
                }
              />
            </label>
            <label>
              <span>Avatar URL</span>
              <input
                type="url"
                value={profile.avatar_url}
                onChange={(event) =>
                  setProfile({ ...profile, avatar_url: event.target.value })
                }
              />
            </label>
            <label className="wide">
              <span>Bio</span>
              <textarea
                maxLength={320}
                rows={4}
                value={profile.bio}
                onChange={(event) =>
                  setProfile({ ...profile, bio: event.target.value })
                }
              />
            </label>
          </div>
          <div className="notification-settings">
            <div>
              <Bell size={18} />
              <span>
                <strong>Rebalance notifications</strong>
                <small>
                  Email reminders only. No transaction is ever submitted
                  automatically.
                </small>
              </span>
            </div>
            <label>
              <span>Notify me</span>
              <select
                value={profile.notification_frequency}
                onChange={(event) =>
                  setProfile({
                    ...profile,
                    notification_frequency: event.target
                      .value as UserProfile["notification_frequency"],
                  })
                }
              >
                <option value="off">Off</option>
                <option value="weekly">Weekly summary</option>
                <option value="on-drift-only">
                  Only when drift crosses threshold
                </option>
              </select>
            </label>
            <label>
              <span>Drift threshold</span>
              <div className="inline-input">
                <input
                  type="number"
                  min="0"
                  max="100"
                  step="0.5"
                  value={profile.drift_threshold_bps / 100}
                  onChange={(event) =>
                    setProfile({
                      ...profile,
                      drift_threshold_bps: Math.round(
                        Number(event.target.value) * 100,
                      ),
                    })
                  }
                />
                <b>%</b>
              </div>
            </label>
            <label>
              <span>Email</span>
              <input
                type="email"
                value={profile.notification_email}
                onChange={(event) =>
                  setProfile({
                    ...profile,
                    notification_email: event.target.value,
                  })
                }
              />
            </label>
          </div>
          <button
            className="primary-button"
            disabled={status === "signing" || status === "saving"}
            onClick={persist}
          >
            {status === "signing" || status === "saving" ? (
              <LoaderCircle className="spin" size={16} />
            ) : (
              <ShieldCheck size={16} />
            )}
            {status === "signing"
              ? "Confirm in wallet"
              : status === "saving"
                ? "Saving signed profile"
                : "Sign and save profile"}
          </button>
          {status === "saved" && (
            <span className="saved-note">
              <Check size={15} /> Signed profile saved
            </span>
          )}
        </section>
      )}
      <section className="creator-baskets">
        <div className="section-title">
          <div>
            <span className="eyebrow">On-chain work</span>
            <h2>Baskets by this creator</h2>
          </div>
          <strong>{creatorBaskets.length}</strong>
        </div>
        <div className="basket-grid">
          {creatorBaskets.map((basket) => (
            <BasketCard key={basket.basket} basket={basket} />
          ))}
          {!creatorBaskets.length && (
            <Empty
              title="No baskets yet"
              copy="The factory creator mapping does not contain a basket for this address."
            />
          )}
        </div>
      </section>
    </div>
  );
}

function CreateBasket({
  wallet,
  onConnect,
  refresh,
}: {
  wallet: WalletState;
  onConnect(): void;
  refresh(): Promise<void>;
}) {
  const navigate = useNavigate();
  const [step, setStep] = useState(0);
  const [selected, setSelected] = useState<string[]>(
    ASSETS.map((asset) => asset.address),
  );
  const [weights, setWeights] = useState([4000, 2000, 2000, 2000]);
  const [autoBalance, setAutoBalance] = useState(true);
  const [name, setName] = useState("Core Four Basket");
  const [description, setDescription] = useState(
    "A diversified basket spanning four wrapped testnet assets.",
  );
  const [tx, setTx] = useState<TxState>(IDLE_TX);
  const [samplePlan, setSamplePlan] = useState<DepositAllocation[]>([]);
  const [sampleState, setSampleState] = useState<
    "loading" | "ready" | "failed"
  >("loading");
  const sum = sumWeights(weights);
  const selectedAssets = ASSETS.filter((asset) =>
    selected.includes(asset.address),
  );
  const steps = ["Choose assets", "Set weights", "Name basket", "Review"];
  const canContinue =
    step === 0
      ? selected.length >= 2
      : step === 1
        ? sum === 10_000
        : step === 2
          ? name.trim().length >= 3 && description.trim().length >= 12
          : sampleState === "ready";

  useEffect(() => {
    if (sum !== 10_000 || selected.length < 2) {
      setSamplePlan([]);
      setSampleState("failed");
      return;
    }
    let active = true;
    setSampleState("loading");
    void quoteDepositAllocation(selected, weights, 100_000_000n)
      .then((plan) => {
        if (active) {
          setSamplePlan(plan);
          setSampleState("ready");
        }
      })
      .catch(() => {
        if (active) {
          setSamplePlan([]);
          setSampleState("failed");
        }
      });
    return () => {
      active = false;
    };
  }, [selected, sum, weights]);

  function toggleAsset(address: string) {
    const next = selected.includes(address)
      ? selected.filter((item) => item !== address)
      : [...selected, address];
    setSelected(next);
    setWeights(equalWeights(next.length));
  }

  function updateWeight(index: number, value: number) {
    setWeights((current) =>
      autoBalance
        ? setBalancedWeight(current, index, value)
        : current.map((weight, itemIndex) =>
            itemIndex === index
              ? Math.max(0, Math.min(10_000, Math.round(value)))
              : weight,
          ),
    );
  }

  async function create() {
    if (!wallet) return onConnect();
    setTx({ phase: "wallet" });
    try {
      const hash = await submitContractCall(
        wallet,
        FACTORY_ID,
        "create_basket",
        [
          addressArg(wallet.address),
          stringArg(name.trim()),
          assetVecArg(selected),
          u32VecArg(weights),
        ],
        (submitted) => setTx({ phase: "pending", hash: submitted }),
      );
      setTx({ phase: "success", hash });
      await refresh();
      window.setTimeout(() => navigate("/explore"), 1200);
    } catch (error) {
      setTx({ phase: "failed", error: friendlyError(error) });
    }
  }

  return (
    <div className="page create-page">
      <PageHead
        eyebrow="Creator workspace"
        title="Build a basket, one decision at a time."
        copy="Choose the assets, define the mandate, review real settlement routes, then deploy the basket and its transferable token with your wallet."
      />
      <ol className="creation-steps">
        {steps.map((label, index) => (
          <li
            key={label}
            className={index === step ? "active" : index < step ? "done" : ""}
          >
            <span>{index < step ? <Check size={14} /> : index + 1}</span>
            <strong>{label}</strong>
          </li>
        ))}
      </ol>
      <section className="creation-workspace">
        <div className="create-form step-panel">
          {step === 0 && (
            <>
              <div className="action-heading">
                <span className="eyebrow">Step 1</span>
                <h2>Pick constituent assets</h2>
                <p>
                  Select at least two assets. Each selected asset will receive a
                  live target weight in the next step.
                </p>
              </div>
              <div className="asset-picker">
                {ASSETS.map((asset) => {
                  const active = selected.includes(asset.address);
                  return (
                    <button
                      key={asset.address}
                      className={active ? "selected" : ""}
                      onClick={() => toggleAsset(asset.address)}
                    >
                      <i style={{ background: asset.color }} />
                      <span>
                        <strong>{asset.symbol}</strong>
                        <small>{asset.name}</small>
                      </span>
                      {active ? <Check size={18} /> : <Plus size={18} />}
                    </button>
                  );
                })}
              </div>
              <div
                className={
                  selected.length >= 2
                    ? "validation-row valid"
                    : "validation-row invalid"
                }
              >
                <span>
                  {selected.length >= 2 ? (
                    <Check size={16} />
                  ) : (
                    <CircleAlert size={16} />
                  )}
                  {selected.length} assets selected
                </span>
              </div>
            </>
          )}
          {step === 1 && (
            <AllocationEditor
              assets={selectedAssets}
              addresses={selected}
              weights={weights}
              sum={sum}
              autoBalance={autoBalance}
              setAutoBalance={setAutoBalance}
              updateWeight={updateWeight}
              equalize={() => setWeights(equalWeights(selected.length))}
              normalize={() => setWeights(normalizeWeights(weights))}
            />
          )}
          {step === 2 && (
            <>
              <div className="action-heading">
                <span className="eyebrow">Step 3</span>
                <h2>Name the mandate</h2>
                <p>
                  Use plain language that tells holders what kind of diversified
                  exposure the basket represents.
                </p>
              </div>
              <div className="identity-fields">
                <label>
                  <span>Basket name</span>
                  <input
                    placeholder="e.g. Stellar Growth Basket"
                    maxLength={64}
                    value={name}
                    onChange={(event) => setName(event.target.value)}
                  />
                </label>
                <label>
                  <span>Description</span>
                  <textarea
                    placeholder="Describe the allocation logic, intended exposure, and rebalance approach."
                    rows={5}
                    maxLength={320}
                    value={description}
                    onChange={(event) => setDescription(event.target.value)}
                  />
                </label>
              </div>
            </>
          )}
          {step === 3 && (
            <>
              <div className="action-heading">
                <span className="eyebrow">Step 4</span>
                <h2>Review before deployment</h2>
                <p>
                  The factory call deploys both the basket contract and its
                  transferable SQIMB share token.
                </p>
              </div>
              <div className="review-title">
                <div>
                  <strong>{name}</strong>
                  <p>{description}</p>
                </div>
                <span>{selected.length} assets</span>
              </div>
              <WeightBar assets={selected} weights={weights} large />
              <div className="creator-assets">
                {selectedAssets.map((asset, index) => (
                  <div key={asset.address}>
                    <i style={{ background: asset.color }} />
                    <span>
                      <strong>{asset.symbol}</strong>
                      <small>{asset.name}</small>
                    </span>
                    <b>{bpsLabel(weights[index])}</b>
                  </div>
                ))}
              </div>
              <CreatorBacktest assets={selected} weights={weights} />
              <AllocationPreview
                plan={samplePlan}
                state={sampleState}
                amount="10"
              />
              <div className="contract-note">
                <ShieldCheck size={18} />
                <span>
                  Your wallet signs the real factory transaction. Sqim cannot
                  deploy or modify this basket without that signature.
                </span>
              </div>
              <button
                className="primary-button full"
                disabled={!canContinue}
                onClick={wallet ? create : onConnect}
              >
                <Plus size={17} />{" "}
                {wallet
                  ? "Deploy basket and share token"
                  : "Connect wallet to deploy"}
              </button>
              <TxStatus tx={tx} />
            </>
          )}
          <div className="step-actions">
            {step > 0 ? (
              <button
                className="secondary-button"
                onClick={() => setStep(step - 1)}
              >
                <ArrowLeft size={15} /> Back
              </button>
            ) : (
              <span />
            )}
            {step < 3 && (
              <button
                className="primary-button"
                disabled={!canContinue}
                onClick={() => setStep(step + 1)}
              >
                Continue <ArrowRight size={15} />
              </button>
            )}
          </div>
        </div>
        <aside className="creator-review">
          <span className="eyebrow">Live review</span>
          <h2>{name.trim() || "Untitled basket"}</h2>
          <p>{description || "Add a description in step three."}</p>
          <WeightBar assets={selected} weights={weights} large />
          <div className="review-facts">
            <div>
              <span>Constituents</span>
              <strong>{selected.length}</strong>
            </div>
            <div>
              <span>Weight total</span>
              <strong className={sum === 10_000 ? "positive" : "negative"}>
                {bpsLabel(sum)}
              </strong>
            </div>
            <div>
              <span>Share asset</span>
              <strong>Transferable SQIMB</strong>
            </div>
            <div>
              <span>Settlement</span>
              <strong>Soroswap, oracle protected</strong>
            </div>
            <div>
              <span>Creator</span>
              <strong>
                {wallet ? compact(wallet.address, 7) : "Connect wallet"}
              </strong>
            </div>
          </div>
          <p className="share-note">
            Target percentages allocate each future deposit across constituents.
            SQIMB shares are minted to depositors at NAV; creation itself does
            not pre-mint shares.
          </p>
        </aside>
      </section>
    </div>
  );
}

function AllocationEditor({
  assets,
  addresses,
  weights,
  sum,
  autoBalance,
  setAutoBalance,
  updateWeight,
  equalize,
  normalize,
}: {
  assets: (typeof ASSETS)[number][];
  addresses: string[];
  weights: number[];
  sum: number;
  autoBalance: boolean;
  setAutoBalance(value: boolean): void;
  updateWeight(index: number, value: number): void;
  equalize(): void;
  normalize(): void;
}) {
  return (
    <div className="allocation-editor">
      <div className="section-title">
        <div>
          <span className="eyebrow">Step 2</span>
          <h2>Allocate the basket</h2>
          <p>
            Drag a slider or type an exact percentage. Auto balance
            redistributes the remainder so the basket always stays at 100%.
          </p>
        </div>
        <strong className={sum === 10_000 ? "positive" : "negative"}>
          {bpsLabel(sum)}
        </strong>
      </div>
      <div className="allocation-toolbar">
        <div
          className="balance-mode"
          role="group"
          aria-label="Weight balancing mode"
        >
          <button
            className={!autoBalance ? "active" : ""}
            onClick={() => setAutoBalance(false)}
          >
            Manual
          </button>
          <button
            className={autoBalance ? "active" : ""}
            onClick={() => {
              setAutoBalance(true);
              if (sum !== 10_000) normalize();
            }}
          >
            <Scale size={14} /> Auto balance
          </button>
        </div>
        <div className="allocation-commands">
          <button className="secondary-button" onClick={equalize}>
            Equal weights
          </button>
          <button
            className="secondary-button"
            disabled={sum === 10_000}
            onClick={normalize}
          >
            Normalize to 100%
          </button>
        </div>
      </div>
      <WeightBar assets={addresses} weights={weights} large />
      <div className="allocation-rows">
        {assets.map((asset, index) => (
          <div className="allocation-row" key={asset.address}>
            <div className="allocation-asset">
              <i style={{ background: asset.color }} />
              <span>
                <strong>{asset.symbol}</strong>
                <small>{asset.name}</small>
              </span>
            </div>
            <input
              aria-label={`${asset.symbol} allocation slider`}
              type="range"
              min="0"
              max="10000"
              step="1"
              value={weights[index]}
              onChange={(event) =>
                updateWeight(index, Number(event.target.value))
              }
            />
            <label className="percentage-input">
              <input
                aria-label={`${asset.symbol} allocation percentage`}
                type="number"
                min="0"
                max="100"
                step="0.25"
                value={(weights[index] / 100).toFixed(2)}
                onChange={(event) =>
                  updateWeight(index, Number(event.target.value) * 100)
                }
              />
              <b>%</b>
            </label>
          </div>
        ))}
      </div>
      <div
        className={
          sum === 10_000 ? "validation-row valid" : "validation-row invalid"
        }
      >
        <span>
          {sum === 10_000 ? <Check size={16} /> : <CircleAlert size={16} />}
          {sum === 10_000
            ? "Ready: target weights total exactly 100%"
            : `${sum < 10_000 ? "Unallocated" : "Over-allocated"} by ${bpsLabel(Math.abs(10_000 - sum))}`}
        </span>
        <small>
          {autoBalance
            ? "Auto balance is on"
            : "Manual mode allows incomplete totals until you normalize"}
        </small>
      </div>
    </div>
  );
}

function ActionForm({
  title,
  label,
  suffix,
  amount,
  setAmount,
  rows,
  disabled,
  button,
  onSubmit,
  wallet,
  onConnect,
  note,
  noteError,
  children,
}: any) {
  const deposit = suffix === "XLM";
  return (
    <div
      className={`action-form ${deposit ? "deposit-form" : "withdraw-form"}`}
    >
      <div className="action-heading">
        <span className="eyebrow">
          {deposit ? "Diversify in one deposit" : "Redeem basket shares"}
        </span>
        <h2>{title}</h2>
        <p>
          {deposit
            ? "Your deposit is allocated atomically across every constituent."
            : "Preview live routes, profit-only fees, and net payout before signing."}
        </p>
      </div>
      <label className="amount-field">
        <span>{label}</span>
        <div>
          <input
            placeholder="0.00"
            aria-label={`${label} ${suffix}`}
            value={amount}
            onChange={(event) => setAmount(event.target.value)}
            inputMode="decimal"
          />
          <b>{suffix}</b>
        </div>
        <small className={noteError ? "negative" : ""}>{note}</small>
      </label>
      <div className="preview-box">
        {rows.map(([key, value]: string[], index: number) => (
          <div key={key} style={{ animationDelay: `${index * 55}ms` }}>
            <span>{key}</span>
            <strong>{value}</strong>
          </div>
        ))}
      </div>
      {children}
      <button
        className="primary-button full action-submit"
        disabled={disabled}
        onClick={wallet ? onSubmit : onConnect}
      >
        {wallet ? button : "Connect wallet"}
        <ArrowRight size={17} />
      </button>
    </div>
  );
}

function AllocationPreview({
  plan,
  state,
  amount,
  compact = false,
}: {
  plan: DepositAllocation[];
  state: "idle" | "loading" | "ready" | "failed";
  amount: string;
  compact?: boolean;
}) {
  return (
    <div
      className={compact ? "allocation-preview compact" : "allocation-preview"}
    >
      <div className="allocation-head">
        <div>
          <span className="eyebrow">Atomic allocation</span>
          <h3>{amount || "0"} XLM across the basket</h3>
        </div>
        <span className="atomic-badge">
          <ShieldCheck size={13} /> All or revert
        </span>
      </div>
      {state === "loading" && (
        <p>
          <LoaderCircle className="spin" size={15} /> Reading live Soroswap
          outputs...
        </p>
      )}
      {state === "failed" && (
        <p className="negative">
          <CircleAlert size={15} /> Live allocation is unavailable. No
          transaction can be submitted.
        </p>
      )}
      {state === "idle" && (
        <p>Enter an amount to preview its constituent allocation.</p>
      )}
      {state === "ready" && (
        <div className="allocation-list">
          {plan.map((item, index) => (
            <div key={item.asset} style={{ animationDelay: `${index * 65}ms` }}>
              <i style={{ background: assetMeta(item.asset).color }} />
              <span>
                <strong>{item.symbol}</strong>
                <small>
                  {item.route === "retain"
                    ? "Retained in basket"
                    : `${formatUnits(item.inputAmount, 4)} XLM via Soroswap`}
                </small>
              </span>
              <b>
                {formatUnits(item.outputAmount, 7)} {item.symbol}
              </b>
              <em>{bpsLabel(item.weightBps)}</em>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function TxStatus({ tx }: { tx: TxState }) {
  if (tx.phase === "idle") return null;
  const pending = tx.phase === "wallet" || tx.phase === "pending";
  return (
    <div className={`tx-status ${tx.phase}`}>
      {pending ? (
        <LoaderCircle className="spin" size={18} />
      ) : tx.phase === "success" ? (
        <Check size={18} />
      ) : (
        <CircleAlert size={18} />
      )}
      <div>
        <strong>
          {tx.phase === "wallet"
            ? "Confirm in wallet"
            : tx.phase === "pending"
              ? "Transaction pending"
              : tx.phase === "success"
                ? "Confirmed on testnet"
                : "Transaction failed"}
        </strong>
        <span>
          {tx.error ||
            (tx.hash
              ? compact(tx.hash, 10)
              : "Waiting for your signature. No changes have been made yet.")}
        </span>
        {tx.hash && (
          <a
            href={`https://stellar.expert/explorer/testnet/tx/${tx.hash}`}
            target="_blank"
            rel="noreferrer"
          >
            View transaction <ExternalLink size={12} />
          </a>
        )}
      </div>
    </div>
  );
}

function WeightBar({
  assets,
  weights,
  large = false,
}: {
  assets: string[];
  weights: number[];
  large?: boolean;
}) {
  const total =
    weights.reduce((sum, value) => sum + Math.max(0, value), 0) || 1;
  return (
    <div
      className={large ? "weight-bar large" : "weight-bar"}
      aria-label="Basket composition"
    >
      {assets.map((address, index) => (
        <span
          key={address}
          title={`${assetMeta(address).symbol} ${bpsLabel(weights[index] || 0)}`}
          style={{
            width: `${(Math.max(0, weights[index] || 0) * 100) / total}%`,
            background: assetMeta(address).color,
            animationDelay: `${index * 70}ms`,
          }}
        />
      ))}
    </div>
  );
}

function WalletDialog({
  wallet,
  close,
  choose,
  disconnect,
}: {
  wallet: WalletState;
  close(): void;
  choose(kind: WalletKind): Promise<void>;
  disconnect(): void;
}) {
  const [error, setError] = useState("");
  const [busy, setBusy] = useState<WalletKind | null>(null);
  async function connect(kind: WalletKind) {
    setBusy(kind);
    setError("");
    try {
      await choose(kind);
    } catch (reason) {
      setError(friendlyError(reason));
      setBusy(null);
    }
  }
  return (
    <div className="modal-backdrop" onMouseDown={close}>
      <div
        className="wallet-dialog"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <button
          className="modal-close"
          onClick={close}
          aria-label="Close wallet dialog"
        >
          <X size={18} />
        </button>
        <span className="eyebrow">Testnet wallet</span>
        <h2>{wallet ? "Wallet connected" : "Choose a wallet"}</h2>
        <p>
          Every contract action is prepared in the browser and signed by your
          wallet.
        </p>
        {wallet ? (
          <>
            <div className="connected-address">
              <Wallet size={19} />
              <span>
                {wallet.kind}
                <strong>{compact(wallet.address, 12)}</strong>
              </span>
            </div>
            <button className="secondary-button full" onClick={disconnect}>
              Disconnect
            </button>
          </>
        ) : (
          <div className="wallet-options">
            <button onClick={() => connect("freighter")}>
              <span className="wallet-logo">F</span>
              <span>
                <strong>Freighter</strong>
                <small>Stellar's browser wallet</small>
              </span>
              {busy === "freighter" ? (
                <LoaderCircle className="spin" />
              ) : (
                <ArrowRight />
              )}
            </button>
            <button onClick={() => connect("rabet")}>
              <span className="wallet-logo rabet">R</span>
              <span>
                <strong>Rabet</strong>
                <small>Injected Stellar wallet</small>
              </span>
              {busy === "rabet" ? (
                <LoaderCircle className="spin" />
              ) : (
                <ArrowRight />
              )}
            </button>
          </div>
        )}
        {error && (
          <p className="dialog-error">
            <CircleAlert size={15} />
            {error}
          </p>
        )}
      </div>
    </div>
  );
}

function PageHead({
  eyebrow,
  title,
  copy,
  action,
}: {
  eyebrow: string;
  title: string;
  copy: string;
  action?: ReactNode;
}) {
  return (
    <section className="page-head">
      <div>
        <span className="eyebrow">{eyebrow}</span>
        <h1>{title}</h1>
        <p>{copy}</p>
      </div>
      {action}
    </section>
  );
}
function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="metric">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
function Loading({ label }: { label: string }) {
  return (
    <div className="loading">
      <span className="loading-mark">
        <i />
        <i />
        <i />
      </span>
      <div>
        <strong>{label}</strong>
        <span className="loading-lines">
          <i />
          <i />
          <i />
        </span>
      </div>
    </div>
  );
}
function Empty({ title, copy }: { title: string; copy: string }) {
  return (
    <div className="empty">
      <Layers3 size={25} />
      <h2>{title}</h2>
      <p>{copy}</p>
    </div>
  );
}
function ConnectEmpty({ onConnect }: { onConnect(): void }) {
  return (
    <div className="empty connect-empty">
      <Wallet size={27} />
      <h2>Connect your testnet wallet</h2>
      <p>Sqim will read your transferable basket balances and cost basis.</p>
      <button className="primary-button" onClick={onConnect}>
        Connect wallet
      </button>
    </div>
  );
}
function compact(value: string, size = 6) {
  return `${value.slice(0, size)}...${value.slice(-size)}`;
}
function timeLabel(value: string) {
  return `as of ${new Date(value).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`;
}
function bpsLabel(value = 0) {
  return `${(value / 100).toFixed(1)}%`;
}
function signedBps(value = 0) {
  return value === 0
    ? "No change"
    : `${value > 0 ? "+" : ""}${(value / 100).toFixed(1)}%`;
}
function equalWeights(count: number) {
  if (count <= 0) return [];
  const base = Math.floor(10_000 / count);
  return Array.from({ length: count }, (_, index) =>
    index + 1 === count ? 10_000 - base * (count - 1) : base,
  );
}
function safeUnits(value: string) {
  try {
    return parseUnits(value);
  } catch {
    return 0n;
  }
}
function maxBigInt(a: bigint, b: bigint) {
  return a > b ? a : b;
}
