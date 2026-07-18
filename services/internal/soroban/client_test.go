package soroban

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestGetEventsSplitsContractFiltersAtFiveIDs(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		var request struct {
			Params struct {
				Filters []struct {
					ContractIDs []string `json:"contractIds"`
				} `json:"filters"`
			} `json:"params"`
		}
		if err := json.NewDecoder(r.Body).Decode(&request); err != nil {
			t.Fatal(err)
		}
		if len(request.Params.Filters) != 2 {
			t.Fatalf("expected two filters, got %d", len(request.Params.Filters))
		}
		for _, filter := range request.Params.Filters {
			if len(filter.ContractIDs) > 5 {
				t.Fatalf("RPC filter contains %d contract IDs", len(filter.ContractIDs))
			}
		}
		_ = json.NewEncoder(w).Encode(map[string]any{
			"jsonrpc": "2.0",
			"id":      1,
			"result": map[string]any{
				"events": []any{},
				"cursor": "",
			},
		})
	}))
	defer server.Close()

	ids := []string{"C1", "C2", "C3", "C4", "C5", "C6"}
	if _, _, err := NewClient(server.URL).GetEvents(context.Background(), GetEventsParams{
		ContractIDs: ids,
		Limit:       100,
	}); err != nil {
		t.Fatal(err)
	}
}
