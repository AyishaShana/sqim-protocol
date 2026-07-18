import {
  Address,
  BASE_FEE,
  Contract,
  Keypair,
  Networks,
  TransactionBuilder,
  authorizeEntry,
  nativeToScVal,
  rpc,
} from "./node_modules/@stellar/stellar-sdk/lib/index.js";

const RPC_URL = process.env.SOROBAN_RPC_URL;
const NETWORK_PASSPHRASE = process.env.SOROBAN_NETWORK_PASSPHRASE;

if (!RPC_URL || !NETWORK_PASSPHRASE) {
  throw new Error(
    "SOROBAN_RPC_URL and SOROBAN_NETWORK_PASSPHRASE are required",
  );
}
if (NETWORK_PASSPHRASE !== Networks.TESTNET) {
  throw new Error("the bundled fallback helper is testnet-only");
}

const [contractId, asset, priceE7, updatedAt] = process.argv.slice(2);
if (!contractId || !asset || !priceE7 || !updatedAt) {
  throw new Error("usage: node scripts/set-testnet-fallback-price.mjs CONTRACT ASSET PRICE_E7 UPDATED_AT");
}

const secrets = [process.env.SQIM_ORACLE_A_SECRET, process.env.SQIM_ORACLE_B_SECRET];
if (secrets.some((secret) => !secret)) {
  throw new Error("SQIM_ORACLE_A_SECRET and SQIM_ORACLE_B_SECRET are required");
}

const signers = secrets.map((secret) => Keypair.fromSecret(secret));
const signerByAddress = new Map(signers.map((signer) => [signer.publicKey(), signer]));
const server = new rpc.Server(RPC_URL);
const source = signers[0];
const account = await server.getAccount(source.publicKey());
const contract = new Contract(contractId);
const operation = contract.call(
  "set_fallback_price",
  new Address(asset).toScVal(),
  nativeToScVal(BigInt(priceE7), { type: "i128" }),
  nativeToScVal(BigInt(updatedAt), { type: "u64" }),
  nativeToScVal(signers.map((signer) => new Address(signer.publicKey()).toScVal())),
);

const transaction = new TransactionBuilder(account, {
  fee: BASE_FEE,
  networkPassphrase: NETWORK_PASSPHRASE,
})
  .addOperation(operation)
  .setTimeout(60)
  .build();

const simulation = await server.simulateTransaction(transaction);
if (rpc.Api.isSimulationError(simulation)) {
  throw new Error(`simulation failed: ${simulation.error}`);
}

const latestLedger = await server.getLatestLedger();
const prepared = rpc.assembleTransaction(transaction, simulation).build();
const invokeOperation = prepared.operations[0];
if (invokeOperation.type !== "invokeHostFunction") {
  throw new Error("expected invokeHostFunction operation");
}

const signedAuth = await Promise.all(
  invokeOperation.auth.map(async (entry) => {
    if (entry.credentials().switch().name === "sorobanCredentialsSourceAccount") {
      return entry;
    }
    const address = Address.fromScAddress(entry.credentials().address().address()).toString();
    const signer = signerByAddress.get(address);
    if (!signer) throw new Error(`missing signer for authorization entry ${address}`);
    return authorizeEntry(
      entry,
      signer,
      latestLedger.sequence + 100,
      NETWORK_PASSPHRASE,
    );
  }),
);
prepared.tx.operations()[0].body().invokeHostFunctionOp().auth(signedAuth);
const signedTransaction = TransactionBuilder.fromXDR(
  prepared.toEnvelope().toXDR("base64"),
  NETWORK_PASSPHRASE,
);
signedTransaction.sign(source);
const sent = await server.sendTransaction(signedTransaction);
let result = { status: sent.status };
for (let attempt = 0; sent.status === "PENDING" && attempt < 20; attempt += 1) {
  await new Promise((resolve) => setTimeout(resolve, 1000));
  const response = await fetch(RPC_URL, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: attempt + 1,
      method: "getTransaction",
      params: { hash: sent.hash },
    }),
  });
  const body = await response.json();
  if (body.error) throw new Error(`getTransaction failed: ${JSON.stringify(body.error)}`);
  result = body.result;
  if (result.status !== "NOT_FOUND") break;
}

console.log(
  JSON.stringify(
    {
      hash: sent.hash,
      sendStatus: sent.status,
      status: result.status,
      errorResultXdr: sent.errorResult?.toXDR("base64"),
    },
    null,
    2,
  ),
);
if (result.status !== "SUCCESS") process.exitCode = 1;
