package soroban

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"time"

	"github.com/AyishaShana/sqim-protocol/services/internal/sqimevent"
)

type Client struct {
	url       string
	http      *http.Client
	requestID int
}

func NewClient(url string) *Client {
	return &Client{
		url:  url,
		http: &http.Client{Timeout: 20 * time.Second},
	}
}

type GetEventsParams struct {
	StartLedger uint32
	Cursor      string
	ContractIDs []string
	Limit       uint32
}

func (c *Client) GetEvents(ctx context.Context, params GetEventsParams) ([]sqimevent.RPCEvent, string, error) {
	if len(params.ContractIDs) == 0 {
		return nil, "", fmt.Errorf("at least one contract ID is required")
	}
	if len(params.ContractIDs) > 25 {
		return nil, "", fmt.Errorf("at most 25 contract IDs can be queried in one cursor stream")
	}
	c.requestID++
	filters := make([]map[string]any, 0, (len(params.ContractIDs)+4)/5)
	for start := 0; start < len(params.ContractIDs); start += 5 {
		end := start + 5
		if end > len(params.ContractIDs) {
			end = len(params.ContractIDs)
		}
		filters = append(filters, map[string]any{
			"type":        "contract",
			"contractIds": params.ContractIDs[start:end],
		})
	}
	payload := map[string]any{
		"jsonrpc": "2.0",
		"id":      c.requestID,
		"method":  "getEvents",
		"params": map[string]any{
			"filters": filters,
			"pagination": map[string]any{
				"limit": params.Limit,
			},
		},
	}
	if params.Cursor != "" {
		payload["params"].(map[string]any)["pagination"].(map[string]any)["cursor"] = params.Cursor
	} else if params.StartLedger > 0 {
		payload["params"].(map[string]any)["startLedger"] = params.StartLedger
	}

	body, err := json.Marshal(payload)
	if err != nil {
		return nil, "", err
	}
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, c.url, bytes.NewReader(body))
	if err != nil {
		return nil, "", err
	}
	req.Header.Set("content-type", "application/json")

	resp, err := c.http.Do(req)
	if err != nil {
		return nil, "", err
	}
	defer resp.Body.Close()
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return nil, "", fmt.Errorf("soroban rpc status %d", resp.StatusCode)
	}

	var out struct {
		Error *struct {
			Code    int    `json:"code"`
			Message string `json:"message"`
		} `json:"error"`
		Result struct {
			Events []sqimevent.RPCEvent `json:"events"`
			Cursor string               `json:"cursor"`
		} `json:"result"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&out); err != nil {
		return nil, "", err
	}
	if out.Error != nil {
		return nil, "", fmt.Errorf("soroban rpc getEvents failed: %s", out.Error.Message)
	}
	if out.Result.Cursor == "" && len(out.Result.Events) > 0 {
		out.Result.Cursor = out.Result.Events[len(out.Result.Events)-1].PagingToken
	}
	return out.Result.Events, out.Result.Cursor, nil
}
