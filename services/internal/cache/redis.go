package cache

import (
	"context"
	"errors"
	"strconv"
	"time"

	"github.com/redis/go-redis/v9"
)

type Cache struct {
	client *redis.Client
}

type Metrics struct {
	NAV    string `json:"nav"`
	AUM    string `json:"aum"`
	Ledger uint32 `json:"ledger"`
	AsOf   string `json:"as_of"`
	Source string `json:"source"`
}

func New(redisURL string) (*Cache, error) {
	options, err := redis.ParseURL(redisURL)
	if err != nil {
		return nil, err
	}
	return &Cache{client: redis.NewClient(options)}, nil
}

func (c *Cache) Close() error {
	return c.client.Close()
}

func (c *Cache) Ping(ctx context.Context) error {
	return c.client.Ping(ctx).Err()
}

func (c *Cache) Metrics(ctx context.Context, basketID string) (Metrics, error) {
	values, err := c.client.HGetAll(ctx, "basket:"+basketID+":metrics").Result()
	if err != nil {
		return Metrics{}, err
	}
	if len(values) == 0 {
		return Metrics{}, errors.New("metrics cache miss")
	}
	ledger, err := strconv.ParseUint(values["ledger"], 10, 32)
	if err != nil {
		return Metrics{}, errors.New("metrics cache has invalid ledger metadata")
	}
	return Metrics{
		NAV: values["nav"], AUM: values["aum"], Ledger: uint32(ledger),
		AsOf: values["as_of"], Source: values["source"],
	}, nil
}

func (c *Cache) SetMetrics(ctx context.Context, basketID string, metrics Metrics, ttl time.Duration) error {
	key := "basket:" + basketID + ":metrics"
	if err := c.client.HSet(
		ctx,
		key,
		"nav", metrics.NAV,
		"aum", metrics.AUM,
		"ledger", metrics.Ledger,
		"as_of", metrics.AsOf,
		"source", metrics.Source,
	).Err(); err != nil {
		return err
	}
	return c.client.Expire(ctx, key, ttl).Err()
}
