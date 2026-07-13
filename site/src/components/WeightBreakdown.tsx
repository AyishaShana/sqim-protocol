import type { BasketAsset } from "../types";
import { formatPercent } from "../lib/format";

type Props = {
  assets: BasketAsset[];
  weights: number[];
};

export function WeightBreakdown({ assets, weights }: Props) {
  return (
    <div className="weight-breakdown">
      {assets.map((asset, index) => {
        const weight = weights[index] ?? 0;
        return (
          <div className="weight-row" key={`${asset.address}-${index}`}>
            <div className="weight-label">
              <span>{asset.symbol ?? `Asset ${index + 1}`}</span>
              <em>{formatPercent(weight)}</em>
            </div>
            <div className="weight-track">
              <span style={{ width: `${Math.max(2, weight / 100)}%` }} />
            </div>
          </div>
        );
      })}
    </div>
  );
}
