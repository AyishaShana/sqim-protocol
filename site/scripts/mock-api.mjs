import http from "node:http";

const port = Number(process.env.SQIM_MOCK_API_PORT ?? "8081");
const basketId = "CC7XPFDPZEMRRHY3NJ7WPB5RDMWIXZMHNULKQALJGIWTXUXDK7JVPG4A";

const basket = {
  basket_id: basketId,
  creator: "ayisha",
  name: "Sqim Testnet Basket",
  share_token_id: "CD3V4GJ3QJPR6JAWEGJNAEGZ4JRLSGEWAMP2TZIYNO2JXMHTZNBBE3KL",
  assets: [
    { address: "native", symbol: "XLM", name: "Stellar Lumens" },
    { address: "test-usdc", symbol: "USDC", name: "Testnet USDC" },
    { address: "test-btc", symbol: "BTC", name: "Testnet BTC" }
  ],
  weights_bps: [5000, 3000, 2000],
  created_at: new Date(Date.now() - 86_400_000).toISOString()
};

const history = [
  event("deposit", "GDEMOACCOUNT", "100", "100", 72),
  event("rebalance", "GCREATOR", "", "", 95),
  event("deposit", "GDEMOACCOUNT", "50", "47.5", 118),
  event("withdraw", "GDEMOACCOUNT", "18", "18", 142)
];

const server = http.createServer((request, response) => {
  response.setHeader("access-control-allow-origin", "*");
  response.setHeader("access-control-allow-methods", "GET, OPTIONS");
  response.setHeader("access-control-allow-headers", "content-type");
  response.setHeader("content-type", "application/json");

  if (request.method === "OPTIONS") {
    response.writeHead(204);
    response.end();
    return;
  }

  const url = new URL(request.url ?? "/", `http://${request.headers.host}`);
  if (url.pathname === "/health") return send(response, { status: "ok", mode: "mock" });
  if (url.pathname === "/baskets") return send(response, [basket]);
  if (url.pathname === `/baskets/${basketId}`) return send(response, basket);
  if (url.pathname === `/baskets/${basketId}/metrics`) return send(response, { nav: "1.07", aum: "132500" });
  if (url.pathname === `/baskets/${basketId}/history`) return send(response, history);

  response.writeHead(404);
  response.end(JSON.stringify({ error: "not found" }));
});

server.listen(port, "127.0.0.1", () => {
  console.log(`Sqim mock API listening on http://127.0.0.1:${port}`);
});

function send(response, value) {
  response.writeHead(200);
  response.end(JSON.stringify(value));
}

function event(type, account, amount, shares, ledger) {
  return {
    id: ledger,
    event_id: `mock-${type}-${ledger}`,
    basket_id: basketId,
    account,
    event_type: type,
    amount,
    shares,
    tx_hash: `mock-${ledger}`,
    ledger,
    raw: {},
    occurred_at: new Date(Date.now() - (200 - ledger) * 60_000).toISOString()
  };
}
