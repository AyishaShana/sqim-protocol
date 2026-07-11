package httpapi

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"strconv"
	"strings"

	"github.com/AyishaShana/sqim-protocol/services/internal/cache"
	"github.com/AyishaShana/sqim-protocol/services/internal/store"
	"github.com/jackc/pgx/v5"
)

type MetricsCache interface {
	Metrics(ctx context.Context, basketID string) (cache.Metrics, error)
}

type Server struct {
	store *store.Store
	cache MetricsCache
}

func New(store *store.Store, cache MetricsCache) *Server {
	return &Server{store: store, cache: cache}
}

func (s *Server) Routes() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /health", s.health)
	mux.HandleFunc("GET /baskets", s.listBaskets)
	mux.HandleFunc("GET /baskets/{basketID}", s.getBasket)
	mux.HandleFunc("GET /baskets/{basketID}/history", s.history)
	mux.HandleFunc("GET /baskets/{basketID}/metrics", s.metrics)
	return cors(mux)
}

func (s *Server) health(w http.ResponseWriter, _ *http.Request) {
	writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
}

func (s *Server) listBaskets(w http.ResponseWriter, r *http.Request) {
	baskets, err := s.store.ListBaskets(r.Context())
	if err != nil {
		writeError(w, err)
		return
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
	writeJSON(w, http.StatusOK, history)
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
	writeJSON(w, http.StatusOK, metrics)
}

func cors(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("access-control-allow-origin", "*")
		w.Header().Set("access-control-allow-methods", "GET, OPTIONS")
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
