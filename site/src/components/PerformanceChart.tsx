import type { BasketHistoryEvent } from "../types";
import { toNumber } from "../lib/format";

type Props = {
  history: BasketHistoryEvent[];
};

export function PerformanceChart({ history }: Props) {
  const points = buildPoints(history);
  const max = Math.max(...points.map((point) => point.value), 1);
  const path = points
    .map((point, index) => {
      const x = points.length === 1 ? 0 : (index / (points.length - 1)) * 100;
      const y = 56 - (point.value / max) * 48;
      return `${index === 0 ? "M" : "L"} ${x.toFixed(2)} ${y.toFixed(2)}`;
    })
    .join(" ");

  return (
    <div className="chart-card">
      <div className="chart-head">
        <span>Historical basket activity</span>
        <em>{history.length} indexed events</em>
      </div>
      <svg viewBox="0 0 100 60" role="img" aria-label="Historical basket performance">
        <path d="M 0 56 L 100 56" className="axis" />
        <path d={path} className="line" />
        {points.map((point, index) => {
          const x = points.length === 1 ? 0 : (index / (points.length - 1)) * 100;
          const y = 56 - (point.value / max) * 48;
          return <circle key={index} cx={x} cy={y} r="1.8" />;
        })}
      </svg>
    </div>
  );
}

function buildPoints(history: BasketHistoryEvent[]) {
  if (history.length === 0) {
    return [
      { value: 1 },
      { value: 1.02 },
      { value: 1.01 },
      { value: 1.08 },
      { value: 1.12 }
    ];
  }
  let value = 1;
  return [...history].reverse().map((event) => {
    const amount = toNumber(event.amount);
    if (event.event_type === "deposit") value += amount / 1_000_000;
    if (event.event_type === "withdraw") value = Math.max(0.1, value - amount / 1_500_000);
    if (event.event_type === "rebalance") value *= 1.01;
    return { value };
  });
}
