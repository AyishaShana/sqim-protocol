export function compactAddress(address: string): string {
  if (!address) return "";
  if (address.length <= 12) return address;
  return `${address.slice(0, 6)}...${address.slice(-6)}`;
}

export function formatNumber(value: number, options: Intl.NumberFormatOptions = {}): string {
  if (!Number.isFinite(value)) return "0";
  return new Intl.NumberFormat("en-US", {
    maximumFractionDigits: 2,
    ...options
  }).format(value);
}

export function formatToken(value: number): string {
  return formatNumber(value, { maximumFractionDigits: 4 });
}

export function formatPercent(bps: number): string {
  return `${formatNumber(bps / 100, { maximumFractionDigits: 2 })}%`;
}

export function toNumber(value: string | number | undefined | null, fallback = 0): number {
  if (value === undefined || value === null || value === "") return fallback;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}
