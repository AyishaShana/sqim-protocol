import { useEffect, useMemo, useState } from "react";
import {
  Activity,
  CircleAlert,
  FlaskConical,
  LoaderCircle,
} from "lucide-react";
import { assetMeta } from "../config";
import { loadNAVHistory, runBacktest } from "../lib/api";
import type { BacktestResult, BasketLive, NAVHistory } from "../types";

type WindowKey = "1Y" | "3Y" | "5Y" | "MAX";

export function HistoricalPerformance({ basket }: { basket: BasketLive }) {
  const [windowKey, setWindowKey] = useState<WindowKey>("MAX");
  const [live, setLive] = useState<NAVHistory | null>(null);
  const [simulation, setSimulation] = useState<BacktestResult | null>(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  const symbols = useMemo(
    () => basket.assets.map((asset) => assetMeta(asset.address).symbol),
    [basket.assets],
  );

  useEffect(() => {
    let active = true;
    setLoading(true);
    setError("");
    Promise.all([
      loadNAVHistory(basket.basket),
      runBacktest(
        symbols,
        basket.originalTargetWeights,
        windowStart(windowKey),
      ),
    ])
      .then(([nextLive, nextSimulation]) => {
        if (active) {
          setLive(nextLive);
          setSimulation(nextSimulation);
        }
      })
      .catch((reason) => {
        if (active)
          setError(reason instanceof Error ? reason.message : String(reason));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [basket.basket, basket.originalTargetWeights, symbols, windowKey]);

  const livePoints = (live?.points ?? []).map((point) => ({
    at: point.at,
    value: Number(point.nav),
  }));
  return (
    <section className="history-section">
      <div className="section-title history-title">
        <div>
          <span className="eyebrow">Historical performance</span>
          <h2>Track record and simulation, kept separate.</h2>
          <p>
            On-chain NAV begins at deployment. Earlier results use real market
            prices in an explicitly simulated model.
          </p>
        </div>
        <WindowPicker value={windowKey} setValue={setWindowKey} />
      </div>
      {loading && (
        <div className="history-loading">
          <LoaderCircle className="spin" size={18} /> Loading indexed NAV and
          historical candles...
        </div>
      )}
      {error && (
        <div className="history-error">
          <CircleAlert size={18} />
          <span>
            <strong>Historical analysis unavailable</strong>
            {error}
          </span>
        </div>
      )}
      {!loading && !error && (
        <div className="history-grid">
          <HistoryCard
            tone="live"
            eyebrow="Live track record"
            title="On-chain NAV since deployment"
            badge="Indexed Soroban events"
            points={livePoints}
            valueSuffix=" XLM"
            empty="No indexed NAV events are available yet."
            footer={
              live
                ? `Deployed ${formatDate(live.deployed_at)} · ${live.source}`
                : ""
            }
          />
          {simulation && <SimulationCard result={simulation} />}
        </div>
      )}
    </section>
  );
}

export function CreatorBacktest({
  assets,
  weights,
}: {
  assets: string[];
  weights: number[];
}) {
  const [windowKey, setWindowKey] = useState<WindowKey>("5Y");
  const [result, setResult] = useState<BacktestResult | null>(null);
  const [status, setStatus] = useState<"idle" | "loading" | "ready" | "failed">(
    "idle",
  );
  const [error, setError] = useState("");
  const symbols = assets.map((address) => assetMeta(address).symbol);
  const assetKey = assets.join(",");
  const weightKey = weights.join(",");

  async function run() {
    setStatus("loading");
    setError("");
    try {
      setResult(await runBacktest(symbols, weights, windowStart(windowKey)));
      setStatus("ready");
    } catch (reason) {
      setResult(null);
      setStatus("failed");
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  useEffect(() => {
    setStatus("idle");
    setResult(null);
  }, [assetKey, weightKey, windowKey]);

  return (
    <section className="creator-backtest">
      <div className="backtest-head">
        <div>
          <span className="eyebrow">
            <FlaskConical size={14} /> Pre-deployment research
          </span>
          <h3>Test the proposed allocation</h3>
          <p>
            Uses real historical daily closes. It does not represent a live Sqim
            basket.
          </p>
        </div>
        <WindowPicker value={windowKey} setValue={setWindowKey} />
      </div>
      {result ? (
        <SimulationCard result={result} compact />
      ) : (
        <div className="backtest-empty">
          <Activity size={20} />
          <span>
            <strong>Run a historical simulation</strong>Return, volatility, and
            drawdown use only dates shared by every selected asset.
          </span>
        </div>
      )}
      {status === "failed" && (
        <p className="negative">
          <CircleAlert size={15} /> {error}
        </p>
      )}
      <button
        className="secondary-button"
        disabled={
          status === "loading" ||
          weights.reduce((sum, value) => sum + value, 0) !== 10_000
        }
        onClick={run}
      >
        {status === "loading" ? (
          <LoaderCircle className="spin" size={15} />
        ) : (
          <FlaskConical size={15} />
        )}
        {status === "loading" ? "Running simulation" : "Run backtest"}
      </button>
    </section>
  );
}

function SimulationCard({
  result,
  compact = false,
}: {
  result: BacktestResult;
  compact?: boolean;
}) {
  const youngestHistory = result.asset_history.find(
    (asset) => asset.symbol === result.youngest_constituent,
  );
  const sourceURL = result.asset_history.find(
    (asset) => asset.available,
  )?.source_url;
  const selectedWindowStartsLater =
    youngestHistory?.first &&
    new Date(result.available_from).getTime() >
      new Date(youngestHistory.first).getTime() + 24 * 60 * 60 * 1000;
  return (
    <article className={`history-card simulated ${compact ? "compact" : ""}`}>
      <div className="history-card-head">
        <div>
          <span className="eyebrow">Simulated history</span>
          <h3>Historical target-weight model</h3>
        </div>
        {sourceURL ? (
          <a
            className="history-source"
            href={sourceURL}
            target="_blank"
            rel="noreferrer"
          >
            {result.provider}
          </a>
        ) : (
          <span className="history-source">{result.provider}</span>
        )}
      </div>
      <div className="history-availability">
        <strong>{result.available_years.toFixed(1)} years available</strong>
        <span>
          {selectedWindowStartsLater
            ? `${result.youngest_constituent} is youngest; selected window from ${formatDate(result.available_from)}`
            : `Limited by ${result.youngest_constituent}, from ${formatDate(result.available_from)}`}
        </span>
      </div>
      <LineChart points={result.series} tone="simulated" />
      <div className="backtest-metrics">
        <Metric label="Total return" value={percent(result.total_return)} />
        <Metric
          label="Annualized return"
          value={percent(result.annualized_return)}
        />
        <Metric
          label="Volatility"
          value={percent(result.annualized_volatility)}
        />
        <Metric label="Max drawdown" value={percent(result.max_drawdown)} />
      </div>
      <p className="simulation-disclaimer">
        <FlaskConical size={15} /> {result.disclaimer}
      </p>
      {!compact && (
        <details>
          <summary>Method and constituent coverage</summary>
          <p>{result.methodology}</p>
          <div className="coverage-list">
            {result.asset_history.map((asset) => (
              <span key={asset.symbol}>
                <strong>{asset.symbol}</strong>
                {formatDate(asset.first)} to {formatDate(asset.last)} ·{" "}
                {asset.points.toLocaleString()} candles
              </span>
            ))}
          </div>
        </details>
      )}
    </article>
  );
}

function HistoryCard({
  eyebrow,
  title,
  badge,
  points,
  footer,
  empty,
  valueSuffix,
}: {
  tone: "live";
  eyebrow: string;
  title: string;
  badge: string;
  points: { at: string; value: number }[];
  footer: string;
  empty: string;
  valueSuffix: string;
}) {
  return (
    <article className="history-card live">
      <div className="history-card-head">
        <div>
          <span className="eyebrow">{eyebrow}</span>
          <h3>{title}</h3>
        </div>
        <span className="history-source">{badge}</span>
      </div>
      {points.length ? (
        <>
          <LineChart points={points} tone="live" valueSuffix={valueSuffix} />
          <p className="history-footer">{footer}</p>
        </>
      ) : (
        <div className="history-empty">
          <Activity size={20} />
          {empty}
        </div>
      )}
      <p className="live-boundary">
        This line contains only NAV values emitted after the basket was
        deployed. Simulated values are never appended to it.
      </p>
    </article>
  );
}

function LineChart({
  points,
  tone,
  valueSuffix = "",
}: {
  points: { at: string; value: number }[];
  tone: "live" | "simulated";
  valueSuffix?: string;
}) {
  if (points.length < 2)
    return (
      <div className="history-empty">
        At least two observations are needed to draw a line.
      </div>
    );
  const width = 720;
  const height = 220;
  const padding = 18;
  const values = points.map((point) => point.value);
  const min = Math.min(...values);
  const max = Math.max(...values);
  const span = max - min || 1;
  const coordinates = points
    .map(
      (point, index) =>
        `${padding + (index * (width - padding * 2)) / (points.length - 1)},${height - padding - ((point.value - min) * (height - padding * 2)) / span}`,
    )
    .join(" ");
  return (
    <div className="line-chart">
      <div className="chart-range">
        <span>
          {formatValue(max)}
          {valueSuffix}
        </span>
        <span>
          {formatValue(min)}
          {valueSuffix}
        </span>
      </div>
      <svg
        role="img"
        aria-label={`${tone} performance from ${formatDate(points[0].at)} to ${formatDate(points[points.length - 1].at)}`}
        viewBox={`0 0 ${width} ${height}`}
        preserveAspectRatio="none"
      >
        <line
          x1={padding}
          y1={height - padding}
          x2={width - padding}
          y2={height - padding}
        />
        <polyline className={tone} points={coordinates} />
      </svg>
      <div className="chart-dates">
        <span>{formatDate(points[0].at)}</span>
        <span>{formatDate(points[points.length - 1].at)}</span>
      </div>
    </div>
  );
}

function WindowPicker({
  value,
  setValue,
}: {
  value: WindowKey;
  setValue(value: WindowKey): void;
}) {
  return (
    <div className="window-picker" aria-label="Historical window">
      {(["1Y", "3Y", "5Y", "MAX"] as WindowKey[]).map((item) => (
        <button
          key={item}
          className={item === value ? "active" : ""}
          onClick={() => setValue(item)}
        >
          {item}
        </button>
      ))}
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
function percent(value: number) {
  return `${value >= 0 ? "+" : ""}${(value * 100).toFixed(1)}%`;
}
function formatDate(value?: string) {
  return value
    ? new Date(value).toLocaleDateString(undefined, {
        month: "short",
        year: "numeric",
      })
    : "Unavailable";
}
function formatValue(value: number) {
  return value >= 1000
    ? value.toLocaleString(undefined, { maximumFractionDigits: 0 })
    : value.toFixed(2);
}
function windowStart(windowKey: WindowKey): string | undefined {
  if (windowKey === "MAX") return undefined;
  const years = Number(windowKey.slice(0, -1));
  const date = new Date();
  date.setUTCFullYear(date.getUTCFullYear() - years);
  return date.toISOString().slice(0, 10);
}
