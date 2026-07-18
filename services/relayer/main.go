package main

import (
	"context"
	"encoding/json"
	"errors"
	"log"
	"os"
	"os/exec"
	"os/signal"
	"regexp"
	"strconv"
	"syscall"
	"time"

	"github.com/AyishaShana/sqim-protocol/services/internal/config"
	"github.com/AyishaShana/sqim-protocol/services/internal/store"
)

type Strategy struct {
	Type              string   `json:"type"`
	DriftThresholdBPS int      `json:"drift_threshold_bps"`
	TargetWeightsBPS  []uint32 `json:"target_weights_bps"`
}

func main() {
	cfg := config.Load()
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	if err := validateTrustModel(cfg); err != nil {
		log.Fatalf("invalid relayer trust config: %v", err)
	}

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

	ticker := time.NewTicker(cfg.StrategyInterval)
	defer ticker.Stop()

	log.Printf("relayer started dry_run=%t quorum=%d signers=%d", cfg.RelayerDryRun, cfg.RebalancerQuorum, len(cfg.RebalancerSigners))
	for {
		if err := runOnce(ctx, db, cfg); err != nil {
			log.Printf("relayer cycle failed: %v", err)
		}
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
		}
	}
}

func validateTrustModel(cfg config.Config) error {
	if cfg.RelayerDryRun {
		return nil
	}
	if cfg.RebalancerQuorum < 2 {
		return errors.New("RELAYER_REBALANCER_QUORUM must be at least 2 when dry-run is disabled")
	}
	if len(cfg.RebalancerSigners) < cfg.RebalancerQuorum {
		return errors.New("RELAYER_REBALANCER_SIGNERS must contain enough signers for quorum")
	}
	if cfg.RebalancerQuorum != 2 {
		return errors.New("live v1 relayer currently supports exactly a 2-of-N on-chain quorum")
	}
	if cfg.RelayerCaller == "" {
		return errors.New("RELAYER_CALLER_ADDRESS is required when dry-run is disabled")
	}
	if cfg.RelayerAuthHelper == "" || cfg.RelayerAuthScript == "" {
		return errors.New("RELAYER_AUTH_HELPER and RELAYER_AUTH_HELPER_SCRIPT are required for live quorum signing")
	}
	foundCaller := false
	for _, signer := range cfg.RebalancerSigners {
		if signer == cfg.RelayerCaller {
			foundCaller = true
			break
		}
	}
	if !foundCaller {
		return errors.New("RELAYER_CALLER_ADDRESS must be in RELAYER_REBALANCER_SIGNERS")
	}
	return nil
}

func runOnce(ctx context.Context, db *store.Store, cfg config.Config) error {
	baskets, err := db.ListStrategyBaskets(ctx)
	if err != nil {
		return err
	}
	for _, basket := range baskets {
		strategy, err := parseStrategy(basket.Strategy)
		if err != nil {
			log.Printf("skip basket %s: %v", basket.BasketID, err)
			continue
		}
		newWeights, err := json.Marshal(strategy.TargetWeightsBPS)
		if err != nil {
			return err
		}
		if !shouldRebalance(strategy, basket.WeightsBPS) {
			if err := db.RecordRebalance(ctx, basket.BasketID, "", "skipped", basket.WeightsBPS, newWeights, 0); err != nil {
				return err
			}
			continue
		}
		status := "dry_run"
		txHash := ""
		if !cfg.RelayerDryRun {
			txHash, err = submitRebalance(ctx, cfg, basket.BasketID, strategy.TargetWeightsBPS)
			if err != nil {
				status = "failed"
				log.Printf("submit rebalance failed basket=%s: %v", basket.BasketID, err)
			} else {
				status = "submitted"
			}
		}
		if err := db.RecordRebalance(ctx, basket.BasketID, txHash, status, basket.WeightsBPS, newWeights, strategy.DriftThresholdBPS); err != nil {
			return err
		}
	}
	return nil
}

func parseStrategy(raw json.RawMessage) (Strategy, error) {
	var strategy Strategy
	if len(raw) == 0 {
		return strategy, errors.New("missing strategy")
	}
	if err := json.Unmarshal(raw, &strategy); err != nil {
		return strategy, err
	}
	if strategy.Type != "drift_threshold" && strategy.Type != "calendar" {
		return strategy, errors.New("unsupported strategy type")
	}
	if len(strategy.TargetWeightsBPS) == 0 {
		return strategy, errors.New("strategy target weights are required")
	}
	return strategy, nil
}

func shouldRebalance(strategy Strategy, rawWeights json.RawMessage) bool {
	if strategy.Type == "calendar" {
		return true
	}
	var current []uint32
	if err := json.Unmarshal(rawWeights, &current); err != nil || len(current) != len(strategy.TargetWeightsBPS) {
		return true
	}
	return maxDriftBPS(current, strategy.TargetWeightsBPS) >= strategy.DriftThresholdBPS
}

func maxDriftBPS(a, b []uint32) int {
	max := 0
	for i := range a {
		diff := int(a[i]) - int(b[i])
		if diff < 0 {
			diff = -diff
		}
		if diff > max {
			max = diff
		}
	}
	return max
}

func submitRebalance(ctx context.Context, cfg config.Config, basketID string, weights []uint32) (string, error) {
	args := authHelperArgs(cfg, basketID, weights)
	cmd := exec.CommandContext(ctx, cfg.RelayerAuthHelper, args...)
	out, err := cmd.CombinedOutput()
	if err != nil {
		return "", errors.New(string(out))
	}
	hash := regexp.MustCompile(`(?i)\b[0-9a-f]{64}\b`).FindString(string(out))
	if hash == "" {
		return "", errors.New("rebalance auth helper succeeded without returning a transaction hash")
	}
	return hash, nil
}

func authHelperArgs(cfg config.Config, basketID string, weights []uint32) []string {
	return []string{
		cfg.RelayerAuthScript,
		basketID,
		weightsArg(weights),
		signersArg(cfg.RebalancerSigners[:cfg.RebalancerQuorum]),
	}
}

func weightsArg(weights []uint32) string {
	out := "["
	for i, weight := range weights {
		if i > 0 {
			out += ","
		}
		out += strconv.FormatUint(uint64(weight), 10)
	}
	return out + "]"
}

func signersArg(signers []string) string {
	encoded, err := json.Marshal(signers)
	if err != nil {
		panic(err)
	}
	return string(encoded)
}
