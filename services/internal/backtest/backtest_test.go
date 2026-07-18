package backtest

import (
	"context"
	"math"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"
)

func TestBinanceFetchParsesPublicKlinesWithoutInventingDates(t *testing.T) {
	requests := 0
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		requests++
		if r.URL.Path != "/api/v3/klines" || r.URL.Query().Get("symbol") != "XLMUSDT" || r.URL.Query().Get("interval") != "1d" {
			t.Fatalf("unexpected Binance request: %s", r.URL.String())
		}
		w.Header().Set("content-type", "application/json")
		if requests == 1 {
			_, _ = w.Write([]byte(`[[1527724800000,"0.28021000","0.30600000","0.28021000","0.29568000","13858336.58",1527811199999,"0",1,"0","0","0"]]`))
			return
		}
		_, _ = w.Write([]byte(`[]`))
	}))
	defer server.Close()

	provider := Binance{BaseURL: server.URL, Client: server.Client()}
	rows, err := provider.Fetch(context.Background(), Catalog["XLM"], "daily", time.Date(2018, 1, 1, 0, 0, 0, 0, time.UTC), time.Date(2019, 1, 1, 0, 0, 0, 0, time.UTC))
	if err != nil {
		t.Fatal(err)
	}
	if len(rows) != 1 || rows[0].Timestamp != 1527724800 || rows[0].Close != 0.29568 || rows[0].Provider != "Binance Public Data" {
		t.Fatalf("unexpected parsed Binance row: %#v", rows)
	}
}

func TestRunUsesOnlyOverlappingRealDatesAndYoungestAsset(t *testing.T) {
	store := NewStore(t.TempDir())
	start := time.Date(2020, 1, 1, 0, 0, 0, 0, time.UTC)
	btc := testRows("BTC", "BTCUSDT", start, []float64{100, 110, 100, 120, 125})
	eth := testRows("ETH", "ETHUSDT", start.Add(24*time.Hour), []float64{100, 90, 95, 99})
	if err := store.Write("BTC", "daily", btc); err != nil {
		t.Fatal(err)
	}
	if err := store.Write("ETH", "daily", eth); err != nil {
		t.Fatal(err)
	}
	engine := Engine{Store: store}
	result, err := engine.Run(Request{Assets: []string{"BTC", "ETH"}, WeightsBPS: []int{5000, 5000}, Granularity: "daily"})
	if err != nil {
		t.Fatal(err)
	}
	if len(result.Series) != 4 {
		t.Fatalf("expected four shared dates, got %d", len(result.Series))
	}
	if result.YoungestConstituent != "ETH" {
		t.Fatalf("expected ETH to limit history, got %s", result.YoungestConstituent)
	}
	if !result.AvailableFrom.Equal(start.Add(24 * time.Hour)) {
		t.Fatalf("unexpected overlap start: %s", result.AvailableFrom)
	}
	if math.IsNaN(result.TotalReturn) || math.IsNaN(result.AnnualizedVolatility) || result.MaxDrawdown > 0 {
		t.Fatalf("invalid metrics: %#v", result)
	}
	if result.Disclaimer != Disclaimer {
		t.Fatal("simulation disclaimer must be returned with every result")
	}
	windowed, err := engine.Run(Request{Assets: []string{"BTC", "ETH"}, WeightsBPS: []int{5000, 5000}, From: "2020-01-03", Granularity: "daily"})
	if err != nil {
		t.Fatal(err)
	}
	if windowed.YoungestConstituent != "ETH" {
		t.Fatalf("selected window must not change the actual youngest constituent, got %s", windowed.YoungestConstituent)
	}
}

func TestRunRejectsUnsupportedAndMalformedAllocations(t *testing.T) {
	engine := Engine{Store: NewStore(t.TempDir())}
	if _, err := engine.Run(Request{Assets: []string{"THIN-STELLAR-ASSET"}, WeightsBPS: []int{10_000}}); err == nil {
		t.Fatal("expected an unsupported asset to fail instead of fabricating history")
	}
	if _, err := engine.Run(Request{Assets: []string{"BTC", "ETH"}, WeightsBPS: []int{5000, 4000}}); err == nil {
		t.Fatal("expected a non-100 percent allocation to fail")
	}
}

func TestStoreWritesAndReadsParquet(t *testing.T) {
	store := NewStore(t.TempDir())
	rows := testRows("XLM", "XLMUSDT", time.Date(2019, 3, 14, 0, 0, 0, 0, time.UTC), []float64{0.1, 0.11})
	if err := store.Write("XLM", "daily", rows); err != nil {
		t.Fatal(err)
	}
	read, err := store.Read("XLM", "daily")
	if err != nil {
		t.Fatal(err)
	}
	if len(read) != 2 || read[1].Close != 0.11 || read[0].Provider != "Binance Public Data" {
		t.Fatalf("unexpected Parquet round trip: %#v", read)
	}
}

func testRows(symbol, product string, start time.Time, closes []float64) []PriceRow {
	rows := make([]PriceRow, len(closes))
	for index, closePrice := range closes {
		rows[index] = PriceRow{Timestamp: start.Add(time.Duration(index) * 24 * time.Hour).Unix(), Symbol: symbol, ProductID: product, Granularity: "daily", Open: closePrice, High: closePrice, Low: closePrice, Close: closePrice, Volume: 1, Provider: "Binance Public Data"}
	}
	return rows
}
