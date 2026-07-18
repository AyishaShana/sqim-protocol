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
	ID           int64           `json:"id"`
	EventID      string          `json:"event_id"`
	BasketID     string          `json:"basket_id"`
	Account      string          `json:"account"`
	Counterparty string          `json:"counterparty"`
	EventType    string          `json:"event_type"`
	Amount       string          `json:"amount"`
	Shares       string          `json:"shares"`
	Fee          string          `json:"fee"`
	NAV          string          `json:"nav"`
	AUM          string          `json:"aum"`
	TxHash       string          `json:"tx_hash"`
	Ledger       int64           `json:"ledger"`
	Raw          json.RawMessage `json:"raw"`
	OccurredAt   time.Time       `json:"occurred_at"`
}

type PortfolioHolding struct {
	BasketID string `json:"basket_id"`
	Shares   string `json:"shares"`
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

func (s *Store) Ping(ctx context.Context) error {
	return s.pool.Ping(ctx)
}

func (s *Store) DeleteBasket(ctx context.Context, basketID string) error {
	tx, err := s.pool.Begin(ctx)
	if err != nil {
		return err
	}
	defer tx.Rollback(ctx)

	if _, err := tx.Exec(ctx, `delete from deposit_withdraw_events where basket_id = $1`, basketID); err != nil {
		return err
	}
	if _, err := tx.Exec(ctx, `delete from basket_configs where basket_id = $1`, basketID); err != nil {
		return err
	}
	return tx.Commit(ctx)
}

func (s *Store) ApplySchemaFile(ctx context.Context, path string) error {
	schema, err := os.ReadFile(path)
	if err != nil {
		return err
	}
	tx, err := s.pool.Begin(ctx)
	if err != nil {
		return err
	}
	defer tx.Rollback(ctx)
	if _, err := tx.Exec(ctx, `select pg_advisory_xact_lock(hashtext('sqim-schema'))`); err != nil {
		return err
	}
	if _, err := tx.Exec(ctx, string(schema)); err != nil {
		return err
	}
	return tx.Commit(ctx)
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
            (event_id, basket_id, account, counterparty, event_type, amount, shares, fee, nav, aum,
             tx_hash, ledger, raw, occurred_at)
        values
            ($1, $2, $3, $4, $5, nullif($6, '')::numeric, nullif($7, '')::numeric,
             nullif($8, '')::numeric, nullif($9, '')::numeric, nullif($10, '')::numeric,
             $11, $12, $13, $14)
        on conflict (event_id) do nothing
    `, event.ID, event.BasketID, event.Account, event.Counterparty, event.Name, event.Amount,
		event.Shares, event.Fee, event.NAV, event.AUM, event.TxHash, event.Ledger, event.Raw,
		event.OccurredAt)
	if err == nil && event.Name == "rebalance" && len(event.WeightsBPS) > 0 {
		_, err = s.pool.Exec(ctx, `
            update basket_configs
            set weights_bps = $2, updated_at = now()
            where basket_id = $1
        `, event.BasketID, event.WeightsBPS)
	}
	return err
}

func (s *Store) UpsertBasketFromEvent(ctx context.Context, event sqimevent.ContractEvent) error {
	_, err := s.pool.Exec(ctx, `
        insert into basket_configs
            (basket_id, creator, name, share_token_id, assets, weights_bps, raw_config, created_at)
        values ($1, $2, $3, $4, $5, $6, $7, $8)
        on conflict (basket_id) do update
        set creator = coalesce(nullif(excluded.creator, ''), basket_configs.creator),
            name = coalesce(nullif(excluded.name, ''), basket_configs.name),
            share_token_id = coalesce(nullif(excluded.share_token_id, ''), basket_configs.share_token_id),
            assets = case when excluded.assets = '[]'::jsonb then basket_configs.assets else excluded.assets end,
            weights_bps = case when excluded.weights_bps = '[]'::jsonb then basket_configs.weights_bps else excluded.weights_bps end,
            raw_config = excluded.raw_config
    `, event.BasketID, event.Account, firstNonEmpty(event.BasketName, "Sqim Basket"),
		event.ShareTokenID, jsonOrEmptyArray(event.Assets), jsonOrEmptyArray(event.WeightsBPS),
		event.Raw, event.OccurredAt)
	return err
}

func (s *Store) WatchedContractIDs(ctx context.Context) ([]string, error) {
	rows, err := s.pool.Query(ctx, `
        select basket_id from basket_configs
        union
        select share_token_id from basket_configs where share_token_id <> ''
    `)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var ids []string
	for rows.Next() {
		var id string
		if err := rows.Scan(&id); err != nil {
			return nil, err
		}
		ids = append(ids, id)
	}
	return ids, rows.Err()
}

func (s *Store) BasketForContract(ctx context.Context, contractID string) (string, error) {
	var basketID string
	err := s.pool.QueryRow(ctx, `
        select basket_id
        from basket_configs
        where basket_id = $1 or share_token_id = $1
        limit 1
    `, contractID).Scan(&basketID)
	return basketID, err
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
        select id, event_id, basket_id, account, counterparty, event_type,
            coalesce(amount::text, ''), coalesce(shares::text, ''), coalesce(fee::text, ''),
            coalesce(nav::text, ''), coalesce(aum::text, ''), tx_hash, ledger, raw, occurred_at
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
		if err := rows.Scan(&ev.ID, &ev.EventID, &ev.BasketID, &ev.Account, &ev.Counterparty,
			&ev.EventType, &ev.Amount, &ev.Shares, &ev.Fee, &ev.NAV, &ev.AUM, &ev.TxHash,
			&ev.Ledger, &ev.Raw, &ev.OccurredAt); err != nil {
			return nil, err
		}
		out = append(out, ev)
	}
	return out, rows.Err()
}

func (s *Store) Portfolio(ctx context.Context, account string) ([]PortfolioHolding, error) {
	rows, err := s.pool.Query(ctx, `
        select basket_id, coalesce(sum(delta), 0)::text
        from (
            select basket_id,
                case event_type
                    when 'deposit' then coalesce(shares, 0)
                    when 'withdraw' then -coalesce(shares, 0)
					when 'basis' then -coalesce(shares, 0)
                    else 0
                end as delta
            from deposit_withdraw_events
            where account = $1
            union all
            select basket_id, coalesce(shares, amount, 0) as delta
            from deposit_withdraw_events
			where event_type = 'basis' and counterparty = $1
        ) positions
        group by basket_id
        having sum(delta) <> 0
        order by basket_id
    `, account)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var holdings []PortfolioHolding
	for rows.Next() {
		var holding PortfolioHolding
		if err := rows.Scan(&holding.BasketID, &holding.Shares); err != nil {
			return nil, err
		}
		holdings = append(holdings, holding)
	}
	return holdings, rows.Err()
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

func jsonOrEmptyArray(value json.RawMessage) json.RawMessage {
	if len(value) == 0 {
		return json.RawMessage(`[]`)
	}
	return value
}

func firstNonEmpty(values ...string) string {
	for _, value := range values {
		if value != "" {
			return value
		}
	}
	return ""
}
