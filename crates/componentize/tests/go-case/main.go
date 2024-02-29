package main

import (
	"fmt"
	"net/http"
	"errors"
	"strings"
	"io"
	"os"

	spinredis "github.com/fermyon/spin/sdk/go/redis"
	spinhttp "github.com/fermyon/spin/sdk/go/http"
	spinconfig "github.com/fermyon/spin/sdk/go/config"
)

func init() {
	spinhttp.Handle(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			w.WriteHeader(405)
		} else if r.URL.Path == "/" {
			bytes, err := io.ReadAll(r.Body)
			if err == nil {
				dispatch(w, strings.Split(string(bytes), "%20"))
			} else {
				w.WriteHeader(500)
				fmt.Fprint(w, err)
			}
		} else if r.URL.Path != "/foo" {
			w.WriteHeader(404)
		} else if len(r.Header) != 1 || r.Header["Foo"][0] != "bar" {
			w.WriteHeader(400)
		} else {
			w.WriteHeader(200)
			w.Header().Set("lorem", "ipsum")
			fmt.Fprint(w, "dolor sit amet")
		}
	})

	spinredis.Handle(func(payload []byte) error {
		return nil
	})
}

func dispatch(w http.ResponseWriter, v []string) {
	err := execute(v)
	if err == nil {
		w.WriteHeader(200)
	} else {
		w.WriteHeader(500)
		fmt.Fprint(w, err)
	}
}

func execute(v []string) error {
	switch v[0] {
	case "config":
		spinconfig.Get(v[1])
	case "http":
		spinhttp.Get(v[1])
	case "wasi-env":
		fmt.Print(os.Getenv(v[1]))
	default:
		return errors.New(fmt.Sprintf("command not yet supported: %s", v[0]))
	}

	return nil
}

func main() {}
