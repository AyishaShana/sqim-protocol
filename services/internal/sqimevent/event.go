package sqimevent

import (
	"encoding/base64"
	"encoding/binary"
	"encoding/json"
	"strings"
	"time"

	"github.com/stellar/go-stellar-sdk/xdr"
)

type RPCEvent struct {
	ID             string          `json:"id"`
	EventType      string          `json:"type"`
	ContractID     string          `json:"contractId"`
	Ledger         uint32          `json:"ledger"`
	LedgerClosedAt string          `json:"ledgerClosedAt"`
	PagingToken    string          `json:"pagingToken"`
	Topic          []string        `json:"topic"`
	Value          json.RawMessage `json:"value"`
	InSuccessfulTx bool            `json:"inSuccessfulContractCall"`
	TxHash         string          `json:"txHash"`
}

type ContractEvent struct {
	ID           string          `json:"id"`
	Name         string          `json:"name"`
	ContractID   string          `json:"contract_id"`
	BasketID     string          `json:"basket_id"`
	Account      string          `json:"account"`
	Counterparty string          `json:"counterparty"`
	Amount       string          `json:"amount"`
	Shares       string          `json:"shares"`
	Fee          string          `json:"fee"`
	NAV          string          `json:"nav"`
	AUM          string          `json:"aum"`
	ShareTokenID string          `json:"share_token_id"`
	BasketName   string          `json:"basket_name"`
	Assets       json.RawMessage `json:"assets"`
	WeightsBPS   json.RawMessage `json:"weights_bps"`
	TxHash       string          `json:"tx_hash"`
	Ledger       uint32          `json:"ledger"`
	OccurredAt   time.Time       `json:"occurred_at"`
	PagingToken  string          `json:"paging_token"`
	Raw          json.RawMessage `json:"raw"`
}

func Normalize(e RPCEvent) ContractEvent {
	topics := make([]string, 0, len(e.Topic))
	for _, topic := range e.Topic {
		topics = append(topics, decodeTopic(topic))
	}

	name := ""
	if len(topics) > 0 {
		name = topics[0]
	}
	if name == "basket" || name == "basket_created" {
		name = "basket-created"
	}

	occurredAt := time.Now().UTC()
	if parsed, err := time.Parse(time.RFC3339, e.LedgerClosedAt); err == nil {
		occurredAt = parsed
	}

	raw, _ := json.Marshal(e)
	out := ContractEvent{
		ID:          firstNonEmpty(e.ID, e.PagingToken),
		Name:        name,
		ContractID:  e.ContractID,
		BasketID:    e.ContractID,
		TxHash:      e.TxHash,
		Ledger:      e.Ledger,
		OccurredAt:  occurredAt,
		PagingToken: e.PagingToken,
		Raw:         raw,
	}
	if len(topics) > 1 {
		out.Account = topics[1]
	}
	if len(topics) > 2 && (name == "transfer" || name == "basis") {
		out.Counterparty = topics[2]
	}
	readXDRValue(e.Value, &out)
	readLooseValue(e.Value, &out)
	return out
}

func decodeTopic(topic string) string {
	trimmed := strings.Trim(topic, "\" ")
	if trimmed == "" {
		return ""
	}
	if !strings.ContainsAny(trimmed, "+/=") && !strings.HasPrefix(trimmed, "AAAA") {
		return trimmed
	}
	var value xdr.ScVal
	if err := xdr.SafeUnmarshalBase64(trimmed, &value); err != nil {
		decoded, decodeErr := base64.StdEncoding.DecodeString(trimmed)
		if decodeErr == nil && len(decoded) >= 8 && binary.BigEndian.Uint32(decoded[:4]) == 15 {
			size := int(binary.BigEndian.Uint32(decoded[4:8]))
			if size >= 0 && len(decoded) >= 8+size {
				return string(decoded[8 : 8+size])
			}
		}
		return trimmed
	}
	return value.String()
}

