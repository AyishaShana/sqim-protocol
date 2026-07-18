package config

import (
	"os"
	"strconv"
	"strings"
	"time"
)

type Config struct {
	APIAddr            string
	DatabaseURL        string
	RedisURL           string
	SorobanRPCURL      string
	NetworkPassphrase  string
	ContractIDs        []string
	PollInterval       time.Duration
	StartLedger        uint32
	SchemaPath         string
	StellarCLIPath     string
	SourceAccount      string
	RelayerCaller      string
	RelayerSignWithKey string
	RelayerAuthHelper  string
	RelayerAuthScript  string
	RelayerDryRun      bool
	RebalancerSigners  []string
	RebalancerQuorum   int
	StrategyInterval   time.Duration
	AllowTestFixtures  bool
}

func Load() Config {
	return Config{
		APIAddr:            env("API_ADDR", ":8080"),
		DatabaseURL:        env("DATABASE_URL", "postgres://sqim:sqim@localhost:5432/sqim?sslmode=disable"),
		RedisURL:           env("REDIS_URL", "redis://localhost:6379/0"),
		SorobanRPCURL:      env("SOROBAN_RPC_URL", ""),
		NetworkPassphrase:  env("SOROBAN_NETWORK_PASSPHRASE", ""),
		ContractIDs:        split(env("SQIM_CONTRACT_IDS", "")),
		PollInterval:       duration("INDEXER_POLL_INTERVAL", 8*time.Second),
		StartLedger:        uint32(intEnv("INDEXER_START_LEDGER", 3608727)),
		SchemaPath:         env("SCHEMA_PATH", "db/schema.sql"),
		StellarCLIPath:     env("STELLAR_CLI_PATH", "stellar"),
		SourceAccount:      env("RELAYER_SOURCE_ACCOUNT", "ayisha"),
		RelayerCaller:      env("RELAYER_CALLER_ADDRESS", ""),
		RelayerSignWithKey: env("RELAYER_SIGN_WITH_KEY", ""),
		RelayerAuthHelper:  env("RELAYER_AUTH_HELPER", ""),
		RelayerAuthScript:  env("RELAYER_AUTH_HELPER_SCRIPT", ""),
		RelayerDryRun:      boolEnv("RELAYER_DRY_RUN", true),
		RebalancerSigners:  split(env("RELAYER_REBALANCER_SIGNERS", "")),
		RebalancerQuorum:   intEnv("RELAYER_REBALANCER_QUORUM", 2),
		StrategyInterval:   duration("RELAYER_INTERVAL", 60*time.Second),
		AllowTestFixtures:  boolEnv("SQIM_ALLOW_TEST_FIXTURES", false),
	}
}

func env(key, fallback string) string {
	if v := strings.TrimSpace(os.Getenv(key)); v != "" {
		return v
	}
	return fallback
}

func split(v string) []string {
	parts := strings.Split(v, ",")
	out := make([]string, 0, len(parts))
	for _, part := range parts {
		if trimmed := strings.TrimSpace(part); trimmed != "" {
			out = append(out, trimmed)
		}
	}
	return out
}

func intEnv(key string, fallback int) int {
	raw := strings.TrimSpace(os.Getenv(key))
	if raw == "" {
		return fallback
	}
	n, err := strconv.Atoi(raw)
	if err != nil {
		return fallback
	}
	return n
}

func boolEnv(key string, fallback bool) bool {
	raw := strings.TrimSpace(os.Getenv(key))
	if raw == "" {
		return fallback
	}
	v, err := strconv.ParseBool(raw)
	if err != nil {
		return fallback
	}
	return v
}

func duration(key string, fallback time.Duration) time.Duration {
	raw := strings.TrimSpace(os.Getenv(key))
	if raw == "" {
		return fallback
	}
	d, err := time.ParseDuration(raw)
	if err != nil {
		return fallback
	}
	return d
}
