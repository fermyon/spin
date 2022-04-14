package redis

import (
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

// Publish a Redis message to the specificed channel and return an error, if any.
func Publish(addr, channel string, payload []byte) error {
	return publish(addr, channel, payload)
}

// Get the value of a key. An error is returned if the value stored at key is
// not a string.
func Get(addr, key string) ([]byte, error) {
	return get(addr, key)
}

// Set key to value. If key alreads holds a value, it is overwritten.
func Set(addr, key string, payload []byte) error {
	return set(addr, key, payload)
}
