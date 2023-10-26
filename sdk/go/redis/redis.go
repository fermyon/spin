// Package redis provides the handler function for the Redis trigger, as well
// as access to Redis within Spin components.
package redis

import (
	"errors"
	"fmt"
	"os"
)

// handler is the function that will be called by the Redis trigger in Spin.
var handler = defaultHandler

// defaultHandler is a placeholder for returning a useful error to stdout when
// the handler is not set.
var defaultHandler = func(payload []byte) error {
	fmt.Fprintln(os.Stderr, "redis handler undefined")
	return nil
}

// Handle sets the handler function for redis.
// It must be set in an init() function.
func Handle(fn func(payload []byte) error) {
	handler = fn
}

// Client is a Redis client.
type Client struct {
	addr string
}

// NewClient returns a Redis client.
func NewClient(address string) *Client {
	return &Client{addr: address}
}

// Publish a Redis message to the specified channel.
func (c *Client) Publish(channel string, payload []byte) error {
	if len(payload) == 0 {
		return errors.New("payload is empty")
	}
	return publish(c.addr, channel, payload)
}

// Get the value of a key. An error is returned if the value stored at key is
// not a string.
func (c *Client) Get(key string) ([]byte, error) {
	return get(c.addr, key)
}

// Set key to value. If key alreads holds a value, it is overwritten.
func (c *Client) Set(key string, payload []byte) error {
	if len(payload) == 0 {
		return errors.New("payload is empty")
	}
	return set(c.addr, key, payload)
}

// Incr increments the number stored at key by one. If the key does not exist,
// it is set to 0 before performing the operation. An error is returned if
// the key contains a value of the wrong type or contains a string that can not
// be represented as integer.
func (c *Client) Incr(key string) (int64, error) {
	return incr(c.addr, key)
}

// Del removes the specified keys. A key is ignored if it does not exist.
func (c *Client) Del(keys ...string) (int64, error) {
	return del(c.addr, keys)
}

// Sadd adds the specified values to the set for the specified key, creating
// it if it does not already exist.
func (c *Client) Sadd(key string, values ...string) (int64, error) {
	return sadd(c.addr, key, values)
}

// Smembers gets the elements of the set for the specified key.
func (c *Client) Smembers(key string) ([]string, error) {
	return smembers(c.addr, key)
}

// Srem removes the specified elements from the set for the specified key.
// This has no effect if the key does not exist.
func (c *Client) Srem(key string, values ...string) (int64, error) {
	return srem(c.addr, key, values)
}

// ResultKind represents a result type returned from executing a Redis command.
type ResultKind uint8

const (
	ResultKindNil ResultKind = iota
	ResultKindStatus
	ResultKindInt64
	ResultKindBinary
)

// String implements fmt.Stringer.
func (r ResultKind) String() string {
	switch r {
	case ResultKindNil:
		return "nil"
	case ResultKindStatus:
		return "status"
	case ResultKindInt64:
		return "int64"
	case ResultKindBinary:
		return "binary"
	default:
		return "unknown"
	}
}

// GoString implements fmt.GoStringer.
func (r ResultKind) GoString() string { return r.String() }

// Result represents a value returned from a Redis command.
type Result struct {
	Kind ResultKind
	Val  any
}

// Execute runs the specified Redis command with the specified arguments,
// returning zero or more results.  This is a general-purpose function which
// should work with any Redis command.
//
// Arguments must be string, []byte, int, int64, or int32.
func (c *Client) Execute(command string, arguments ...any) ([]*Result, error) {
	var params []*argument
	for _, a := range arguments {
		p, err := createParameter(a)
		if err != nil {
			return nil, err
		}
		params = append(params, p)
	}
	return execute(c.addr, command, params)
}
