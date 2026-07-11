package store

import (
	"context"
	"encoding/json"
	"os"
	"time"

	"github.com/AyishaShana/sqim-protocol/services/internal/sqimevent"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

type Store struct {
	pool *pgxpool.Pool
}

type BasketConfig struct {
	BasketID     string          `json:"basket_id"`
	Creator      string          `json:"creator"`
	Name         string          `json:"name"`
	ShareTokenID string          `json:"share_token_id"`
	Assets       json.RawMessage `json:"assets"`
	WeightsBPS   json.RawMessage `json:"weights_bps"`
	Strategy     json.RawMessage `json:"strategy"`
	CreatedAt    time.Time       `json:"created_at"`
}

type HistoryEvent struct {
	ID         int64           `json:"id"`
	EventID    string          `json:"event_id"`
	BasketID   string          `json:"basket_id"`
	Account    string          `json:"account"`
	EventType  string          `json:"event_type"`
	Amount     string          `json:"amount"`
	Shares     string          `json:"shares"`
	TxHash     string          `json:"tx_hash"`
	Ledger     int64           `json:"ledger"`
	Raw        json.RawMessage `json:"raw"`
	OccurredAt time.Time       `json:"occurred_at"`
}

func New(ctx context.Context, databaseURL string) (*Store, error) {
	pool, err := pgxpool.New(ctx, databaseURL)
	if err != nil {
		return nil, err
	}
	if err := pool.Ping(ctx); err != nil {
		pool.Close()
		return nil, err
	}
	return &Store{pool: pool}, nil
}

func (s *Store) Close() {
	s.pool.Close()
}

func (s *Store) ApplySchemaFile(ctx context.Context, path string) error {
	schema, err := os.ReadFile(path)
	if err != nil {
		return err
	}
	_, err = s.pool.Exec(ctx, string(schema))
	return err
}

func (s *Store) Cursor(ctx context.Context, key string) (string, error) {
	var cursor string
	err := s.pool.QueryRow(ctx, `select cursor from indexer_state where key = $1`, key).Scan(&cursor)
	if err == pgx.ErrNoRows {
		return "", nil
	}
	return cursor, err
}

func (s *Store) SaveCursor(ctx context.Context, key, cursor string, ledger uint32) error {
	_, err := s.pool.Exec(ctx, `
		insert into indexer_state (key, cursor, last_ledger, updated_at)
		values ($1, $2, $3, now())
		on conflict (key) do update
		set cursor = excluded.cursor, last_ledger = excluded.last_ledger, updated_at = now()
	`, key, cursor, ledger)
	return err
}

func (s *Store) InsertEvent(ctx context.Context, event sqimevent.ContractEvent) error {
	if event.ID == "" {
		event.ID = event.TxHash
	}
	if event.BasketID == "" {
		event.BasketID = event.ContractID
	}
	if event.OccurredAt.IsZero() {
		event.OccurredAt = time.Now().UTC()
	}

	if event.Name == "basket-created" {
		if err := s.UpsertBasketFromEvent(ctx, event); err != nil {
			return err
		}
	}

	_, err := s.pool.Exec(ctx, `
		insert into deposit_withdraw_events
			(event_id, basket_id, account, event_type, amount, shares, tx_hash, ledger, raw, occurred_at)
		values
			($1, $2, $3, $4, nullif($5, '')::numeric, nullif($6, '')::numeric, $7, $8, $9, $10)
		on conflict (event_id) do nothing
	`, event.ID, event.BasketID, event.Account, event.Name, event.Amount, event.Shares, event.TxHash, event.Ledger, event.Raw, event.OccurredAt)
	return err
}

func (s *Store) UpsertBasketFromEvent(ctx context.Context, event sqimevent.ContractEvent) error {
	_, err := s.pool.Exec(ctx, `
		insert into basket_configs (basket_id, creator, name, raw_config, created_at)
		values ($1, $2, $3, $4, $5)
		on conflict (basket_id) do update
		set creator = coalesce(nullif(excluded.creator, ''), basket_configs.creator),
			name = coalesce(nullif(excluded.name, ''), basket_configs.name),
			raw_config = excluded.raw_config
	`, event.BasketID, event.Account, "Sqim Basket", event.Raw, event.OccurredAt)
	return err
}

func (s *Store) ListBaskets(ctx context.Context) ([]BasketConfig, error) {
	rows, err := s.pool.Query(ctx, `
		select basket_id, creator, name, share_token_id, assets, weights_bps, strategy, created_at
		from basket_configs
		order by created_at desc
	`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	return scanBaskets(rows)
}

func (s *Store) Basket(ctx context.Context, basketID string) (BasketConfig, error) {
	rows, err := s.pool.Query(ctx, `
		select basket_id, creator, name, share_token_id, assets, weights_bps, strategy, created_at
		from basket_configs
		where basket_id = $1
	`, basketID)
	if err != nil {
		return BasketConfig{}, err
	}
	defer rows.Close()
	baskets, err := scanBaskets(rows)
	if err != nil {
		return BasketConfig{}, err
	}
	if len(baskets) == 0 {
		return BasketConfig{}, pgx.ErrNoRows
	}
	return baskets[0], nil
}

func (s *Store) History(ctx context.Context, basketID string, limit int) ([]HistoryEvent, error) {
	rows, err := s.pool.Query(ctx, `
		select id, event_id, basket_id, account, event_type, coalesce(amount::text, ''), coalesce(shares::text, ''),
			tx_hash, ledger, raw, occurred_at
		from deposit_withdraw_events
		where basket_id = $1
		order by occurred_at desc, id desc
		limit $2
	`, basketID, limit)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var out []HistoryEvent
	for rows.Next() {
		var ev HistoryEvent
		if err := rows.Scan(&ev.ID, &ev.EventID, &ev.BasketID, &ev.Account, &ev.EventType, &ev.Amount, &ev.Shares, &ev.TxHash, &ev.Ledger, &ev.Raw, &ev.OccurredAt); err != nil {
			return nil, err
		}
		out = append(out, ev)
	}
	return out, rows.Err()
}

func (s *Store) ListStrategyBaskets(ctx context.Context) ([]BasketConfig, error) {
	rows, err := s.pool.Query(ctx, `
		select basket_id, creator, name, share_token_id, assets, weights_bps, strategy, created_at
		from basket_configs
		where strategy is not null and strategy <> '{}'::jsonb
		order by created_at
	`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	return scanBaskets(rows)
}

func (s *Store) RecordRebalance(ctx context.Context, basketID, txHash, status string, oldWeights, newWeights json.RawMessage, driftBPS int) error {
	_, err := s.pool.Exec(ctx, `
		insert into rebalance_history (basket_id, tx_hash, status, old_weights_bps, new_weights_bps, drift_bps)
		values ($1, $2, $3, $4, $5, $6)
	`, basketID, txHash, status, oldWeights, newWeights, driftBPS)
	return err
}

func scanBaskets(rows pgx.Rows) ([]BasketConfig, error) {
	var out []BasketConfig
	for rows.Next() {
		var b BasketConfig
		if err := rows.Scan(&b.BasketID, &b.Creator, &b.Name, &b.ShareTokenID, &b.Assets, &b.WeightsBPS, &b.Strategy, &b.CreatedAt); err != nil {
			return nil, err
		}
		out = append(out, b)
	}
	return out, rows.Err()
}
