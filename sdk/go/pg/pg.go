package pg

import (
	"context"
	"database/sql"
	"database/sql/driver"
	"errors"
	"io"
	"reflect"
)

// globalValueConv a valueConv instance
var globalValueConv = &valueConv{}

// Open returns a new connection to the database.
func Open(address string) *sql.DB {
	return sql.OpenDB(&connector{address})
}

// connector implements driver.Connector.
type connector struct {
	address string
}

// Connect returns a connection to the database.
func (d *connector) Connect(_ context.Context) (driver.Conn, error) {
	return d.Open(d.address)
}

// Driver returns the underlying Driver of the Connector.
func (d *connector) Driver() driver.Driver {
	return d
}

// Open returns a new connection to the database.
func (d *connector) Open(address string) (driver.Conn, error) {
	return &conn{address: address}, nil
}

// conn implements driver.Conn
type conn struct {
	address string
}

var _ driver.Conn = (*conn)(nil)

// Prepare returns a prepared statement, bound to this connection.
func (c *conn) Prepare(query string) (driver.Stmt, error) {
	return &stmt{c: c, query: query}, nil
}

func (c *conn) Close() error {
	return nil
}

func (c *conn) Begin() (driver.Tx, error) {
	return nil, errors.New("transactions are unsupported by this driver")
}

type stmt struct {
	c     *conn
	query string
}

var _ driver.Stmt = (*stmt)(nil)
var _ driver.ColumnConverter = (*stmt)(nil)

// Close closes the statement.
func (s *stmt) Close() error {
	return nil
}

// NumInput returns the number of placeholder parameters.
func (s *stmt) NumInput() int {
	// Golang sql won't sanity check argument counts before Query.
	return -1
}

// Query executes a query that may return rows, such as a SELECT.
func (s *stmt) Query(args []driver.Value) (driver.Rows, error) {
	params := make([]any, len(args))
	for i := range args {
		params[i] = args[i]
	}
	return query(s.c.address, s.query, params)
}

// Exec executes a query that doesn't return rows, such as an INSERT or
// UPDATE.
func (s *stmt) Exec(args []driver.Value) (driver.Result, error) {
	params := make([]any, len(args))
	for i := range args {
		params[i] = args[i]
	}
	n, err := execute(s.c.address, s.query, params)
	return &result{rowsAffected: int64(n)}, err
}

// ColumnConverter return globalValueConv to don't use driver.DefaultParameterConverter
func (s *stmt) ColumnConverter(_ int) driver.ValueConverter {
	return globalValueConv
}

// valueConv a convertor not convert value
type valueConv struct{}

func (c *valueConv) ConvertValue(v any) (driver.Value, error) {
	return driver.Value(v), nil
}

type result struct {
	rowsAffected int64
}

func (r result) LastInsertId() (int64, error) {
	return -1, errors.New("LastInsertId is unsupported by this driver")
}

func (r result) RowsAffected() (int64, error) {
	return r.rowsAffected, nil
}

type rows struct {
	columns    []string
	columnType []uint8
	pos        int
	len        int
	rows       [][]any
	closed     bool
}

var _ driver.Rows = (*rows)(nil)
var _ driver.RowsColumnTypeScanType = (*rows)(nil)
var _ driver.RowsNextResultSet = (*rows)(nil)

// Columns return column names.
func (r *rows) Columns() []string {
	return r.columns
}

// Close closes the rows iterator.
func (r *rows) Close() error {
	r.rows = nil
	r.pos = 0
	r.len = 0
	r.closed = true
	return nil
}

// Next moves the cursor to the next row.
func (r *rows) Next(dest []driver.Value) error {
	if !r.HasNextResultSet() {
		return io.EOF
	}
	for i := 0; i != len(r.columns); i++ {
		dest[i] = driver.Value(r.rows[r.pos][i])
	}
	r.pos++
	return nil
}

// HasNextResultSet is called at the end of the current result set and
// reports whether there is another result set after the current one.
func (r *rows) HasNextResultSet() bool {
	return r.pos < r.len
}

// NextResultSet advances the driver to the next result set even
// if there are remaining rows in the current result set.
//
// NextResultSet should return io.EOF when there are no more result sets.
func (r *rows) NextResultSet() error {
	if r.HasNextResultSet() {
		r.pos++
		return nil
	}
	return io.EOF // Per interface spec.
}

// ColumnTypeScanType return the value type that can be used to scan types into.
func (r *rows) ColumnTypeScanType(index int) reflect.Type {
	return colTypeToReflectType(r.columnType[index])
}
