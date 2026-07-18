package store

import "testing"

func TestFixtureContractIDPatternsCannotMasqueradeAsLiveContracts(t *testing.T) {
	fixtures := []string{"CBASKETTESTNET123", "CFACTORY", "CTEST-ONLY", "CFIXTURE01"}
	for _, contractID := range fixtures {
		if !IsFixtureContractID(contractID) {
			t.Fatalf("expected %q to be identified as a fixture contract ID", contractID)
		}
	}

	live := []string{
		"CLIVEBASKET",
		"CLIVESHARETOKEN",
	}
	for _, contractID := range live {
		if IsFixtureContractID(contractID) {
			t.Fatalf("live contract %q was incorrectly classified as a fixture", contractID)
		}
	}
}
