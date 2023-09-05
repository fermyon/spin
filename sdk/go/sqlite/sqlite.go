// Package sqlite provides access to database stores within Spin
// components.
package sqlite

import (
	"context"
	"database/sql"
	"database/sql/driver"
	"errors"
	"io"
)

// Open returns a new connection to the database.
func Open(name string) *sql.DB {
	return sql.OpenDB(&connector{name: name})
}

// Conn represents a database connection.
type Conn struct {
	_ptr uint32
}

// Close the connection.
func (db *Conn) Close() error {
	db.close()
	return nil
}

// Query executes a query that may return rows, such as a SELECT.
func (c *Conn) Query(query string, args []driver.Value) (driver.Rows, error) {
	params := make([]any, len(args))
	for i := range args {
		params[i] = args[i]
	}
	return c.execute(query, params)
}

// QueryContext executes a query that may return rows, such as a SELECT.
func (c *Conn) QueryContext(_ context.Context, query string, args []driver.Value) (driver.Rows, error) {
	return c.Query(query, args)
}

// Exec isn't implemented. Use Query method.
func (c *Conn) Exec(_ context.Context, _ string, _ []driver.NamedValue) (driver.Result, error) {
	return nil, errors.New("Exec method not implemented")
}

// ExecContext isn't implemented. Use Query method.
func (c *Conn) ExecContext(_ context.Context, _ string, _ []driver.NamedValue) (driver.Result, error) {
	return nil, errors.New("ExecContext method not implemented")
}

// Prepare isn't implemented.
func (c *Conn) Prepare(_ string) (driver.Stmt, error) {
	return nil, errors.New("Prepare method not implemented")
}

// Begin isn't implemented.
func (c *Conn) Begin() (driver.Tx, error) {
	return nil, errors.New("Begin method not implemented")
}

// connector implements driver.Connector.
type connector struct {
	name string
}

// Connect returns a connection to the database.
func (d *connector) Connect(_ context.Context) (driver.Conn, error) {
	return open(d.name)
}

// Driver returns the underlying Driver of the Connector.
func (d *connector) Driver() driver.Driver {
	return d
}

// Open returns a new connection to the database.
func (d *connector) Open(name string) (driver.Conn, error) {
	return open(name)
}

type results struct {
	columns []string
	pos     int
	len     int
	rows    [][]any
	closed  bool
}

// Columns return column names.
func (r *results) Columns() []string {
	return r.columns
}

// Close closes the rows iterator.
func (r *results) Close() error {
	r.rows = nil
	r.pos = 0
	r.len = 0
	r.closed = true
	return nil
}

// Next moves the cursor to the next row.
func (r *results) Next(dest []driver.Value) error {
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
func (r *results) HasNextResultSet() bool {
	return r.pos < r.len
}

// NextResultSet advances the driver to the next result set even
// if there are remaining rows in the current result set.
//
// NextResultSet should return io.EOF when there are no more result sets.
func (r *results) NextResultSet() error {
	if r.HasNextResultSet() {
		r.pos++
		return nil
	}
	return io.EOF // Per interface spec.
}
