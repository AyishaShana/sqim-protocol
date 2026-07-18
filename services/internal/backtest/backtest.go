package backtest

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"math"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/parquet-go/parquet-go"
)

const Disclaimer = "Simulated using historical prices, not a live basket track record. Results exclude fees, slippage, taxes, and liquidity constraints and are not investment advice."

type Asset struct {
	Symbol    string `json:"symbol"`
	Name      string `json:"name"`
	ProductID string `json:"product_id"`
}

var Catalog = map[string]Asset{
	"BTC":  {Symbol: "BTC", Name: "Bitcoin", ProductID: "BTCUSDT"},
	"ETH":  {Symbol: "ETH", Name: "Ether", ProductID: "ETHUSDT"},
	"XLM":  {Symbol: "XLM", Name: "Stellar Lumens", ProductID: "XLMUSDT"},
	"SOL":  {Symbol: "SOL", Name: "Solana", ProductID: "SOLUSDT"},
	"USDC": {Symbol: "USDC", Name: "USD Coin", ProductID: "USDCUSDT"},
}

type PriceRow struct {
	Timestamp   int64   `parquet:"timestamp" json:"timestamp"`
	Symbol      string  `parquet:"symbol" json:"symbol"`
	ProductID   string  `parquet:"product_id" json:"product_id"`
	Granularity string  `parquet:"granularity" json:"granularity"`
	Open        float64 `parquet:"open" json:"open"`
	High        float64 `parquet:"high" json:"high"`
	Low         float64 `parquet:"low" json:"low"`
	Close       float64 `parquet:"close" json:"close"`
	Volume      float64 `parquet:"volume" json:"volume"`
	Provider    string  `parquet:"provider" json:"provider"`
}

type AssetHistory struct {
	Asset
	Granularity string    `json:"granularity"`
	Available   bool      `json:"available"`
	First       time.Time `json:"first,omitempty"`
	Last        time.Time `json:"last,omitempty"`
	Points      int       `json:"points"`
	Provider    string    `json:"provider"`
	SourceURL   string    `json:"source_url"`
}

type Request struct {
	Assets      []string `json:"assets"`
	WeightsBPS  []int    `json:"weights_bps"`
	From        string   `json:"from,omitempty"`
	To          string   `json:"to,omitempty"`
	Granularity string   `json:"granularity,omitempty"`
}

type Point struct {
	At    time.Time `json:"at"`
	Value float64   `json:"value"`
}

type Result struct {
	Series               []Point        `json:"series"`
	TotalReturn          float64        `json:"total_return"`
	AnnualizedVolatility float64        `json:"annualized_volatility"`
	MaxDrawdown          float64        `json:"max_drawdown"`
	AnnualizedReturn     float64        `json:"annualized_return"`
	AvailableFrom        time.Time      `json:"available_from"`
	AvailableTo          time.Time      `json:"available_to"`
	AvailableYears       float64        `json:"available_years"`
	YoungestConstituent  string         `json:"youngest_constituent"`
	Granularity          string         `json:"granularity"`
	Methodology          string         `json:"methodology"`
	Provider             string         `json:"provider"`
	Disclaimer           string         `json:"disclaimer"`
	AssetHistory         []AssetHistory `json:"asset_history"`
}

type Store struct {
	root string
	mu   sync.Mutex
}

func NewStore(root string) *Store {
	return &Store{root: root}
}

func (s *Store) Read(symbol, granularity string) ([]PriceRow, error) {
	rows, err := parquet.ReadFile[PriceRow](s.path(symbol, granularity))
	if errors.Is(err, os.ErrNotExist) {
		return nil, nil
	}
	return rows, err
}

func (s *Store) Write(symbol, granularity string, rows []PriceRow) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	if err := os.MkdirAll(s.root, 0o755); err != nil {
		return err
	}
	sort.Slice(rows, func(i, j int) bool { return rows[i].Timestamp < rows[j].Timestamp })
	return parquet.WriteFile(s.path(symbol, granularity), rows)
}

func (s *Store) path(symbol, granularity string) string {
	clean := strings.ToLower(strings.ReplaceAll(symbol, "/", "-"))
	return filepath.Join(s.root, clean+"-"+granularity+".parquet")
}

type Binance struct {
	BaseURL string
	Client  *http.Client
}

