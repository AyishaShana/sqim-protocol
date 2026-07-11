package sqimevent

import (
	"encoding/base64"
	"encoding/binary"
	"encoding/json"
	"testing"
	"time"
)

func TestNormalizePlainDepositEvent(t *testing.T) {
	raw := RPCEvent{
		ID:             "evt-1",
		EventType:      "contract",
		ContractID:     "CBASKET",
		Ledger:         7,
		LedgerClosedAt: time.Now().UTC().Format(time.RFC3339),
		Topic:          []string{"deposit", "GACCOUNT"},
		Value:          json.RawMessage(`{"basket_id":"CBASKET","amount":"10","shares":"9"}`),
	}

	event := Normalize(raw)
	if event.Name != "deposit" || event.BasketID != "CBASKET" || event.Amount != "10" || event.Shares != "9" {
		t.Fatalf("unexpected normalized event: %#v", event)
	}
}

func TestNormalizeSymbolTopic(t *testing.T) {
	topic := make([]byte, 8+len("rebalance"))
	binary.BigEndian.PutUint32(topic[:4], 15)
	binary.BigEndian.PutUint32(topic[4:8], uint32(len("rebalance")))
	copy(topic[8:], "rebalance")

	event := Normalize(RPCEvent{
		ID:         "evt-2",
		ContractID: "CBASKET",
		Topic:      []string{base64.StdEncoding.EncodeToString(topic)},
	})
	if event.Name != "rebalance" {
		t.Fatalf("expected rebalance topic, got %q", event.Name)
	}
}
