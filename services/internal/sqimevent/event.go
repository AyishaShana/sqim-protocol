package sqimevent

import (
	"encoding/base64"
	"encoding/binary"
	"encoding/json"
	"strings"
	"time"
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
	ID          string          `json:"id"`
	Name        string          `json:"name"`
	ContractID  string          `json:"contract_id"`
	BasketID    string          `json:"basket_id"`
	Account     string          `json:"account"`
	Amount      string          `json:"amount"`
	Shares      string          `json:"shares"`
	TxHash      string          `json:"tx_hash"`
	Ledger      uint32          `json:"ledger"`
	OccurredAt  time.Time       `json:"occurred_at"`
	PagingToken string          `json:"paging_token"`
	Raw         json.RawMessage `json:"raw"`
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
	if name == "basket" {
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
	decoded, err := base64.StdEncoding.DecodeString(trimmed)
	if err != nil || len(decoded) < 8 {
		return trimmed
	}
	// Soroban RPC topics are XDR-encoded ScVal values. For SCV_SYMBOL the
	// discriminant is 15, followed by a 32-bit length and the symbol bytes.
	if binary.BigEndian.Uint32(decoded[:4]) == 15 {
		n := int(binary.BigEndian.Uint32(decoded[4:8]))
		if n >= 0 && len(decoded) >= 8+n {
			return string(decoded[8 : 8+n])
		}
	}
	return trimmed
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
}

func firstNonEmpty(values ...string) string {
	for _, value := range values {
		if value != "" {
			return value
		}
	}
	return ""
}
