package main

import (
	"fmt"
	"net/http"
	"os"
	"reflect"
	"sort"
	"strconv"

	spin_http "github.com/fermyon/spin/sdk/go/v2/http"
	"github.com/fermyon/spin/sdk/go/v2/redis"
)

func init() {

	// handler for the http trigger
	spin_http.Handle(func(w http.ResponseWriter, _ *http.Request) {

		// addr is the environment variable set in `spin.toml` that points to the
		// address of the Redis server.
		addr := os.Getenv("REDIS_ADDRESS")

		// channel is the environment variable set in `spin.toml` that specifies
		// the Redis channel that the component will publish to.
		channel := os.Getenv("REDIS_CHANNEL")

		// payload is the data publish to the redis channel.
		payload := []byte(`Hello redis from tinygo!`)

		rdb := redis.NewClient(addr)

		if err := rdb.Publish(channel, payload); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		// set redis `mykey` = `myvalue`
		if err := rdb.Set("mykey", []byte("myvalue")); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		// get redis payload for `mykey`
		if payload, err := rdb.Get("mykey"); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		} else {
			w.Write([]byte("mykey value was: "))
			w.Write(payload)
			w.Write([]byte("\n"))
		}

		// incr `spin-go-incr` by 1
		if payload, err := rdb.Incr("spin-go-incr"); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		} else {
			w.Write([]byte("spin-go-incr value: "))
			w.Write([]byte(strconv.FormatInt(payload, 10)))
			w.Write([]byte("\n"))
		}

		// delete `spin-go-incr` and `mykey`
		if payload, err := rdb.Del("spin-go-incr", "mykey", "non-existing-key"); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		} else {
			w.Write([]byte("deleted keys num: "))
			w.Write([]byte(strconv.FormatInt(payload, 10)))
			w.Write([]byte("\n"))
		}

		if _, err := rdb.Sadd("myset", "foo", "bar"); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		{
			expected := []string{"bar", "foo"}
			payload, err := rdb.Smembers("myset")
			if err != nil {
				http.Error(w, err.Error(), http.StatusInternalServerError)
				return
			}
			sort.Strings(payload)
			if !reflect.DeepEqual(payload, expected) {
				http.Error(
					w,
					fmt.Sprintf(
						"unexpected SMEMBERS result: expected %v, got %v",
						expected,
						payload,
					),
					http.StatusInternalServerError,
				)
				return
			}
		}

		if _, err := rdb.Srem("myset", "bar"); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		{
			expected := []string{"foo"}
			if payload, err := rdb.Smembers("myset"); err != nil {
				http.Error(w, err.Error(), http.StatusInternalServerError)
				return
			} else if !reflect.DeepEqual(payload, expected) {
				http.Error(
					w,
					fmt.Sprintf(
						"unexpected SMEMBERS result: expected %v, got %v",
						expected,
						payload,
					),
					http.StatusInternalServerError,
				)
				return
			}
		}

		if _, err := rdb.Execute("set", "message", "hello"); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		if _, err := rdb.Execute("append", "message", " world"); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		if payload, err := rdb.Execute("get", "message"); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		} else if !reflect.DeepEqual(
			payload,
			[]*redis.Result{{
				Kind: redis.ResultKindBinary,
				Val:  []byte("hello world"),
			}}) {
			http.Error(w, "unexpected GET result", http.StatusInternalServerError)
			return
		}
	})
}

func main() {}
