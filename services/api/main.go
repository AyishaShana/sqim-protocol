package main

import (
	"context"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/AyishaShana/sqim-protocol/services/internal/cache"
	"github.com/AyishaShana/sqim-protocol/services/internal/config"
	"github.com/AyishaShana/sqim-protocol/services/internal/httpapi"
	"github.com/AyishaShana/sqim-protocol/services/internal/store"
)

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

	var redisCache *cache.Cache
	if c, err := cache.New(cfg.RedisURL); err == nil && c.Ping(ctx) == nil {
		redisCache = c
		defer redisCache.Close()
	} else {
		log.Printf("redis unavailable, metrics endpoint will return cache misses")
	}

	srv := &http.Server{
		Addr:              cfg.APIAddr,
		Handler:           httpapi.New(db, redisCache).Routes(),
		ReadHeaderTimeout: 5 * time.Second,
	}
	go func() {
		<-ctx.Done()
		shutdownCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		_ = srv.Shutdown(shutdownCtx)
	}()

	log.Printf("api listening on %s", cfg.APIAddr)
	if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		log.Fatalf("api failed: %v", err)
	}
}
