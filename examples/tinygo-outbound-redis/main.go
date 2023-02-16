package main

import (
	"net/http"
	"os"
	"strconv"
	"reflect"
	"fmt"

	"golang.org/x/exp/slices"
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
			return
		} else {
			w.Write([]byte("mykey value was: "))
			w.Write(payload)
			w.Write([]byte("\n"))
		}

		// incr `spin-go-incr` by 1
		if payload, err := redis.Incr(addr, "spin-go-incr"); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		} else {
			w.Write([]byte("spin-go-incr value: "))
			w.Write([]byte(strconv.FormatInt(payload, 10)))
			w.Write([]byte("\n"))
		}

		// delete `spin-go-incr` and `mykey`
		if payload, err := redis.Del(addr, []string{"spin-go-incr", "mykey", "non-existing-key"}); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		} else {
			w.Write([]byte("deleted keys num: "))
			w.Write([]byte(strconv.FormatInt(payload, 10)))
			w.Write([]byte("\n"))
		}

		if _, err := redis.Sadd(addr, "myset", []string{"foo", "bar"}); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		{
			expected := []string{"bar", "foo"}
			payload, err := redis.Smembers(addr, "myset")
			if err != nil {
				http.Error(w, err.Error(), http.StatusInternalServerError)
				return
			}
			slices.Sort(payload)
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

		if _, err := redis.Srem(addr, "myset", []string{"bar"}); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		{
			expected := []string{"foo"}
			if payload, err := redis.Smembers(addr, "myset"); err != nil {
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

		message := redis.RedisParameter{Kind: redis.RedisParameterKindBinary, Val: []byte("message")}
		hello := redis.RedisParameter{Kind: redis.RedisParameterKindBinary, Val: []byte("hello")}
		if _, err := redis.Execute(addr, "set", []redis.RedisParameter{message, hello}); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		world := redis.RedisParameter{Kind: redis.RedisParameterKindBinary, Val: []byte(" world")}
		if _, err := redis.Execute(addr, "append", []redis.RedisParameter{message, world}); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		if payload, err := redis.Execute(addr, "get", []redis.RedisParameter{message}); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		} else if !reflect.DeepEqual(
			payload,
			[]redis.RedisResult{redis.RedisResult{
				Kind: redis.RedisResultKindBinary,
				Val: []byte("hello world"),
			}}) {
			http.Error(w, "unexpected GET result", http.StatusInternalServerError)
			return
		}
	})
}

func main() {}
