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
