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

// conn represents a database connection.
type conn struct {
	_ptr uint32
}

// Close the connection.
func (c *conn) Close() error {
	c.close()
	return nil
}

// Prepare returns a prepared statement, bound to this connection.
func (c *conn) Prepare(query string) (driver.Stmt, error) {
	return &stmt{c: c, query: query}, nil
}

// Begin isn't supported.
func (c *conn) Begin() (driver.Tx, error) {
	return nil, errors.New("transactions are unsupported by this driver")
}

// connector implements driver.Connector.
type connector struct {
	conn *conn
	name string
}

// Connect returns a connection to the database.
func (d *connector) Connect(_ context.Context) (driver.Conn, error) {
	if d.conn != nil {
		return d.conn, nil
	}

	return d.Open(d.name)
}

// Driver returns the underlying Driver of the Connector.
func (d *connector) Driver() driver.Driver {
	return d
}

// Open returns a new connection to the database.
func (d *connector) Open(name string) (driver.Conn, error) {
	conn, err := open(name)
	if err != nil {
		return nil, err
	}

	d.conn = conn

	return conn, err
}

// Close closes the connection to the database.
func (d *connector) Close() error {
	if d.conn != nil {
		d.conn.Close()
	}

	return nil
}

type rows struct {
	columns []string
	pos     int
	len     int
	rows    [][]any
	closed  bool
}

var _ driver.Rows = (*rows)(nil)

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

type stmt struct {
	c     *conn
	query string
}

var _ driver.Stmt = (*stmt)(nil)

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
	return s.c.execute(s.query, params)
}

// Exec executes a query that doesn't return rows, such as an INSERT or
// UPDATE.
func (s *stmt) Exec(args []driver.Value) (driver.Result, error) {
	params := make([]any, len(args))
	for i := range args {
		params[i] = args[i]
	}
	_, err := s.c.execute(s.query, params)
	return &result{}, err
}

type result struct{}

func (r result) LastInsertId() (int64, error) {
	return -1, errors.New("LastInsertId is unsupported by this driver")
}

func (r result) RowsAffected() (int64, error) {
	return -1, errors.New("RowsAffected is unsupported by this driver")
}
