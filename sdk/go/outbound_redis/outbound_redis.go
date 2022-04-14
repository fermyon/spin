// Package outbound_redis contains the helper functions for interacting with
// Redis in Spin components using TinyGo.
package outbound_redis

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
