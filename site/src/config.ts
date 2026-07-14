export const API_URL = import.meta.env.VITE_SQIM_API_URL ?? "http://localhost:8081";
export const RPC_URL = import.meta.env.VITE_SOROBAN_RPC_URL ?? "https://soroban-testnet.stellar.org";
export const NETWORK_PASSPHRASE =
  import.meta.env.VITE_SOROBAN_NETWORK_PASSPHRASE ?? "Test SDF Network ; September 2015";

export const FACTORY_CONTRACT_ID =
  import.meta.env.VITE_SQIM_FACTORY_CONTRACT_ID ??
  "CBYWTMUFK6DXO4CN4QZASWXAK7BXGJLPWDQNA3CNBOMRCX7GUGTNNKPZ";

export const DEFAULT_BASKET_CONTRACT_ID =
  import.meta.env.VITE_SQIM_DEFAULT_BASKET_ID ??
  "CABCGGFYGPWYNRPJIXFN6YHGER7YHY4CH4GWHQZUHAFEO7A6EJNS64VZ";

export const WITHDRAWAL_FEE_BPS = Number(import.meta.env.VITE_WITHDRAWAL_FEE_BPS ?? "100");
export const ENABLE_CONTRACT_WRITES = import.meta.env.VITE_SQIM_ENABLE_CONTRACT_WRITES !== "false";
export const INVESTABLE_BASKET_IDS = (
  import.meta.env.VITE_SQIM_INVESTABLE_BASKET_IDS ?? DEFAULT_BASKET_CONTRACT_ID
)
  .split(",")
  .map((value: string) => value.trim())
  .filter(Boolean);
