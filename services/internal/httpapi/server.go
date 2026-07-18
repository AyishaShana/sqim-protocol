package httpapi

import (
	"bytes"
	"context"
	"crypto/ed25519"
	"crypto/rand"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/mail"
	"net/url"
	"os"
	"strconv"
	"strings"
	"time"

	"github.com/AyishaShana/sqim-protocol/services/internal/cache"
	"github.com/AyishaShana/sqim-protocol/services/internal/store"
	"github.com/jackc/pgx/v5"
	"github.com/stellar/go-stellar-sdk/strkey"
)

type MetricsCache interface {
	Metrics(ctx context.Context, basketID string) (cache.Metrics, error)
}

type Server struct {
	store         *store.Store
	cache         MetricsCache
	backtesterURL string
	httpClient    *http.Client
}

func New(store *store.Store, cache MetricsCache) *Server {
	backtesterURL := strings.TrimRight(strings.TrimSpace(os.Getenv("BACKTESTER_URL")), "/")
	if backtesterURL == "" {
		backtesterURL = "http://localhost:8090"
	}
	return &Server{store: store, cache: cache, backtesterURL: backtesterURL, httpClient: &http.Client{Timeout: 30 * time.Second}}
}

func (s *Server) WithBacktesterURL(backtesterURL string) *Server {
	s.backtesterURL = strings.TrimRight(backtesterURL, "/")
	return s
}

func (s *Server) Routes() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /health", s.health)
	mux.HandleFunc("GET /baskets", s.listBaskets)
	mux.HandleFunc("GET /baskets/{basketID}", s.getBasket)
	mux.HandleFunc("GET /baskets/{basketID}/history", s.history)
	mux.HandleFunc("GET /baskets/{basketID}/nav-history", s.navHistory)
	mux.HandleFunc("GET /baskets/{basketID}/metrics", s.metrics)
	mux.HandleFunc("GET /portfolio/{account}", s.portfolio)
	mux.HandleFunc("GET /profiles/{address}", s.profile)
	mux.HandleFunc("POST /profiles/{address}/challenge", s.profileChallenge)
	mux.HandleFunc("PUT /profiles/{address}", s.updateProfile)
	mux.HandleFunc("GET /backtesting/assets", s.backtestingAssets)
	mux.HandleFunc("POST /backtesting/run", s.runBacktest)
	return cors(mux)
}

type profileUpdateRequest struct {
	DisplayName           string `json:"display_name"`
	Bio                   string `json:"bio"`
	AvatarURL             string `json:"avatar_url"`
	NotificationFrequency string `json:"notification_frequency"`
	DriftThresholdBPS     int    `json:"drift_threshold_bps"`
	NotificationEmail     string `json:"notification_email"`
	Nonce                 string `json:"nonce"`
	Signature             string `json:"signature"`
}

func (s *Server) profile(w http.ResponseWriter, r *http.Request) {
	address := strings.TrimSpace(r.PathValue("address"))
	profile, err := s.store.Profile(r.Context(), address)
	if errors.Is(err, pgx.ErrNoRows) {
		writeJSON(w, http.StatusOK, store.UserProfile{
			Address: address, NotificationFrequency: "off", DriftThresholdBPS: 500,
		})
		return
	}
	if err != nil {
		writeError(w, err)
		return
	}
	writeJSON(w, http.StatusOK, profile)
}

func (s *Server) profileChallenge(w http.ResponseWriter, r *http.Request) {
	address := strings.TrimSpace(r.PathValue("address"))
	if _, err := decodeAccountAddress(address); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid Stellar account address"})
		return
	}
	random := make([]byte, 24)
	if _, err := rand.Read(random); err != nil {
		writeError(w, err)
		return
	}
	nonce := base64.RawURLEncoding.EncodeToString(random)
	expiresAt := time.Now().UTC().Add(5 * time.Minute)
	message := fmt.Sprintf("Sqim creator profile update\nAddress: %s\nNonce: %s\nExpires: %d", address, nonce, expiresAt.Unix())
	if err := s.store.CreateProfileChallenge(r.Context(), store.ProfileChallenge{
		Address: address, Nonce: nonce, Message: message, ExpiresAt: expiresAt,
	}); err != nil {
		writeError(w, err)
		return
	}
	writeJSON(w, http.StatusCreated, map[string]any{
		"nonce": nonce, "message": message, "expires_at": expiresAt,
	})
}

