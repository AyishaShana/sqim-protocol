import * as freighter from "@stellar/freighter-api";
import * as StellarSdk from "@stellar/stellar-sdk/minimal";
import {
  ACTIVE_BASKET_ID,
  ACTIVE_BASKET_TOKEN_ID,
  ASSETS,
  FACTORY_ID,
  LIQUIDITY_POOLS,
  NETWORK_PASSPHRASE,
  ORACLE_ID,
  PUBLIC_ACCOUNT,
  PUBLIC_ACCOUNT_FALLBACK,
  ROUTER_ID,
  RPC_URL,
  SETTLEMENT_ID,
  assetMeta,
} from "../config";
import type {
  BasketLive,
  BasketSpec,
  DepositAllocation,
  PoolLive,
  Position,
  SwapQuote,
  WalletKind,
} from "../types";

const sdk = StellarSdk as any;
const freighterApi = freighter as any;
let simulationAccountPromise: Promise<any> | null = null;

declare global {
  interface Window {
    rabet?: {
      connect(): Promise<{ publicKey?: string; error?: string }>;
      sign(
        xdr: string,
        network: string,
      ): Promise<{ xdr?: string; error?: string }>;
      signMessage?(
        message: string,
        network: string,
      ): Promise<{ signature?: string; error?: string }>;
      disconnect?(): Promise<void>;
    };
  }
}

export async function connectWallet(kind: WalletKind): Promise<string> {
  if (kind === "rabet") {
    if (!window.rabet)
      throw new Error("Rabet is not installed in this browser.");
    const result = await window.rabet.connect();
    if (result.error || !result.publicKey)
      throw new Error(result.error || "Rabet access was rejected.");
    return result.publicKey;
  }
  const connected = await freighterApi.isConnected();
  const available =
    typeof connected === "boolean" ? connected : connected?.isConnected;
  if (!available)
    throw new Error("Freighter is not installed in this browser.");
  const result = await freighterApi.requestAccess();
  if (result?.error)
    throw new Error(result.error.message || "Freighter access was rejected.");
  const address =
    typeof result === "string" ? result : result.address || result.publicKey;
  if (!address) throw new Error("Freighter did not return an account.");
  const network = await freighterApi.getNetworkDetails?.();
  if (
    network?.networkPassphrase &&
    network.networkPassphrase !== NETWORK_PASSPHRASE
  ) {
    throw new Error("Switch Freighter to Stellar Testnet before continuing.");
  }
  return address;
}

