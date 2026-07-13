import * as freighter from "@stellar/freighter-api";
import * as StellarSdk from "@stellar/stellar-sdk";
import { NETWORK_PASSPHRASE, RPC_URL } from "../config";

const stellar = StellarSdk as any;
const freighterApi = freighter as any;

export async function connectFreighter(): Promise<{ publicKey: string; network: string }> {
  const connected = await safeCall<boolean>("isConnected", false);
  if (!connected) {
    throw new Error("Freighter is not available. Install Freighter or enable the extension.");
  }

  const access = await freighterApi.requestAccess();
  const publicKey = typeof access === "string" ? access : access.address ?? access.publicKey;
  if (!publicKey) throw new Error("Freighter did not return a public key.");

  let network = "TESTNET";
  try {
    const details = await freighterApi.getNetworkDetails?.();
    network = details?.network ?? details?.networkPassphrase ?? network;
  } catch {
    network = "TESTNET";
  }
  return { publicKey, network };
}

export async function signAndSubmitContractCall(
  publicKey: string,
  contractID: string,
  method: string,
  args: unknown[]
): Promise<string> {
  const server = new stellar.rpc.Server(RPC_URL, { allowHttp: RPC_URL.startsWith("http://") });
  const account = await server.getAccount(publicKey);
  const contract = new stellar.Contract(contractID);
  const operation = contract.call(method, ...args);

  const transaction = new stellar.TransactionBuilder(account, {
    fee: stellar.BASE_FEE,
    networkPassphrase: NETWORK_PASSPHRASE
  })
    .addOperation(operation)
    .setTimeout(60)
    .build();

  const prepared = await server.prepareTransaction(transaction);
  const signed = await freighterApi.signTransaction(prepared.toXDR(), {
    address: publicKey,
    networkPassphrase: NETWORK_PASSPHRASE
  });
  const signedXDR =
    typeof signed === "string" ? signed : signed.signedTxXdr ?? signed.signedXDR ?? signed.xdr;
  if (!signedXDR) throw new Error("Wallet did not return a signed transaction.");

  const signedTx = stellar.TransactionBuilder.fromXDR(signedXDR, NETWORK_PASSPHRASE);
  const result = await server.sendTransaction(signedTx);
  return result.hash ?? result.id ?? signedTx.hash().toString("hex");
}

export function addressArg(address: string): unknown {
  return new stellar.Address(address).toScVal();
}

export function i128Arg(value: number): unknown {
  return stellar.nativeToScVal(Math.round(value), { type: "i128" });
}

export function u32VecArg(values: number[]): unknown {
  return stellar.nativeToScVal(values.map((value) => Math.round(value)), { type: ["vec", "u32"] });
}

export function assetVecArg(addresses: string[]): unknown {
  const assets = addresses.map((address) => ({
    address: new stellar.Address(address).toScVal(),
    symbol: stellar.nativeToScVal("")
  }));
  return stellar.nativeToScVal(assets);
}

export function addressVecArg(addresses: string[]): unknown {
  return stellar.nativeToScVal(addresses.map((address) => new stellar.Address(address).toScVal()));
}

async function safeCall<T>(method: string, fallback: T): Promise<T> {
  try {
    if (typeof freighterApi[method] !== "function") return fallback;
    const result = await freighterApi[method]();
    if (typeof result === "object" && result !== null && "isConnected" in result) {
      return result.isConnected as T;
    }
    return result as T;
  } catch {
    return fallback;
  }
}
