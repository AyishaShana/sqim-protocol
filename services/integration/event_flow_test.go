//go:build integration

package integration

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/AyishaShana/sqim-protocol/services/internal/httpapi"
	"github.com/AyishaShana/sqim-protocol/services/internal/sqimevent"
	"github.com/AyishaShana/sqim-protocol/services/internal/store"
)

func TestSimulatedContractEventFlowsIndexerToPostgresToAPI(t *testing.T) {
	databaseURL := os.Getenv("SQIM_TEST_DATABASE_URL")
	if databaseURL == "" {
		t.Skip("set SQIM_TEST_DATABASE_URL to run Postgres-backed integration test")
	}

	ctx := context.Background()
	db, err := store.New(ctx, databaseURL)
	if err != nil {
		t.Fatal(err)
	}
	defer db.Close()

	schemaPath := filepath.Join("..", "db", "schema.sql")
	if err := db.ApplySchemaFile(ctx, schemaPath); err != nil {
		t.Fatal(err)
	}

	raw := sqimevent.RPCEvent{
		ID:             "simulated-1",
		EventType:      "contract",
		ContractID:     "CBASKETTESTNET",
		Ledger:         42,
		LedgerClosedAt: time.Now().UTC().Format(time.RFC3339),
		PagingToken:    "cursor-42",
		Topic:          []string{"deposit", "GDEPOSITOR"},
		TxHash:         "tx-simulated",
		Value:          json.RawMessage(`{"basket_id":"CBASKETTESTNET","amount":"10000000","shares":"10000000"}`),
	}
	if err := db.UpsertBasketFromEvent(ctx, sqimevent.ContractEvent{
		ID:         "basket-created-1",
		Name:       "basket-created",
		ContractID: "CFACTORY",
		BasketID:   "CBASKETTESTNET",
		Account:    "GCREATOR",
		Raw:        json.RawMessage(`{"name":"Test Basket"}`),
		OccurredAt: time.Now().UTC(),
	}); err != nil {
		t.Fatal(err)
	}
	if err := db.InsertEvent(ctx, sqimevent.Normalize(raw)); err != nil {
		t.Fatal(err)
	}

	api := httptest.NewServer(httpapi.New(db, nil).Routes())
	defer api.Close()

	resp, err := http.Get(api.URL + "/baskets/CBASKETTESTNET/history")
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200, got %d", resp.StatusCode)
	}

	var events []map[string]any
	if err := json.NewDecoder(resp.Body).Decode(&events); err != nil {
		t.Fatal(err)
	}
	if len(events) == 0 {
		t.Fatal("expected simulated event in API response")
	}
	if events[0]["event_type"] != "deposit" {
		t.Fatalf("expected deposit event, got %#v", events[0]["event_type"])
	}
}