export async function submitContractCall(
  wallet: { kind: WalletKind; address: string },
  contractId: string,
  method: string,
  args: any[],
  onSubmitted: (hash: string) => void,
): Promise<string> {
  const server = serverForRpc();
  const account = await server.getAccount(wallet.address);
  const transaction = new sdk.TransactionBuilder(account, {
    fee: sdk.BASE_FEE,
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(new sdk.Contract(contractId).call(method, ...args))
    .setTimeout(90)
    .build();
  const prepared = await server.prepareTransaction(transaction);
  let signedXdr: string;
  if (wallet.kind === "rabet") {
    const signed = await window.rabet!.sign(
      prepared.toXDR(),
      NETWORK_PASSPHRASE,
    );
    if (signed.error || !signed.xdr)
      throw new Error(signed.error || "Rabet did not sign the transaction.");
    signedXdr = signed.xdr;
  } else {
    const signed = await freighterApi.signTransaction(prepared.toXDR(), {
      address: wallet.address,
      networkPassphrase: NETWORK_PASSPHRASE,
    });
    if (signed?.error)
      throw new Error(
        signed.error.message || "Freighter rejected the transaction.",
      );
    signedXdr =
      typeof signed === "string" ? signed : signed.signedTxXdr || signed.xdr;
  }
  const signedTransaction = sdk.TransactionBuilder.fromXDR(
    signedXdr,
    NETWORK_PASSPHRASE,
  );
  const sent = await server.sendTransaction(signedTransaction);
  const hash = sent.hash || signedTransaction.hash().toString("hex");
  if (sent.status === "ERROR" || sent.status === "TRY_AGAIN_LATER") {
    throw new Error(
      `Testnet rejected the transaction (${sent.status}). Hash: ${hash}`,
    );
  }
  onSubmitted(hash);
  for (let attempt = 0; attempt < 40; attempt += 1) {
    await delay(1_000);
    const result = await server.getTransaction(hash);
    if (result.status === sdk.rpc.Api.GetTransactionStatus.SUCCESS) return hash;
    if (result.status === sdk.rpc.Api.GetTransactionStatus.FAILED) {
      throw new Error(
        `Testnet confirmed this transaction as failed. Hash: ${hash}`,
      );
    }
  }
  throw new Error(`Timed out waiting for testnet confirmation. Hash: ${hash}`);
}

export async function loadBaskets(): Promise<BasketLive[]> {
  const count = Number(await simulate(FACTORY_ID, "basket_count", []));
  const specs = await Promise.all(
    Array.from(
      { length: count },
      (_, id) =>
        simulate(FACTORY_ID, "basket", [u32Arg(id)]) as Promise<BasketSpec>,
    ),
  );
  return Promise.all(specs.map(loadBasket));
}

export async function loadCreatorBasketAddresses(
  creator: string,
): Promise<string[]> {
  const result = await simulate(FACTORY_ID, "baskets_by_creator", [
    addressArg(creator),
  ]);
  return Array.isArray(result) ? result.map(String) : [];
}

export async function validateDeployment(): Promise<{
  basketCount: number;
  activeBasket: BasketSpec;
}> {
  let basketCount: number;
  let specs: BasketSpec[];
  try {
    basketCount = Number(await simulate(FACTORY_ID, "basket_count", []));
    specs = await Promise.all(
      Array.from(
        { length: basketCount },
        (_, id) =>
          simulate(FACTORY_ID, "basket", [u32Arg(id)]) as Promise<BasketSpec>,
      ),
    );
  } catch (error) {
    throw new Error(
      `Configured factory ${FACTORY_ID} does not resolve on Stellar testnet: ${rawError(error)}`,
    );
  }

  const activeBasket = assertActiveFactorySpec(
    specs,
    ACTIVE_BASKET_ID,
    ACTIVE_BASKET_TOKEN_ID,
  );
  try {
    const [shareToken, supply, maxSlippageBps, maxAgeSeconds] =
      await Promise.all([
        simulate(ACTIVE_BASKET_ID, "share_token", []),
        simulate(ACTIVE_BASKET_TOKEN_ID, "total_supply", []),
        simulate(SETTLEMENT_ID, "max_slippage_bps", []),
        simulate(ORACLE_ID, "max_age_seconds", []),
      ]);
    if (String(shareToken) !== ACTIVE_BASKET_TOKEN_ID) {
      throw new Error(
        `basket reports share token ${String(shareToken)}, expected ${ACTIVE_BASKET_TOKEN_ID}`,
      );
    }
    if (
      BigInt(supply) < 0n ||
      Number(maxSlippageBps) < 0 ||
      Number(maxAgeSeconds) <= 0
    ) {
      throw new Error("deployment metadata returned invalid values");
    }
  } catch (error) {
    throw new Error(
      `Configured basket generation is inconsistent or unavailable on testnet: ${rawError(error)}`,
    );
  }
  return { basketCount, activeBasket };
}

export function assertActiveFactorySpec(
  specs: BasketSpec[],
  basketId = ACTIVE_BASKET_ID,
  tokenId = ACTIVE_BASKET_TOKEN_ID,
): BasketSpec {
  const active = specs.find((spec) => spec.basket === basketId);
  if (!active) {
    throw new Error(
      `Active basket ${basketId} is not registered by factory ${FACTORY_ID}.`,
    );
  }
  if (active.basket_token !== tokenId) {
    throw new Error(
      `Factory maps active basket ${basketId} to ${active.basket_token}, not configured token ${tokenId}.`,
    );
  }
  return active;
}

export async function loadBasket(spec: BasketSpec): Promise<BasketLive> {
  const addresses = spec.assets.map((asset) => asset.address);
  const [
    nav,
    aum,
    supply,
    feeBps,
    maxDriftBps,
    paused,
    targetWeights,
    holdings,
    prices,
  ] = await Promise.all([
    simulate(spec.basket, "nav", []),
    simulate(spec.basket, "total_basket_value", []),
    simulate(spec.basket_token, "total_supply", []),
    simulate(spec.basket, "withdrawal_fee_bps", []),
    simulate(spec.basket, "max_drift_bps", []),
    simulate(spec.basket, "paused", []),
    simulate(spec.basket, "target_weights_bps", []),
    Promise.all(
      addresses.map((address) =>
        simulate(spec.basket, "holding", [addressArg(address)]),
      ),
    ),
    Promise.all(
      addresses.map(async (address) => {
        const result = await simulate(ORACLE_ID, "price", [
          addressArg(address),
        ]);
        return BigInt(result.price_e7 ?? result.priceE7 ?? result.price);
      }),
    ),
  ]);
  const depositIndex = addresses.indexOf(ASSETS[0].address);
  const depositPrice = BigInt(prices[Math.max(0, depositIndex)]);
  const values = holdings.map(
    (holding, index) =>
      (BigInt(holding) * BigInt(prices[index])) / depositPrice,
  );
  const total = values.reduce((sum, value) => sum + value, 0n);
  const actualWeights = values.map((value) =>
    total > 0n ? Number((value * 10_000n) / total) : 0,
  );
  return {
    ...spec,
    originalTargetWeights: spec.target_weights_bps.map(Number),
    target_weights_bps: targetWeights.map(Number),
    nav: BigInt(nav),
    aum: BigInt(aum),
    supply: BigInt(supply),
    feeBps: Number(feeBps),
    maxDriftBps: Number(maxDriftBps),
    paused: Boolean(paused),
    holdings: holdings.map(BigInt),
    prices: prices.map(BigInt),
    actualWeights,
    asOf: new Date().toISOString(),
  };
}

export async function signWalletMessage(
  wallet: { kind: WalletKind; address: string },
  message: string,
): Promise<string> {
  if (wallet.kind === "rabet") {
    if (!window.rabet?.signMessage)
      throw new Error(
        "Rabet message signing is unavailable. Use Freighter to edit a creator profile.",
      );
    const result = await window.rabet.signMessage(message, NETWORK_PASSPHRASE);
    if (result.error || !result.signature)
      throw new Error(result.error || "Rabet rejected the profile signature.");
    return result.signature;
  }
  const result = await freighterApi.signMessage(message, {
    address: wallet.address,
    networkPassphrase: NETWORK_PASSPHRASE,
  });
  if (result?.error)
    throw new Error(
      result.error.message || "Freighter rejected the profile signature.",
    );
  if (!result?.signedMessage)
    throw new Error("Freighter did not return a profile signature.");
  if (typeof result.signedMessage === "string") return result.signedMessage;
  const bytes =
    result.signedMessage instanceof Uint8Array
      ? result.signedMessage
      : new Uint8Array(result.signedMessage);
  let binary = "";
  bytes.forEach((byte: number) => {
    binary += String.fromCharCode(byte);
  });
  return window.btoa(binary);
}

export async function loadPosition(
  basket: BasketLive,
  account: string,
): Promise<Position> {
  const [balance, rawPosition] = await Promise.all([
    simulate(basket.basket_token, "balance", [addressArg(account)]),
    simulate(basket.basket, "position", [addressArg(account)]),
  ]);
  const trackedShares = BigInt(
    rawPosition.tracked_shares ?? rawPosition.trackedShares ?? 0,
  );
  const averageCost = BigInt(
    rawPosition.average_cost_per_share ??
      rawPosition.averageCostPerShare ??
      basket.nav,
  );
  const tokenBalance = BigInt(balance);
  const currentValue = (tokenBalance * basket.nav) / 10_000_000n;
  const cost = (tokenBalance * averageCost) / 10_000_000n;
  return {
    balance: tokenBalance,
    trackedShares,
    averageCostPerShare: averageCost,
    currentValue,
    unrealizedPnl: currentValue - cost,
  };
}

export async function loadLiquidity(account?: string): Promise<PoolLive[]> {
  return Promise.all(
    LIQUIDITY_POOLS.map(async (pool) => {
      try {
        const [reserves, lpBalance] = await Promise.all([
          simulate(pool.pair, "get_reserves", []),
          account
            ? simulate(pool.pair, "balance", [addressArg(account)])
            : Promise.resolve(undefined),
        ]);
        if (!Array.isArray(reserves) || reserves.length < 2)
          throw new Error("Pool returned invalid reserves.");
        return {
          ...pool,
          assetReserve: BigInt(reserves[0]),
          xlmReserve: BigInt(reserves[1]),
          lpBalance: lpBalance === undefined ? undefined : BigInt(lpBalance),
        };
      } catch (error) {
        return {
          ...pool,
          assetReserve: 0n,
          xlmReserve: 0n,
          error: friendlyError(error),
        };
      }
    }),
  );
}

export async function quoteRebalance(
  basket: BasketLive,
  targetWeights: number[],
): Promise<SwapQuote[]> {
  const depositIndex = basket.assets.findIndex(
    (asset) => asset.address === ASSETS[0].address,
  );
  const depositPrice = basket.prices[depositIndex];
  const targetHoldings = targetWeights.map(
    (weight, index) =>
      (basket.aum * BigInt(weight) * depositPrice) /
      (10_000n * basket.prices[index]),
  );
  const quotes: SwapQuote[] = [];
  let baseAvailable = 0n;
  for (let index = 0; index < basket.assets.length; index += 1) {
    const delta = basket.holdings[index] - targetHoldings[index];
    if (delta <= 0n) continue;
    if (index === depositIndex) {
      baseAvailable += delta;
      continue;
    }
    const output = await routerQuote(delta, [
      basket.assets[index].address,
      ASSETS[0].address,
    ]);
    baseAvailable += output;
    quotes.push({
      side: "sell",
      symbol: assetMeta(basket.assets[index].address).symbol,
      amountIn: delta,
      amountOut: output,
      inputSymbol: assetMeta(basket.assets[index].address).symbol,
      outputSymbol: "XLM",
    });
  }
  const deficits = targetHoldings.map((target, index) =>
    target > basket.holdings[index] ? target - basket.holdings[index] : 0n,
  );
  const deficitValue = deficits.reduce(
    (sum, amount, index) =>
      sum + (amount * basket.prices[index]) / depositPrice,
    0n,
  );
  for (let index = 0; index < basket.assets.length; index += 1) {
    if (deficits[index] <= 0n || index === depositIndex || deficitValue === 0n)
      continue;
    const baseIn =
      (baseAvailable *
        ((deficits[index] * basket.prices[index]) / depositPrice)) /
      deficitValue;
    if (baseIn <= 0n) continue;
    const output = await routerQuote(baseIn, [
      ASSETS[0].address,
      basket.assets[index].address,
    ]);
    quotes.push({
      side: "buy",
      symbol: assetMeta(basket.assets[index].address).symbol,
      amountIn: baseIn,
      amountOut: output,
      inputSymbol: "XLM",
      outputSymbol: assetMeta(basket.assets[index].address).symbol,
    });
  }
  return quotes;
}

export async function quoteDepositAllocation(
  assetAddresses: string[],
  targetWeights: number[],
  amount: bigint,
): Promise<DepositAllocation[]> {
  if (amount <= 0n)
    throw new Error("Deposit amount must be greater than zero.");
  if (assetAddresses.length !== targetWeights.length) {
    throw new Error("Basket weights must total 100% before quoting a deposit.");
  }
  const inputAmounts = allocateDepositInputs(targetWeights, amount);
  const allocations: DepositAllocation[] = [];
  for (let index = 0; index < assetAddresses.length; index += 1) {
    const inputAmount = inputAmounts[index];
    const asset = assetAddresses[index];
    const retain = asset === ASSETS[0].address;
    const outputAmount =
      retain || inputAmount === 0n
        ? inputAmount
        : await routerQuote(inputAmount, [ASSETS[0].address, asset]);
    allocations.push({
      asset,
      symbol: assetMeta(asset).symbol,
      weightBps: targetWeights[index],
      inputAmount,
      outputAmount,
      route: retain ? "retain" : "swap",
    });
  }
  return allocations;
}

export async function quoteWithdrawalPayout(
  basket: BasketLive,
  burnAmount: bigint,
): Promise<bigint> {
  if (burnAmount <= 0n || basket.supply <= 0n)
    throw new Error("Shares to burn must be greater than zero.");
  if (burnAmount > basket.supply)
    throw new Error("Shares to burn exceed the basket supply.");
  const depositIndex = basket.assets.findIndex(
    (asset) => asset.address === ASSETS[0].address,
  );
  if (depositIndex < 0)
    throw new Error("Basket does not contain the configured payout asset.");
  const slippageBps = Number(
    await simulate(SETTLEMENT_ID, "max_slippage_bps", []),
  );
  const depositPrice = basket.prices[depositIndex];
  let payout = 0n;
  for (let index = 0; index < basket.assets.length; index += 1) {
    const constituentAmount =
      (basket.holdings[index] * burnAmount) / basket.supply;
    if (constituentAmount <= 0n) continue;
    if (index === depositIndex) {
      payout += constituentAmount;
      continue;
    }
    const expected = (constituentAmount * basket.prices[index]) / depositPrice;
    const realized = await routerQuote(constituentAmount, [
      basket.assets[index].address,
      ASSETS[0].address,
    ]);
    enforceOracleQuote(
      assetMeta(basket.assets[index].address).symbol,
      constituentAmount,
      expected,
      realized,
      slippageBps,
    );
    payout += realized;
  }
  return payout;
}

export function enforceOracleQuote(
  symbol: string,
  amountIn: bigint,
  expected: bigint,
  realized: bigint,
  slippageBps: number,
): void {
  const minimum = (expected * BigInt(10_000 - slippageBps)) / 10_000n;
  if (realized >= minimum) return;
  if (amountIn < 100n) {
    throw new Error(
      `${symbol} redemption is dust-sized at this share amount. Increase the shares to burn; no transaction was submitted.`,
    );
  }
  throw new Error(
    `The live ${symbol}/XLM quote is below the oracle-protected minimum. Increase the amount or wait for pool prices to realign.`,
  );
}

export function allocateDepositInputs(
  targetWeights: number[],
  amount: bigint,
): bigint[] {
  if (
    targetWeights.length === 0 ||
    targetWeights.some((weight) => !Number.isInteger(weight) || weight < 0) ||
    targetWeights.reduce((sum, weight) => sum + weight, 0) !== 10_000
  ) {
    throw new Error("Basket weights must total 100% before quoting a deposit.");
  }

  let allocated = 0n;
  return targetWeights.map((weight, index) => {
    const input =
      index + 1 === targetWeights.length
        ? amount - allocated
        : (amount * BigInt(weight)) / 10_000n;
    allocated += input;
    return input;
  });
}

async function routerQuote(amount: bigint, path: string[]): Promise<bigint> {
  const amounts = await simulate(ROUTER_ID, "router_get_amounts_out", [
    i128RawArg(amount),
    addressVecArg(path),
  ]);
  return BigInt(amounts[amounts.length - 1]);
}

export async function simulate(
  contractId: string,
  method: string,
  args: any[],
  source = PUBLIC_ACCOUNT,
): Promise<any> {
  const server = serverForRpc();
  const account = await getSimulationAccount(server, source);
  const transaction = new sdk.TransactionBuilder(account, {
    fee: sdk.BASE_FEE,
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(new sdk.Contract(contractId).call(method, ...args))
    .setTimeout(60)
    .build();
  const result: any = await withTimeout(
    server.simulateTransaction(transaction),
    20_000,
    `Testnet read timed out: ${method}`,
  );
  if (sdk.rpc.Api.isSimulationError(result))
    throw new Error(result.error || `Testnet read failed: ${method}`);
  if (!result.result?.retval)
    throw new Error(`No testnet value returned for ${method}.`);
  return sdk.scValToNative(result.result.retval);
}

async function getSimulationAccount(
  server: any,
  requested: string,
): Promise<any> {
  if (!simulationAccountPromise) {
    const candidates = [
      ...new Set([requested, PUBLIC_ACCOUNT, PUBLIC_ACCOUNT_FALLBACK]),
    ];
    simulationAccountPromise = (async () => {
      const attempts = await Promise.allSettled(
        candidates.map((candidate) =>
          withTimeout(
            server.getAccount(candidate),
            15_000,
            `Account lookup timed out: ${candidate}`,
          ),
        ),
      );
      const funded = attempts.find((attempt) => attempt.status === "fulfilled");
      if (funded?.status === "fulfilled") return funded.value;
      if (candidates[0]) return new sdk.Account(candidates[0], "0");
      throw new Error(
        "No valid account is available for read-only simulation.",
      );
    })().catch((error) => {
      simulationAccountPromise = null;
      throw error;
    });
  }
  return simulationAccountPromise;
}

function serverForRpc() {
  return new sdk.rpc.Server(RPC_URL, {
    allowHttp: RPC_URL.startsWith("http://"),
  });
}

export const addressArg = (value: string) => new sdk.Address(value).toScVal();
export const stringArg = (value: string) =>
  sdk.nativeToScVal(value, { type: "string" });
export const u32Arg = (value: number) =>
  sdk.nativeToScVal(value, { type: "u32" });
export const i128RawArg = (value: bigint) =>
  sdk.nativeToScVal(value, { type: "i128" });
export const i128Arg = (value: string) => i128RawArg(parseUnits(value));
export const u32VecArg = (values: number[]) =>
  sdk.xdr.ScVal.scvVec(values.map(u32Arg));
export const addressVecArg = (values: string[]) =>
  sdk.xdr.ScVal.scvVec(values.map(addressArg));
export const assetVecArg = (values: string[]) =>
  sdk.xdr.ScVal.scvVec(
    values.map((value: string) =>
      sdk.xdr.ScVal.scvMap([
        new sdk.xdr.ScMapEntry({
          key: sdk.xdr.ScVal.scvSymbol("address"),
          val: addressArg(value),
        }),
      ]),
    ),
  );

export function parseUnits(value: string): bigint {
  const match = /^(\d+)(?:\.(\d{0,7}))?$/.exec(value.trim());
  if (!match)
    throw new Error(
      "Enter a positive amount with no more than seven decimals.",
    );
  const units =
    BigInt(match[1]) * 10_000_000n +
    BigInt((match[2] || "").padEnd(7, "0") || "0");
  if (units <= 0n) throw new Error("Amount must be greater than zero.");
  return units;
}

export function formatUnits(value: bigint, maximumFractionDigits = 4): string {
  const negative = value < 0n;
  const absolute = negative ? -value : value;
  const whole = absolute / 10_000_000n;
  const fraction = (absolute % 10_000_000n)
    .toString()
    .padStart(7, "0")
    .slice(0, maximumFractionDigits)
    .replace(/0+$/, "");
  return `${negative ? "-" : ""}${whole.toLocaleString()}${fraction ? `.${fraction}` : ""}`;
}

export function friendlyError(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  if (
    /user.{0,24}(reject|declin|den)|request.{0,24}(reject|declin|den)|cancelled|canceled/i.test(
      message,
    )
  ) {
    return "You rejected the transaction in your wallet. No changes were made.";
  }
  if (message.includes("Error(Contract, #507)"))
    return "A Soroswap route is below the oracle-protected minimum. For deposits, reduce the amount; for dust withdrawals, increase the shares to burn.";
  const typedCode = /Error\(Contract, #(\d+)\)/.exec(message)?.[1];
  if (typedCode && CONTRACT_ERROR_MESSAGES[typedCode])
    return CONTRACT_ERROR_MESSAGES[typedCode];
  if (/auth|unauthorized/i.test(message))
    return "This wallet is not authorized for that action.";
  if (/stale|oracle/i.test(message))
    return "No fresh oracle price is available, so the contract rejected the action safely.";
  if (/InvalidAction|UnreachableCodeReached/i.test(message))
    return "The contract rejected these inputs. Check balances, weight totals, and drift limits.";
  if (/insufficient/i.test(message))
    return "The wallet does not have enough testnet balance for this action.";
  return message;
}

const CONTRACT_ERROR_MESSAGES: Record<string, string> = {
  "1004": "This wallet is not authorized for that basket action.",
  "1006":
    "This basket administration change is still inside its testnet timelock.",
  "1007":
    "This basket is paused. Deposits, withdrawals, and rebalances are disabled.",
  "1008":
    "This transaction exceeds the basket's maximum single-transaction size.",
  "1009":
    "The basket amount must be positive and within the supported precision.",
  "1010":
    "Basket weights must contain one value per asset and total exactly 100%.",
  "1013": "The wallet does not hold enough transferable basket shares.",
  "1019":
    "The requested rebalance exceeds the on-chain maximum drift for at least one asset.",
  "1020": "The authorized rebalancer signature quorum was not met.",
  "1021": "A supplied rebalancer signer is not authorized by this basket.",
  "2004": "The wallet does not hold enough transferable basket shares.",
  "2005": "The basket-token allowance is too small for this action.",
  "3005":
    "Basket weights must contain one value per asset and total exactly 100%.",
  "3006": "The configured rebalancer quorum is invalid.",
  "4007": "No valid Soroswap route is configured for this asset pair.",
  "4008":
    "The real Soroswap output breaches the oracle-protected slippage limit, so the trade was rejected.",
  "4009": "The oracle quote is invalid or too small to execute safely.",
  "4011": "Soroswap returned an invalid or inconsistent route result.",
  "5006":
    "No fresh primary or quorum-signed fallback oracle price is available.",
  "5008": "A fallback oracle signer is not authorized.",
  "5009": "The fallback oracle signature quorum was not met.",
};

function rawError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

const delay = (ms: number) =>
  new Promise((resolve) => window.setTimeout(resolve, ms));
function withTimeout<T>(
  promise: Promise<T>,
  milliseconds: number,
  message: string,
): Promise<T> {
  return Promise.race([
    promise,
    new Promise<T>((_, reject) =>
      window.setTimeout(() => reject(new Error(message)), milliseconds),
    ),
  ]);
}
