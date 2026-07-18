package store

import (
	"context"
	"time"
)

type NAVPoint struct {
	At        time.Time `json:"at"`
	NAV       string    `json:"nav"`
	EventType string    `json:"event_type"`
	TxHash    string    `json:"tx_hash"`
	Ledger    int64     `json:"ledger"`
}

func (s *Store) DeploymentTime(ctx context.Context, basketID string) (time.Time, error) {
	var deployedAt time.Time
	err := s.pool.QueryRow(ctx, `select created_at from basket_configs where basket_id = $1`, basketID).Scan(&deployedAt)
	return deployedAt, err
}

func (s *Store) NAVHistory(ctx context.Context, basketID string) ([]NAVPoint, error) {
	rows, err := s.pool.Query(ctx, `
		select occurred_at, nav::text, event_type, tx_hash, ledger
		from deposit_withdraw_events
		where basket_id = $1 and nav is not null and nav > 0
		order by occurred_at, id
	`, basketID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	points := make([]NAVPoint, 0)
	for rows.Next() {
		var point NAVPoint
		if err := rows.Scan(&point.At, &point.NAV, &point.EventType, &point.TxHash, &point.Ledger); err != nil {
			return nil, err
		}
		points = append(points, point)
	}
	return points, rows.Err()
}
