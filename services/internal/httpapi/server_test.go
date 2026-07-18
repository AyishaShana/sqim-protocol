package httpapi

import (
	"context"
	"crypto/ed25519"
	"crypto/rand"
	"encoding/base64"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/AyishaShana/sqim-protocol/services/internal/cache"
	"github.com/stellar/go-stellar-sdk/strkey"
)

type fixedMetricsCache struct{ value cache.Metrics }

func (f fixedMetricsCache) Metrics(context.Context, string) (cache.Metrics, error) {
	return f.value, nil
}

func TestMetricsIncludesLedgerFreshness(t *testing.T) {
	asOf := time.Date(2026, 7, 18, 10, 0, 0, 0, time.UTC).Format(time.RFC3339)
	handler := New(nil, fixedMetricsCache{value: cache.Metrics{
		NAV: "10000000", AUM: "25000000", Ledger: 3609000, AsOf: asOf, Source: "indexed_soroban_event",
	}}).Routes()
	request := httptest.NewRequest(http.MethodGet, "/baskets/CBASKET/metrics", nil)
	request.SetPathValue("basketID", "CBASKET")
	recorder := httptest.NewRecorder()

	handler.ServeHTTP(recorder, request)
	if recorder.Code != http.StatusOK {
		t.Fatalf("expected metrics 200, got %d", recorder.Code)
	}
	body := recorder.Body.String()
	for _, expected := range []string{`"nav":"1"`, `"aum":"2.5"`, `"ledger":3609000`, `"as_of":"` + asOf + `"`} {
		if !strings.Contains(body, expected) {
			t.Fatalf("metrics response missing %s: %s", expected, body)
		}
	}
}

func TestFormatE7(t *testing.T) {
	tests := map[string]string{
		"":          "",
		"0":         "0",
		"1":         "0.0000001",
		"10700000":  "1.07",
		"100000000": "10",
		"-2500000":  "-0.25",
	}
	for input, expected := range tests {
		if actual := formatE7(input); actual != expected {
			t.Fatalf("formatE7(%q) = %q, want %q", input, actual, expected)
		}
	}
}

func TestBacktestProxyPreservesResultAndFailureStatus(t *testing.T) {
	upstream := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/backtests" || r.Method != http.MethodPost {
			t.Fatalf("unexpected upstream request: %s %s", r.Method, r.URL.Path)
		}
		body, _ := io.ReadAll(r.Body)
		if !strings.Contains(string(body), `"BTC"`) {
			t.Fatalf("proxy dropped request body: %s", body)
		}
		w.Header().Set("content-type", "application/json")
		w.WriteHeader(http.StatusUnprocessableEntity)
		_, _ = w.Write([]byte(`{"error":"insufficient overlapping history"}`))
	}))
	defer upstream.Close()

	server := httptest.NewServer(New(nil, nil).WithBacktesterURL(upstream.URL).Routes())
	defer server.Close()
	response, err := http.Post(server.URL+"/backtesting/run", "application/json", strings.NewReader(`{"assets":["BTC"]}`))
	if err != nil {
		t.Fatal(err)
	}
	defer response.Body.Close()
	if response.StatusCode != http.StatusUnprocessableEntity {
		t.Fatalf("expected upstream status 422, got %d", response.StatusCode)
	}
}

func TestVerifyProfileSignature(t *testing.T) {
	publicKey, privateKey, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		t.Fatal(err)
	}
	address, err := strkey.Encode(strkey.VersionByteAccountID, publicKey)
	if err != nil {
		t.Fatal(err)
	}
	message := "Sqim creator profile update"
	signature := base64.StdEncoding.EncodeToString(ed25519.Sign(privateKey, []byte(message)))
	if err := verifyProfileSignature(address, message, signature); err != nil {
		t.Fatalf("expected valid signature: %v", err)
	}
	if err := verifyProfileSignature(address, message+" changed", signature); err == nil {
		t.Fatal("expected changed message to fail signature verification")
	}
}

func TestValidateProfileUpdate(t *testing.T) {
	valid := profileUpdateRequest{
		DisplayName:           "Ayisha",
		Bio:                   "Basket creator",
		AvatarURL:             "https://example.com/avatar.png",
		NotificationFrequency: "on-drift-only",
		DriftThresholdBPS:     500,
		NotificationEmail:     "creator@example.com",
		Nonce:                 "nonce",
		Signature:             "signature",
	}
	if err := validateProfileUpdate(valid); err != nil {
		t.Fatalf("expected valid profile: %v", err)
	}
	valid.NotificationFrequency = "every-minute"
	if err := validateProfileUpdate(valid); err == nil {
		t.Fatal("expected invalid notification frequency to fail")
	}
	valid.NotificationFrequency = "weekly"
	valid.NotificationEmail = ""
	if err := validateProfileUpdate(valid); err == nil {
		t.Fatal("expected enabled email notifications without an email to fail")
	}
}
