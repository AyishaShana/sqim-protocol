package sqimevent

import (
	"encoding/base64"
	"encoding/binary"
	"encoding/json"
	"testing"
	"time"

	"github.com/stellar/go-stellar-sdk/xdr"
)

func TestNormalizePlainDepositEvent(t *testing.T) {
	raw := RPCEvent{
		ID:             "evt-1",
		EventType:      "contract",
		ContractID:     "CBASKET",
		Ledger:         7,
		LedgerClosedAt: time.Now().UTC().Format(time.RFC3339),
		Topic:          []string{"deposit", "GACCOUNT"},
		Value:          json.RawMessage(`{"basket_id":"CBASKET","amount":"10","shares":"9","nav":"11000000","aum":"99000000"}`),
	}

	event := Normalize(raw)
	if event.Name != "deposit" || event.BasketID != "CBASKET" || event.Amount != "10" || event.Shares != "9" {
		t.Fatalf("unexpected normalized event: %#v", event)
	}
	if event.NAV != "11000000" || event.AUM != "99000000" {
		t.Fatalf("expected loose simulated NAV/AUM fields, got %#v", event)
	}
}

func TestNormalizeBasisEvent(t *testing.T) {
	from := "GDSZDTOZG7PCGIU2T5BLWGIA5IIKTP4427F6UV7QNBZJCECUKRVLYUW5"
	to := "GBCM6STGJDWLXPXBY7FHD6VYQO6ZDHSMPB7STLPM4TLHY4QRGJWZ73IS"
	shares, err := xdr.NewScVal(xdr.ScValTypeScvI128, xdr.Int128Parts{Lo: 25_000_000})
	if err != nil {
		t.Fatal(err)
	}
	cost, err := xdr.NewScVal(xdr.ScValTypeScvI128, xdr.Int128Parts{Lo: 12_000_000})
	if err != nil {
		t.Fatal(err)
	}
	values := xdr.ScVec{shares, cost}
	vec, err := xdr.NewScVal(xdr.ScValTypeScvVec, &values)
	if err != nil {
		t.Fatal(err)
	}
	value, err := xdr.MarshalBase64(vec)
	if err != nil {
		t.Fatal(err)
	}
	event := Normalize(RPCEvent{
		ID:         "basis-1",
		ContractID: "CBASKET",
		Topic:      []string{"basis", from, to},
		Value:      json.RawMessage(`"` + value + `"`),
	})
	if event.Name != "basis" || event.Account != from || event.Counterparty != to {
		t.Fatalf("unexpected basis topics: %#v", event)
	}
	if event.Shares != "25000000" || event.NAV != "12000000" {
		t.Fatalf("unexpected basis value: %#v", event)
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

func TestNormalizeLiveFactoryXDR(t *testing.T) {
	event := Normalize(RPCEvent{
		ID:         "live-factory",
		ContractID: "CBYWTMUFK6DXO4CN4QZASWXAK7BXGJLPWDQNA3CNBOMRCX7GUGTNNKPZ",
		Topic: []string{
			"AAAADwAAAAZiYXNrZXQAAA==",
			"AAAAEgAAAAAAAAAAU+9cpFpvSr082cpk1e0HOQjiLK+oGVwOaguv1K8HDMk=",
		},
		Value: json.RawMessage(`"AAAAEAAAAAEAAAACAAAAEgAAAAGtltdF72BI8c0Mhp4JTN1OWTnqJCXytaaMAAWU2AJ4PwAAABIAAAABtPWxuk9JlOpGcq1Xia1MbeeLKHAD8KHk8PDiePTEaVw="`),
	})

	if event.Name != "basket-created" {
		t.Fatalf("expected basket-created, got %q", event.Name)
	}
	if event.BasketID != "CCWZNV2F55QER4ONBSDJ4CKM3VHFSOPKEQS7FNNGRQAALFGYAJ4D7DWX" {
		t.Fatalf("unexpected basket id: %s", event.BasketID)
	}
	if event.ShareTokenID != "CC2PLMN2J5EZJ2SGOKWVPCNNJRW6PCZIOAB7BIPE6DYOE6HUYRUVYUNP" {
		t.Fatalf("unexpected share token id: %s", event.ShareTokenID)
	}
}
