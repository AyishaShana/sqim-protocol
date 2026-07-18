package store

import (
	"context"
	"fmt"
	"strings"

	"github.com/jackc/pgx/v5"
)

var fixtureContractPrefixes = []string{"CBASKET", "CFACTORY", "CTEST", "CFIXTURE"}

func IsFixtureContractID(contractID string) bool {
	upper := strings.ToUpper(strings.TrimSpace(contractID))
	for _, prefix := range fixtureContractPrefixes {
		if strings.HasPrefix(upper, prefix) {
			return true
		}
	}
	return false
}

// AssertNoFixtureIDs prevents integration-test records from shadowing live factory data.
// Test processes may opt out explicitly with SQIM_ALLOW_TEST_FIXTURES=true.
func (s *Store) AssertNoFixtureIDs(ctx context.Context) error {
	var contractID string
	err := s.pool.QueryRow(ctx, `
		select contract_id
		from (
			select basket_id as contract_id from basket_configs
			union all
			select share_token_id as contract_id from basket_configs where share_token_id <> ''
			union all
			select basket_id as contract_id from deposit_withdraw_events
		) ids
		where upper(contract_id) like 'CBASKET%'
		   or upper(contract_id) like 'CFACTORY%'
		   or upper(contract_id) like 'CTEST%'
		   or upper(contract_id) like 'CFIXTURE%'
		limit 1
	`).Scan(&contractID)
	if err == pgx.ErrNoRows {
		return nil
	}
	if err != nil {
		return err
	}
	return fmt.Errorf("fixture contract ID %q found; clean the database or set SQIM_ALLOW_TEST_FIXTURES=true only for an isolated test database", contractID)
}