func (b Binance) Fetch(ctx context.Context, asset Asset, granularity string, from, to time.Time) ([]PriceRow, error) {
	seconds, err := granularitySeconds(granularity)
	if err != nil {
		return nil, err
	}
	client := b.Client
	if client == nil {
		client = &http.Client{Timeout: 20 * time.Second}
	}
	base := strings.TrimRight(b.BaseURL, "/")
	if base == "" {
		base = "https://data-api.binance.vision"
	}
	interval := map[string]string{"daily": "1d", "minute": "1m"}[granularity]
	byTime := map[int64]PriceRow{}
	startMillis := from.UTC().UnixMilli()
	endMillis := to.UTC().UnixMilli()
	for startMillis < endMillis {
		query := url.Values{}
		query.Set("symbol", asset.ProductID)
		query.Set("interval", interval)
		query.Set("startTime", strconv.FormatInt(startMillis, 10))
		query.Set("endTime", strconv.FormatInt(endMillis, 10))
		query.Set("limit", "1000")
		endpoint := fmt.Sprintf("%s/api/v3/klines?%s", base, query.Encode())
		req, err := http.NewRequestWithContext(ctx, http.MethodGet, endpoint, nil)
		if err != nil {
			return nil, err
		}
		req.Header.Set("User-Agent", "sqim-backtester/1.0")
		resp, err := client.Do(req)
		if err != nil {
			return nil, err
		}
		if resp.StatusCode != http.StatusOK {
			resp.Body.Close()
			return nil, fmt.Errorf("binance public klines returned %s for %s", resp.Status, asset.ProductID)
		}
		var candles [][]json.RawMessage
		err = json.NewDecoder(resp.Body).Decode(&candles)
		resp.Body.Close()
		if err != nil {
			return nil, err
		}
		if len(candles) == 0 {
			break
		}
		lastOpenMillis := startMillis
		for _, candle := range candles {
			if len(candle) < 6 {
				continue
			}
			openMillis, err := parseJSONInt(candle[0])
			if err != nil {
				return nil, fmt.Errorf("decode %s open time: %w", asset.ProductID, err)
			}
			if openMillis > lastOpenMillis {
				lastOpenMillis = openMillis
			}
			open, err := parseJSONFloat(candle[1])
			if err != nil {
				return nil, err
			}
			high, err := parseJSONFloat(candle[2])
			if err != nil {
				return nil, err
			}
			low, err := parseJSONFloat(candle[3])
			if err != nil {
				return nil, err
			}
			closePrice, err := parseJSONFloat(candle[4])
			if err != nil {
				return nil, err
			}
			volume, err := parseJSONFloat(candle[5])
			if err != nil {
				return nil, err
			}
			if openMillis < from.UnixMilli() || openMillis > to.UnixMilli() || low <= 0 || high <= 0 || open <= 0 || closePrice <= 0 {
				continue
			}
			timestamp := time.UnixMilli(openMillis).UTC().Unix()
			byTime[timestamp] = PriceRow{Timestamp: timestamp, Symbol: asset.Symbol, ProductID: asset.ProductID, Granularity: granularity, Low: low, High: high, Open: open, Close: closePrice, Volume: volume, Provider: "Binance Public Data"}
		}
		nextStart := lastOpenMillis + seconds*1000
		if nextStart <= startMillis {
			break
		}
		startMillis = nextStart
		select {
		case <-ctx.Done():
			return nil, ctx.Err()
		case <-time.After(50 * time.Millisecond):
		}
	}
	rows := make([]PriceRow, 0, len(byTime))
	for _, row := range byTime {
		rows = append(rows, row)
	}
	sort.Slice(rows, func(i, j int) bool { return rows[i].Timestamp < rows[j].Timestamp })
	return rows, nil
}

type Engine struct {
	Store    *Store
	Provider Binance
	Earliest time.Time
}

func (e *Engine) Sync(ctx context.Context, symbols []string, granularity string) ([]AssetHistory, error) {
	if granularity == "" {
		granularity = "daily"
	}
	from := e.Earliest
	if from.IsZero() {
		from = time.Date(2015, 1, 1, 0, 0, 0, 0, time.UTC)
	}
	// Daily simulations must never include today's still-open UTC candle.
	to := time.Now().UTC().Truncate(24 * time.Hour).Add(-time.Millisecond)
	out := make([]AssetHistory, 0, len(symbols))
	for _, raw := range symbols {
		symbol := strings.ToUpper(strings.TrimSpace(raw))
		asset, ok := Catalog[symbol]
		if !ok {
			out = append(out, AssetHistory{Asset: Asset{Symbol: symbol, Name: symbol}, Granularity: granularity, Available: false, Provider: "Binance Public Data", SourceURL: "https://data.binance.vision"})
			continue
		}
		rows, err := e.Provider.Fetch(ctx, asset, granularity, from, to)
		if err != nil {
			return nil, err
		}
		if err := e.Store.Write(symbol, granularity, rows); err != nil {
			return nil, err
		}
		out = append(out, historyFor(asset, granularity, rows))
	}
	return out, nil
}

