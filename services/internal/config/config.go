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
	RelayerDryRun      bool
	RebalancerSigners  []string
	RebalancerQuorum   int
	StrategyInterval   time.Duration
}

func Load() Config {
	return Config{
		APIAddr:           env("API_ADDR", ":8080"),
		DatabaseURL:       env("DATABASE_URL", "postgres://sqim:sqim@localhost:5432/sqim?sslmode=disable"),
		RedisURL:          env("REDIS_URL", "redis://localhost:6379/0"),
		SorobanRPCURL:     env("SOROBAN_RPC_URL", "https://soroban-testnet.stellar.org"),
		NetworkPassphrase: env("SOROBAN_NETWORK_PASSPHRASE", "Test SDF Network ; September 2015"),
		ContractIDs: split(env("SQIM_CONTRACT_IDS",
			"CBYWTMUFK6DXO4CN4QZASWXAK7BXGJLPWDQNA3CNBOMRCX7GUGTNNKPZ,CABCGGFYGPWYNRPJIXFN6YHGER7YHY4CH4GWHQZUHAFEO7A6EJNS64VZ,CARGKEM34YZ4DCNMSSLMOPMIAAJGAF5CKGAAMJWV7E2AI5QFFGPGYEBM,CDNKWO64BEB4GJ4EAVSIHE3IVCGYI25UZK6DYD3CGSSRJBJGN63N3K7U,CDMZIC6FKPT6B6LYPAZVXBV2COP5BZSAM4VIIY67OGGGVPNBW7QWAKNR,CASNUWD2Z4RCUUY4LZEO7XRKHGMNRR55K7BMMUUH6PVIMYZF4IMU5RS4")),
		PollInterval:     duration("INDEXER_POLL_INTERVAL", 8*time.Second),
		StartLedger:      uint32(intEnv("INDEXER_START_LEDGER", 0)),
		SchemaPath:       env("SCHEMA_PATH", "db/schema.sql"),
		StellarCLIPath:   env("STELLAR_CLI_PATH", "stellar"),
		SourceAccount:    env("RELAYER_SOURCE_ACCOUNT", "ayisha"),
		RelayerDryRun:    boolEnv("RELAYER_DRY_RUN", true),
		RebalancerSigners: split(env("RELAYER_REBALANCER_SIGNERS", "")),
		RebalancerQuorum: intEnv("RELAYER_REBALANCER_QUORUM", 2),
		StrategyInterval: duration("RELAYER_INTERVAL", 60*time.Second),
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
