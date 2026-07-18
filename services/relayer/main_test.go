package main

import (
	"reflect"
	"testing"

	"github.com/AyishaShana/sqim-protocol/services/internal/config"
)

func TestSignersArgIsValidJSON(t *testing.T) {
	got := signersArg([]string{"GA", "GB"})
	if got != `["GA","GB"]` {
		t.Fatalf("unexpected signer JSON: %s", got)
	}
}

func TestAuthHelperArgsIncludeQuorumSigners(t *testing.T) {
	cfg := config.Config{
		RelayerAuthScript: "sign-rebalance.mjs",
		RebalancerSigners: []string{"GA", "GB", "GC"},
		RebalancerQuorum:  2,
	}
	got := authHelperArgs(cfg, "CBASKET", []uint32{4500, 5500})
	want := []string{
		"sign-rebalance.mjs",
		"CBASKET",
		"[4500,5500]",
		`["GA","GB"]`,
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("unexpected auth-helper arguments: %#v", got)
	}
}
