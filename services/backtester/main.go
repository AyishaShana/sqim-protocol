package main

import (
	"context"
	"encoding/json"
	"flag"
	"log"
	"net/http"
	"os"
	"strings"
	"time"

	"github.com/AyishaShana/sqim-protocol/services/internal/backtest"
)

func main() {
	syncOnly := flag.Bool("sync-only", false, "sync configured real market data to Parquet and exit")
	granularity := flag.String("granularity", "daily", "Parquet candle granularity: daily or minute")
	from := flag.String("from", "", "optional sync start date in YYYY-MM-DD form; recommended for minute data")
	flag.Parse()
	root := env("BACKTEST_DATA_DIR", "backtester/data")
	engine := &backtest.Engine{
		Store:    backtest.NewStore(root),
		Provider: backtest.Binance{BaseURL: env("MARKET_DATA_BASE_URL", "https://data-api.binance.vision")},
	}
	if *from != "" {
		parsed, err := time.Parse("2006-01-02", *from)
		if err != nil {
			log.Fatal("--from must use YYYY-MM-DD")
		}
		engine.Earliest = parsed.UTC()
	}
	if *syncOnly {
		histories, err := engine.Sync(context.Background(), configuredSymbols(), *granularity)
		if err != nil {
			log.Fatal(err)
		}
		for _, history := range histories {
			log.Printf("%s: %d %s candles, %s to %s", history.Symbol, history.Points, history.Granularity, history.First.Format("2006-01-02"), history.Last.Format("2006-01-02"))
		}
		return
	}

	mux := http.NewServeMux()
	mux.HandleFunc("GET /health", func(w http.ResponseWriter, _ *http.Request) {
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok", "mode": "historical-simulation", "provider": "binance-public-data", "quote_currency": "USDT"})
	})
	mux.HandleFunc("GET /assets", func(w http.ResponseWriter, r *http.Request) {
		histories, err := engine.Histories(r.URL.Query().Get("granularity"))
		if err != nil {
			writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
			return
		}
		writeJSON(w, http.StatusOK, histories)
	})
	mux.HandleFunc("POST /backtests", func(w http.ResponseWriter, r *http.Request) {
		var request backtest.Request
		if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 64<<10)).Decode(&request); err != nil {
			writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid backtest request"})
			return
		}
		result, err := engine.Run(request)
		if err != nil {
			writeJSON(w, http.StatusUnprocessableEntity, map[string]string{"error": err.Error()})
			return
		}
		writeJSON(w, http.StatusOK, result)
	})

	server := &http.Server{Addr: env("BACKTESTER_ADDR", ":8090"), Handler: cors(mux), ReadHeaderTimeout: 5 * time.Second}
	log.Printf("sqim backtester listening on %s with Parquet data at %s", server.Addr, root)
	log.Fatal(server.ListenAndServe())
}

func configuredSymbols() []string {
	raw := env("BACKTEST_ASSETS", "BTC,ETH,XLM,SOL,USDC")
	parts := strings.Split(raw, ",")
	out := make([]string, 0, len(parts))
	for _, part := range parts {
		if symbol := strings.TrimSpace(part); symbol != "" {
			out = append(out, symbol)
		}
	}
	return out
}

func env(key, fallback string) string {
	if value := strings.TrimSpace(os.Getenv(key)); value != "" {
		return value
	}
	return fallback
}

func cors(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("access-control-allow-origin", "*")
		w.Header().Set("access-control-allow-methods", "GET, POST, OPTIONS")
		w.Header().Set("access-control-allow-headers", "content-type")
		if r.Method == http.MethodOptions {
			w.WriteHeader(http.StatusNoContent)
			return
		}
		next.ServeHTTP(w, r)
	})
}

func writeJSON(w http.ResponseWriter, status int, value any) {
	w.Header().Set("content-type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(value)
}