func readXDRValue(raw json.RawMessage, out *ContractEvent) {
	var encoded string
	if len(raw) == 0 || json.Unmarshal(raw, &encoded) != nil || encoded == "" {
		return
	}
	var value xdr.ScVal
	if xdr.SafeUnmarshalBase64(encoded, &value) != nil {
		return
	}
	values, ok := value.GetVec()
	if !ok || values == nil {
		if out.Name == "transfer" {
			out.Shares = value.String()
			out.Amount = value.String()
		}
		return
	}

	switch out.Name {
	case "basket-created":
		if len(*values) >= 2 {
			out.BasketID = (*values)[0].String()
			out.ShareTokenID = (*values)[1].String()
		}
		if len(*values) >= 5 {
			out.BasketName = (*values)[2].String()
			out.Assets = assetJSON((*values)[3])
			out.WeightsBPS = uintJSON((*values)[4])
		}
	case "deposit":
		out.Amount = vecString(values, 0)
		out.Shares = vecString(values, 1)
		out.NAV = vecString(values, 2)
		out.AUM = vecString(values, 3)
	case "withdraw":
		out.Shares = vecString(values, 0)
		out.Amount = vecString(values, 1)
		out.Fee = vecString(values, 2)
		out.NAV = vecString(values, 3)
		out.AUM = vecString(values, 4)
	case "rebalance":
		if len(*values) >= 3 {
			out.WeightsBPS = uintJSON((*values)[0])
			out.NAV = (*values)[1].String()
			out.AUM = (*values)[2].String()
		}
	case "basis":
		out.Shares = vecString(values, 0)
		out.NAV = vecString(values, 1)
	}
}

func vecString(values *xdr.ScVec, index int) string {
	if values == nil || index < 0 || index >= len(*values) {
		return ""
	}
	return (*values)[index].String()
}

func assetJSON(value xdr.ScVal) json.RawMessage {
	values, ok := value.GetVec()
	if !ok || values == nil {
		return json.RawMessage(`[]`)
	}
	assets := make([]map[string]string, 0, len(*values))
	for _, item := range *values {
		address := ""
		if item.Type == xdr.ScValTypeScvAddress {
			address = item.String()
		} else if entries, ok := item.GetMap(); ok && entries != nil {
			for _, entry := range *entries {
				if entry.Key.String() == "address" {
					address = entry.Val.String()
					break
				}
			}
		}
		if address != "" {
			assets = append(assets, map[string]string{"address": address})
		}
	}
	encoded, _ := json.Marshal(assets)
	return encoded
}

func uintJSON(value xdr.ScVal) json.RawMessage {
	values, ok := value.GetVec()
	if !ok || values == nil {
		return json.RawMessage(`[]`)
	}
	weights := make([]uint32, 0, len(*values))
	for _, item := range *values {
		if number, ok := item.GetU32(); ok {
			weights = append(weights, uint32(number))
		}
	}
	encoded, _ := json.Marshal(weights)
	return encoded
}

func readLooseValue(raw json.RawMessage, out *ContractEvent) {
	if len(raw) == 0 {
		return
	}
	var m map[string]any
	if err := json.Unmarshal(raw, &m); err != nil {
		return
	}
	if v, ok := m["basket_id"].(string); ok && v != "" {
		out.BasketID = v
	}
	if v, ok := m["basket"].(string); ok && v != "" {
		out.BasketID = v
	}
	if v, ok := m["account"].(string); ok && v != "" {
		out.Account = v
	}
	if v, ok := m["amount"].(string); ok {
		out.Amount = v
	}
	if v, ok := m["shares"].(string); ok {
		out.Shares = v
	}
	if v, ok := m["fee"].(string); ok {
		out.Fee = v
	}
	if v, ok := m["nav"].(string); ok {
		out.NAV = v
	}
	if v, ok := m["aum"].(string); ok {
		out.AUM = v
	}
}

func firstNonEmpty(values ...string) string {
	for _, value := range values {
		if value != "" {
			return value
		}
	}
	return ""
}