func (e *Engine) Histories(granularity string) ([]AssetHistory, error) {
	if granularity == "" {
		granularity = "daily"
	}
	symbols := make([]string, 0, len(Catalog))
	for symbol := range Catalog {
		symbols = append(symbols, symbol)
	}
	sort.Strings(symbols)
	out := make([]AssetHistory, 0, len(symbols))
	for _, symbol := range symbols {
		rows, err := e.Store.Read(symbol, granularity)
		if err != nil {
			return nil, err
		}
		out = append(out, historyFor(Catalog[symbol], granularity, rows))
	}
	return out, nil
}

func (e *Engine) Run(request Request) (Result, error) {
	if err := validateRequest(request); err != nil {
		return Result{}, err
	}
	granularity := request.Granularity
	if granularity == "" {
		granularity = "daily"
	}
	from, to, err := parseWindow(request.From, request.To)
	if err != nil {
		return Result{}, err
	}
	seriesByAsset := make([]map[int64]float64, len(request.Assets))
	histories := make([]AssetHistory, len(request.Assets))
	youngest := ""
	var youngestFirst int64
	var overlapStart int64
	var overlapEnd int64 = math.MaxInt64
	for index, raw := range request.Assets {
		symbol := strings.ToUpper(strings.TrimSpace(raw))
		asset, ok := Catalog[symbol]
		if !ok {
			return Result{}, fmt.Errorf("historical data is unavailable for %s; no supported provider mapping exists", symbol)
		}
		rows, err := e.Store.Read(symbol, granularity)
		if err != nil {
			return Result{}, err
		}
		if len(rows) < 2 {
			return Result{}, fmt.Errorf("historical data is unavailable for %s; sync real provider candles first", symbol)
		}
		histories[index] = historyFor(asset, granularity, rows)
		if rows[0].Timestamp > youngestFirst {
			youngestFirst = rows[0].Timestamp
			youngest = symbol
		}
		prices := map[int64]float64{}
		for _, row := range rows {
			at := time.Unix(row.Timestamp, 0).UTC()
			if !at.Before(from) && !at.After(to) {
				prices[row.Timestamp] = row.Close
			}
		}
		if len(prices) < 2 {
			return Result{}, fmt.Errorf("%s has fewer than two real candles in the selected window", symbol)
		}
		first, last := bounds(prices)
		if first > overlapStart {
			overlapStart = first
		}
		if last < overlapEnd {
			overlapEnd = last
		}
		seriesByAsset[index] = prices
	}
	timestamps := commonTimestamps(seriesByAsset, overlapStart, overlapEnd)
	if len(timestamps) < 2 {
		return Result{}, errors.New("constituents do not have enough overlapping real candle dates")
	}
	points := make([]Point, 0, len(timestamps))
	returns := make([]float64, 0, len(timestamps)-1)
	value := 100.0
	points = append(points, Point{At: time.Unix(timestamps[0], 0).UTC(), Value: value})
	peak := value
	maxDrawdown := 0.0
	for day := 1; day < len(timestamps); day++ {
		factor := 0.0
		for assetIndex := range request.Assets {
			previous := seriesByAsset[assetIndex][timestamps[day-1]]
			current := seriesByAsset[assetIndex][timestamps[day]]
			factor += float64(request.WeightsBPS[assetIndex]) / 10_000 * current / previous
		}
		dailyReturn := factor - 1
		returns = append(returns, dailyReturn)
		value *= factor
		if value > peak {
			peak = value
		}
		drawdown := value/peak - 1
		if drawdown < maxDrawdown {
			maxDrawdown = drawdown
		}
		points = append(points, Point{At: time.Unix(timestamps[day], 0).UTC(), Value: value})
	}
	firstAt := points[0].At
	lastAt := points[len(points)-1].At
	years := lastAt.Sub(firstAt).Hours() / 24 / 365.2425
	annualizedReturn := 0.0
	if years > 0 {
		annualizedReturn = math.Pow(value/100, 1/years) - 1
	}
	return Result{
		Series: points, TotalReturn: value/100 - 1, AnnualizedVolatility: standardDeviation(returns) * math.Sqrt(365), MaxDrawdown: maxDrawdown, AnnualizedReturn: annualizedReturn,
		AvailableFrom: firstAt, AvailableTo: lastAt, AvailableYears: years, YoungestConstituent: youngest, Granularity: granularity,
		Methodology: "Daily target-weight rebalancing on dates shared by all constituents using Binance USDT-quoted spot close candles; normalized to 100; no fees, slippage, taxes, or liquidity constraints.",
		Provider:    "Binance Public Data", Disclaimer: Disclaimer, AssetHistory: histories,
	}, nil
}

