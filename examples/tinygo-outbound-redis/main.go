package main

import (
	"net/http"
	"os"

	spin_http "github.com/fermyon/spin/sdk/go/http"
	"github.com/fermyon/spin/sdk/go/redis"
)

func init() {

	// handler for the http trigger
	spin_http.Handle(func(w http.ResponseWriter, r *http.Request) {

		// addr is the environment variable set in `spin.toml` that points to the
		// address of the Redis server.
		addr := os.Getenv("REDIS_ADDRESS")

		// channel is the environment variable set in `spin.toml` that specifies
		// the Redis channel that the component will publish to.
		channel := os.Getenv("REDIS_CHANNEL")

		// payload is the data publish to the redis channel.
		payload := []byte(`Hello redis from tinygo!`)

		if err := redis.Publish(addr, channel, payload); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		// set redis `mykey` = `myvalue`
		if err := redis.Set(addr, "mykey", []byte("myvalue")); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		// get redis payload for `mykey`
		if payload, err := redis.Get(addr, "mykey"); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		} else {
			w.Write([]byte("mykey value was: "))
			w.Write(payload)
		}
	})
}

func main() {}
