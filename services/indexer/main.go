package main

import (
	"context"
	"log"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/AyishaShana/sqim-protocol/services/internal/config"
	"github.com/AyishaShana/sqim-protocol/services/internal/soroban"
	"github.com/AyishaShana/sqim-protocol/services/internal/sqimevent"
	"github.com/AyishaShana/sqim-protocol/services/internal/store"
)

const cursorKey = "soroban-events"

func main() {
	cfg := config.Load()
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	db, err := store.New(ctx, cfg.DatabaseURL)
	if err != nil {
		log.Fatalf("connect postgres: %v", err)
	}
	defer db.Close()
	if err := db.ApplySchemaFile(ctx, cfg.SchemaPath); err != nil {
		log.Fatalf("apply schema: %v", err)
	}

	client := soroban.NewClient(cfg.SorobanRPCURL)
	ticker := time.NewTicker(cfg.PollInterval)
	defer ticker.Stop()

	log.Printf("indexer watching %d contracts on %s", len(cfg.ContractIDs), cfg.SorobanRPCURL)
	for {
		if err := poll(ctx, db, client, cfg); err != nil {
			log.Printf("poll failed: %v", err)
		}
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
		}
	}
}

func poll(ctx context.Context, db *store.Store, client *soroban.Client, cfg config.Config) error {
	cursor, err := db.Cursor(ctx, cursorKey)
	if err != nil {
		return err
	}
	events, nextCursor, err := client.GetEvents(ctx, soroban.GetEventsParams{
		StartLedger: cfg.StartLedger,
		Cursor:      cursor,
		ContractIDs: cfg.ContractIDs,
		Limit:       100,
	})
	if err != nil {
		return err
	}
	var lastLedger uint32
	for _, rpcEvent := range events {
		if !wanted(rpcEvent) {
			continue
		}
		normalized := sqimevent.Normalize(rpcEvent)
		if err := db.InsertEvent(ctx, normalized); err != nil {
			return err
		}
		lastLedger = normalized.Ledger
	}
	if nextCursor != "" {
		return db.SaveCursor(ctx, cursorKey, nextCursor, lastLedger)
	}
	return nil
}

func wanted(event sqimevent.RPCEvent) bool {
	normalized := sqimevent.Normalize(event)
	switch normalized.Name {
	case "deposit", "withdraw", "rebalance", "basket-created":
		return true
	default:
		return false
	}
}