func (s *Server) updateProfile(w http.ResponseWriter, r *http.Request) {
	address := strings.TrimSpace(r.PathValue("address"))
	var request profileUpdateRequest
	if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 32<<10)).Decode(&request); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid profile payload"})
		return
	}
	if err := validateProfileUpdate(request); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": err.Error()})
		return
	}
	challenge, err := s.store.ProfileChallenge(r.Context(), address, request.Nonce)
	if errors.Is(err, pgx.ErrNoRows) {
		writeJSON(w, http.StatusUnauthorized, map[string]string{"error": "profile signature challenge is missing, expired, or already used"})
		return
	}
	if err != nil {
		writeError(w, err)
		return
	}
	if err := verifyProfileSignature(address, challenge.Message, request.Signature); err != nil {
		writeJSON(w, http.StatusUnauthorized, map[string]string{"error": err.Error()})
		return
	}
	if err := s.store.ConsumeProfileChallenge(r.Context(), address, request.Nonce); err != nil {
		writeJSON(w, http.StatusUnauthorized, map[string]string{"error": "profile signature challenge was already used"})
		return
	}
	profile, err := s.store.SaveProfile(r.Context(), store.UserProfile{
		Address: address, DisplayName: strings.TrimSpace(request.DisplayName), Bio: strings.TrimSpace(request.Bio),
		AvatarURL: strings.TrimSpace(request.AvatarURL), NotificationFrequency: request.NotificationFrequency,
		DriftThresholdBPS: request.DriftThresholdBPS, NotificationEmail: strings.TrimSpace(request.NotificationEmail),
	})
	if err != nil {
		writeError(w, err)
		return
	}
	writeJSON(w, http.StatusOK, profile)
}

func validateProfileUpdate(request profileUpdateRequest) error {
	if len(strings.TrimSpace(request.DisplayName)) > 64 || len(strings.TrimSpace(request.Bio)) > 320 {
		return errors.New("display name or bio is too long")
	}
	if request.DriftThresholdBPS < 0 || request.DriftThresholdBPS > 10_000 {
		return errors.New("drift threshold must be between 0 and 10000 basis points")
	}
	switch request.NotificationFrequency {
	case "off", "weekly", "on-drift-only":
	default:
		return errors.New("notification frequency must be off, weekly, or on-drift-only")
	}
	if avatar := strings.TrimSpace(request.AvatarURL); avatar != "" {
		parsed, err := url.ParseRequestURI(avatar)
		if err != nil || (parsed.Scheme != "https" && parsed.Scheme != "http") {
			return errors.New("avatar URL must be an http or https URL")
		}
	}
	if email := strings.TrimSpace(request.NotificationEmail); email != "" {
		if _, err := mail.ParseAddress(email); err != nil {
			return errors.New("notification email is invalid")
		}
	} else if request.NotificationFrequency != "off" {
		return errors.New("notification email is required when notifications are enabled")
	}
	if strings.TrimSpace(request.Nonce) == "" || strings.TrimSpace(request.Signature) == "" {
		return errors.New("nonce and wallet signature are required")
	}
	return nil
}

func verifyProfileSignature(address, message, encodedSignature string) error {
	publicKey, err := decodeAccountAddress(address)
	if err != nil {
		return errors.New("invalid Stellar account address")
	}
	signature, err := decodeSignature(encodedSignature)
	if err != nil || len(signature) != ed25519.SignatureSize {
		return errors.New("invalid wallet signature encoding")
	}
	if !ed25519.Verify(publicKey, []byte(message), signature) {
		return errors.New("wallet signature does not match the profile address")
	}
	return nil
}

func decodeAccountAddress(address string) (ed25519.PublicKey, error) {
	raw, err := strkey.Decode(strkey.VersionByteAccountID, address)
	if err != nil || len(raw) != ed25519.PublicKeySize {
		return nil, errors.New("invalid Stellar account address")
	}
	return ed25519.PublicKey(raw), nil
}

func decodeSignature(encoded string) ([]byte, error) {
	trimmed := strings.TrimSpace(encoded)
	if decoded, err := base64.StdEncoding.DecodeString(trimmed); err == nil {
		return decoded, nil
	}
	if decoded, err := base64.RawStdEncoding.DecodeString(trimmed); err == nil {
		return decoded, nil
	}
	return hex.DecodeString(trimmed)
}

func (s *Server) health(w http.ResponseWriter, r *http.Request) {
	if err := s.store.Ping(r.Context()); err != nil {
		writeJSON(w, http.StatusServiceUnavailable, map[string]string{
			"status": "error",
			"mode":   "live",
			"error":  "postgres unavailable",
		})
		return
	}
	writeJSON(w, http.StatusOK, map[string]string{"status": "ok", "mode": "live"})
}

func (s *Server) listBaskets(w http.ResponseWriter, r *http.Request) {
	baskets, err := s.store.ListBaskets(r.Context())
	if err != nil {
		writeError(w, err)
		return
	}
	if baskets == nil {
		baskets = []store.BasketConfig{}
	}
	writeJSON(w, http.StatusOK, baskets)
}

func (s *Server) getBasket(w http.ResponseWriter, r *http.Request) {
	basket, err := s.store.Basket(r.Context(), r.PathValue("basketID"))
	if errors.Is(err, pgx.ErrNoRows) {
		writeJSON(w, http.StatusNotFound, map[string]string{"error": "basket not found"})
		return
	}
	if err != nil {
		writeError(w, err)
		return
	}
	writeJSON(w, http.StatusOK, basket)
}

