import deployment from "../../config/testnet.json";

export const RPC_URL =
  import.meta.env.VITE_SOROBAN_RPC_URL ?? deployment.rpcUrl;
export const API_URL =
  import.meta.env.VITE_SQIM_API_URL ?? "http://localhost:8081";
export const NETWORK_PASSPHRASE = deployment.networkPassphrase;
export const FACTORY_ID =
  import.meta.env.VITE_SQIM_FACTORY_ID ?? deployment.factory;
export const ACTIVE_BASKET_ID =
  import.meta.env.VITE_SQIM_ACTIVE_BASKET_ID ?? deployment.basket;
export const ACTIVE_BASKET_TOKEN_ID =
  import.meta.env.VITE_SQIM_ACTIVE_BASKET_TOKEN_ID ?? deployment.basketToken;
export const SETTLEMENT_ID =
  import.meta.env.VITE_SQIM_SETTLEMENT_ID ?? deployment.settlement;
export const ORACLE_ID =
  import.meta.env.VITE_SQIM_ORACLE_ID ?? deployment.oracleAdapter;
export const ROUTER_ID = deployment.router;
export const PUBLIC_ACCOUNT =
  import.meta.env.VITE_SQIM_SIMULATION_ACCOUNT ??
  deployment.simulationAccounts[0];
export const PUBLIC_ACCOUNT_FALLBACK = deployment.simulationAccounts[1];
export const REBALANCERS = deployment.rebalancers;
export const REBALANCER_THRESHOLD = deployment.rebalancerThreshold;

export type BasketPresentation = {
  name: string;
  shortDescription: string;
  strategy: string;
  rationale: string;
  rebalancePolicy: string;
  inception: string;
  suggested: boolean;
};

const BASKET_PRESENTATIONS: Record<string, BasketPresentation> = {
  [deployment.basket]: {
    name: "Sqim Stellar Core Index",
    shortDescription:
      "A liquidity-aware core allocation across XLM, wrapped BTC, ETH, and SOL.",
    strategy: "Core diversified index",
    rationale:
      "XLM anchors settlement liquidity while three wrapped majors broaden network and market exposure. The 40/20/20/20 mandate keeps the base asset dominant without allowing one wrapped constituent to control the basket.",
    rebalancePolicy:
      "Creator-directed with a 10% per-asset drift bound and oracle-protected Soroswap execution.",
    inception:
      "Deployed as Sqim's primary four-asset testnet basket and funded across three live XLM liquidity pools.",
    suggested: true,
  },
};

export function basketPresentation(
  id: string,
  onChainName: string,
  assetCount: number,
): BasketPresentation {
  return (
    BASKET_PRESENTATIONS[id] ?? {
      name: onChainName,
      shortDescription: `${assetCount}-asset creator basket with transferable SQIMB shares.`,
      strategy: "Creator-defined allocation",
      rationale:
        "The basket's assets and target weights were selected by its creator and enforced by the deployed contract.",
      rebalancePolicy:
        "Creator or authorized quorum, subject to the basket's on-chain drift and settlement guards.",
      inception:
        "Deployed permissionlessly through the Sqim factory on Stellar testnet.",
      suggested: false,
    }
  );
}

export const LIQUIDITY_POOLS = [
  { symbol: "XLM / ETH", assetSymbol: "ETH", pair: deployment.pairs.ETH },
  { symbol: "XLM / BTC", assetSymbol: "BTC", pair: deployment.pairs.BTC },
  { symbol: "XLM / SOL", assetSymbol: "SOL", pair: deployment.pairs.SOL },
] as const;

export const ASSETS = [
  {
    symbol: "XLM",
    name: "Stellar Lumens",
    address: deployment.assets.XLM,
    color: "#39c6a3",
  },
  {
    symbol: "ETH",
    name: "Wrapped Ether",
    address: deployment.assets.ETH,
    color: "#8b8df3",
  },
  {
    symbol: "BTC",
    name: "Wrapped Bitcoin",
    address: deployment.assets.BTC,
    color: "#f4b44b",
  },
  {
    symbol: "SOL",
    name: "Wrapped Solana",
    address: deployment.assets.SOL,
    color: "#e8738a",
  },
] as const;

export const assetMeta = (address: string) =>
  ASSETS.find((asset) => asset.address === address) ?? {
    symbol: `${address.slice(0, 4)}...`,
    name: "Soroban asset",
    address,
    color: "#8b9691",
  };
