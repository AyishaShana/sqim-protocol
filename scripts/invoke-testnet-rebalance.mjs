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

const [contractId, weightsJSON, signersJSON] = process.argv.slice(2);
if (!contractId || !weightsJSON || !signersJSON) {
  throw new Error(
    "usage: node scripts/invoke-testnet-rebalance.mjs CONTRACT WEIGHTS_JSON SIGNERS_JSON",
  );
}
if (NETWORK_PASSPHRASE !== Networks.TESTNET) {
  throw new Error("the bundled multisig smoke helper is testnet-only");
}

const weights = JSON.parse(weightsJSON);
const signerAddresses = signersJSON.startsWith("[")
  ? JSON.parse(signersJSON)
  : signersJSON.split(",").map((address) => address.trim()).filter(Boolean);
const secrets = [
  process.env.SQIM_REBALANCER_A_SECRET,
  process.env.SQIM_REBALANCER_B_SECRET,
];
if (secrets.some((secret) => !secret)) {
  throw new Error("SQIM_REBALANCER_A_SECRET and SQIM_REBALANCER_B_SECRET are required");
}

const signers = secrets.map((secret) => Keypair.fromSecret(secret));
const signerByAddress = new Map(signers.map((signer) => [signer.publicKey(), signer]));
for (const address of signerAddresses) {
  if (!signerByAddress.has(address)) {
    throw new Error(`missing private signer for ${address}`);
  }
}

const source = signers[0];
const server = new rpc.Server(RPC_URL);
const account = await server.getAccount(source.publicKey());
const contract = new Contract(contractId);
const operation = contract.call(
  "rebalance",
  new Address(source.publicKey()).toScVal(),
  nativeToScVal(weights.map((weight) => nativeToScVal(weight, { type: "u32" }))),
  nativeToScVal(signerAddresses.map((address) => new Address(address).toScVal())),
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

const prepared = rpc.assembleTransaction(transaction, simulation).build();
const invokeOperation = prepared.tx.operations()[0].body().invokeHostFunctionOp();
const latestLedger = await server.getLatestLedger();
const signedAuth = await Promise.all(
  invokeOperation.auth().map(async (entry) => {
    if (entry.credentials().switch().name === "sorobanCredentialsSourceAccount") {
      return entry;
    }
    const address = Address.fromScAddress(entry.credentials().address().address()).toString();
    const signer = signerByAddress.get(address);
    if (!signer) throw new Error(`missing signer for authorization entry ${address}`);
    return authorizeEntry(entry, signer, latestLedger.sequence + 100, NETWORK_PASSPHRASE);
  }),
);
invokeOperation.auth(signedAuth);
prepared.sign(source);

const sent = await server.sendTransaction(prepared);
let result = { status: sent.status };
for (let attempt = 0; attempt < 30; attempt += 1) {
  if (sent.status !== "PENDING") break;
  await new Promise((resolve) => setTimeout(resolve, 1_000));
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

console.log(JSON.stringify({ hash: sent.hash, status: result.status }));
if (result.status !== "SUCCESS") process.exitCode = 1;
