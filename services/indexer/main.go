package main

import (
	"context"
	"log"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/AyishaShana/sqim-protocol/services/internal/cache"
	"github.com/AyishaShana/sqim-protocol/services/internal/config"
	"github.com/AyishaShana/sqim-protocol/services/internal/soroban"
	"github.com/AyishaShana/sqim-protocol/services/internal/sqimevent"
	"github.com/AyishaShana/sqim-protocol/services/internal/store"
)

const cursorKey = "soroban-events"

func main() {
	cfg := config.Load()
	if cfg.SorobanRPCURL == "" || len(cfg.ContractIDs) == 0 {
		log.Fatal("SOROBAN_RPC_URL and SQIM_CONTRACT_IDS are required")
	}
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
	if !cfg.AllowTestFixtures {
		if err := db.AssertNoFixtureIDs(ctx); err != nil {
			log.Fatalf("refuse non-test database with fixture contract IDs: %v", err)
		}
	}

	client := soroban.NewClient(cfg.SorobanRPCURL)
	metricsCache, err := cache.New(cfg.RedisURL)
	if err != nil {
		log.Fatalf("configure redis: %v", err)
	}
	defer metricsCache.Close()
	if err := metricsCache.Ping(ctx); err != nil {
		log.Fatalf("connect redis: %v", err)
	}
	ticker := time.NewTicker(cfg.PollInterval)
	defer ticker.Stop()

	log.Printf("indexer watching %d contracts on %s", len(cfg.ContractIDs), cfg.SorobanRPCURL)
	for {
		if err := poll(ctx, db, metricsCache, client, cfg); err != nil {
			log.Printf("poll failed: %v", err)
		}
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
		}
	}
}

func poll(
	ctx context.Context,
	db *store.Store,
	metricsCache *cache.Cache,
	client *soroban.Client,
	cfg config.Config,
) error {
	cursor, err := db.Cursor(ctx, cursorKey)
	if err != nil {
		return err
	}
	discovered, err := db.WatchedContractIDs(ctx)
	if err != nil {
		return err
	}
	contractIDs := mergeContractIDs(cfg.ContractIDs, discovered)
	events, nextCursor, err := client.GetEvents(ctx, soroban.GetEventsParams{
		StartLedger: cfg.StartLedger,
		Cursor:      cursor,
		ContractIDs: contractIDs,
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
		if normalized.Name == "transfer" {
			basketID, resolveErr := db.BasketForContract(ctx, normalized.ContractID)
			if resolveErr != nil {
				continue
			}
			normalized.BasketID = basketID
		}
		if err := db.InsertEvent(ctx, normalized); err != nil {
			return err
		}
		if normalized.NAV != "" && normalized.AUM != "" {
			if err := metricsCache.SetMetrics(ctx, normalized.BasketID, cache.Metrics{
				NAV: normalized.NAV, AUM: normalized.AUM, Ledger: normalized.Ledger,
				AsOf: normalized.OccurredAt.UTC().Format(time.RFC3339), Source: "indexed_soroban_event",
			}, 15*time.Minute); err != nil {
				return err
			}
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
	case "deposit", "withdraw", "rebalance", "basket-created", "transfer", "basis":
		return true
	default:
		return false
	}
}

func mergeContractIDs(groups ...[]string) []string {
	seen := make(map[string]struct{})
	var out []string
	for _, group := range groups {
		for _, id := range group {
			if id == "" {
				continue
			}
			if _, ok := seen[id]; ok {
				continue
			}
			seen[id] = struct{}{}
			out = append(out, id)
		}
	}
	return out
}