func historyFor(asset Asset, granularity string, rows []PriceRow) AssetHistory {
	history := AssetHistory{Asset: asset, Granularity: granularity, Available: len(rows) > 0, Points: len(rows), Provider: "Binance Public Data", SourceURL: "https://data.binance.vision"}
	if len(rows) > 0 {
		history.First = time.Unix(rows[0].Timestamp, 0).UTC()
		history.Last = time.Unix(rows[len(rows)-1].Timestamp, 0).UTC()
	}
	return history
}

func granularitySeconds(granularity string) (int64, error) {
	switch granularity {
	case "daily":
		return 86400, nil
	case "minute":
		return 60, nil
	default:
		return 0, errors.New("granularity must be daily or minute")
	}
}

func parseJSONInt(value json.RawMessage) (int64, error) {
	return strconv.ParseInt(strings.Trim(string(value), `"`), 10, 64)
}

func parseJSONFloat(value json.RawMessage) (float64, error) {
	return strconv.ParseFloat(strings.Trim(string(value), `"`), 64)
}

func validateRequest(request Request) error {
	if len(request.Assets) < 1 || len(request.Assets) != len(request.WeightsBPS) {
		return errors.New("assets and weights_bps must be non-empty matching arrays")
	}
	total := 0
	for _, weight := range request.WeightsBPS {
		if weight < 0 || weight > 10_000 {
			return errors.New("each weight must be between 0 and 10000 basis points")
		}
		total += weight
	}
	if total != 10_000 {
		return errors.New("weights_bps must total 10000")
	}
	_, err := granularitySeconds(defaultString(request.Granularity, "daily"))
	return err
}

func parseWindow(rawFrom, rawTo string) (time.Time, time.Time, error) {
	from := time.Date(2015, 1, 1, 0, 0, 0, 0, time.UTC)
	to := time.Now().UTC()
	var err error
	if rawFrom != "" {
		from, err = time.Parse("2006-01-02", rawFrom)
		if err != nil {
			return time.Time{}, time.Time{}, errors.New("from must use YYYY-MM-DD")
		}
	}
	if rawTo != "" {
		to, err = time.Parse("2006-01-02", rawTo)
		if err != nil {
			return time.Time{}, time.Time{}, errors.New("to must use YYYY-MM-DD")
		}
		to = to.Add(24*time.Hour - time.Second)
	}
	if !from.Before(to) {
		return time.Time{}, time.Time{}, errors.New("from must be before to")
	}
	return from.UTC(), to.UTC(), nil
}

func bounds(values map[int64]float64) (int64, int64) {
	first := int64(math.MaxInt64)
	last := int64(0)
	for timestamp := range values {
		if timestamp < first {
			first = timestamp
		}
		if timestamp > last {
			last = timestamp
		}
	}
	return first, last
}

func commonTimestamps(series []map[int64]float64, from, to int64) []int64 {
	result := make([]int64, 0)
	for timestamp := range series[0] {
		if timestamp < from || timestamp > to {
			continue
		}
		present := true
		for index := 1; index < len(series); index++ {
			if _, ok := series[index][timestamp]; !ok {
				present = false
				break
			}
		}
		if present {
			result = append(result, timestamp)
		}
	}
	sort.Slice(result, func(i, j int) bool { return result[i] < result[j] })
	return result
}

func standardDeviation(values []float64) float64 {
	if len(values) < 2 {
		return 0
	}
	mean := 0.0
	for _, value := range values {
		mean += value
	}
	mean /= float64(len(values))
	variance := 0.0
	for _, value := range values {
		variance += math.Pow(value-mean, 2)
	}
	return math.Sqrt(variance / float64(len(values)-1))
}

func defaultString(value, fallback string) string {
	if value == "" {
		return fallback
	}
	return value
}
