package postgres

import (
	"fmt"
	"os"
)

type RowSet struct {
	//TODO
}

type ParameterValue struct {
	//TODO
}

// Run the specified Postgres command with the specified arguments, returning zero
// or more results.
func Execute(addr string, statement string, params []ParameterValue) (RowSet, error) {
	return execute(addr, statement, params)
}

/*
void outbound_pg_query(
	outbound_pg_string_t *address,
	outbound_pg_string_t *statement,
	outbound_pg_list_parameter_value_t *params,
	outbound_pg_expected_row_set_pg_error_t *ret0);
void outbound_pg_execute(
	outbound_pg_string_t *address,
	outbound_pg_string_t *statement,
	outbound_pg_list_parameter_value_t *params,
	outbound_pg_expected_u64_pg_error_t *ret0);
*/