func (s *Server) history(w http.ResponseWriter, r *http.Request) {
	limit := 50
	if raw := strings.TrimSpace(r.URL.Query().Get("limit")); raw != "" {
		if parsed, err := strconv.Atoi(raw); err == nil && parsed > 0 && parsed <= 500 {
			limit = parsed
		}
	}
	history, err := s.store.History(r.Context(), r.PathValue("basketID"), limit)
	if err != nil {
		writeError(w, err)
		return
	}
	for index := range history {
		history[index].Amount = formatE7(history[index].Amount)
		history[index].Shares = formatE7(history[index].Shares)
		history[index].Fee = formatE7(history[index].Fee)
		history[index].NAV = formatE7(history[index].NAV)
		history[index].AUM = formatE7(history[index].AUM)
	}
	writeJSON(w, http.StatusOK, history)
}

func (s *Server) navHistory(w http.ResponseWriter, r *http.Request) {
	basketID := r.PathValue("basketID")
	deployedAt, err := s.store.DeploymentTime(r.Context(), basketID)
	if errors.Is(err, pgx.ErrNoRows) {
		writeJSON(w, http.StatusNotFound, map[string]string{"error": "basket not found"})
		return
	}
	if err != nil {
		writeError(w, err)
		return
	}
	points, err := s.store.NAVHistory(r.Context(), basketID)
	if err != nil {
		writeError(w, err)
		return
	}
	for index := range points {
		points[index].NAV = formatE7(points[index].NAV)
	}
	writeJSON(w, http.StatusOK, map[string]any{
		"basket_id": basketID, "deployed_at": deployedAt, "source": "Soroban events indexed by Sqim",
		"label": "Live on-chain basket NAV since deployment", "points": points,
	})
}

func (s *Server) backtestingAssets(w http.ResponseWriter, r *http.Request) {
	s.proxyBacktester(w, r, http.MethodGet, "/assets", nil)
}

func (s *Server) runBacktest(w http.ResponseWriter, r *http.Request) {
	body, err := io.ReadAll(http.MaxBytesReader(w, r.Body, 128<<10))
	if err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "backtest request is too large"})
		return
	}
	s.proxyBacktester(w, r, http.MethodPost, "/backtests", body)
}

func (s *Server) proxyBacktester(w http.ResponseWriter, r *http.Request, method, path string, body []byte) {
	request, err := http.NewRequestWithContext(r.Context(), method, s.backtesterURL+path, bytes.NewReader(body))
	if err != nil {
		writeError(w, err)
		return
	}
	if len(body) > 0 {
		request.Header.Set("content-type", "application/json")
	}
	response, err := s.httpClient.Do(request)
	if err != nil {
		writeJSON(w, http.StatusServiceUnavailable, map[string]string{"error": "historical simulation service unavailable"})
		return
	}
	defer response.Body.Close()
	w.Header().Set("content-type", "application/json")
	w.WriteHeader(response.StatusCode)
	_, _ = io.Copy(w, io.LimitReader(response.Body, 8<<20))
}

func (s *Server) metrics(w http.ResponseWriter, r *http.Request) {
	if s.cache == nil {
		writeJSON(w, http.StatusServiceUnavailable, map[string]string{"error": "metrics cache unavailable"})
		return
	}
	metrics, err := s.cache.Metrics(r.Context(), r.PathValue("basketID"))
	if err != nil {
		writeJSON(w, http.StatusNotFound, map[string]string{"error": "metrics cache miss"})
		return
	}
	metrics.NAV = formatE7(metrics.NAV)
	metrics.AUM = formatE7(metrics.AUM)
	writeJSON(w, http.StatusOK, metrics)
}

func (s *Server) portfolio(w http.ResponseWriter, r *http.Request) {
	holdings, err := s.store.Portfolio(r.Context(), r.PathValue("account"))
	if err != nil {
		writeError(w, err)
		return
	}
	if holdings == nil {
		holdings = []store.PortfolioHolding{}
	}
	for index := range holdings {
		holdings[index].Shares = formatE7(holdings[index].Shares)
	}
	writeJSON(w, http.StatusOK, holdings)
}

func formatE7(value string) string {
	value = strings.TrimSpace(value)
	if value == "" {
		return ""
	}
	sign := ""
	if strings.HasPrefix(value, "-") {
		sign = "-"
		value = strings.TrimPrefix(value, "-")
	}
	for _, digit := range value {
		if digit < '0' || digit > '9' {
			return sign + value
		}
	}
	value = strings.TrimLeft(value, "0")
	if value == "" {
		return "0"
	}
	if len(value) <= 7 {
		value = strings.Repeat("0", 8-len(value)) + value
	}
	whole := value[:len(value)-7]
	fraction := strings.TrimRight(value[len(value)-7:], "0")
	if fraction == "" {
		return sign + whole
	}
	return sign + whole + "." + fraction
}

func cors(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("access-control-allow-origin", "*")
		w.Header().Set("access-control-allow-methods", "GET, POST, PUT, OPTIONS")
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

func writeError(w http.ResponseWriter, err error) {
	writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
}
