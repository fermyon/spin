package main

import (
	"fmt"
	"net/http"

	spinhttp "github.com/fermyon/spin/sdk/go/http"
)

func init() {
	spinhttp.Handle(func(w http.ResponseWriter, r *http.Request) {
		transport := spinhttp.NewTransport()

		// or you can also do client := spinhttp.NewClient()
		client := &http.Client{
			Transport: transport,
		}

		resp, err := client.Get("https://example.com")
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.WriteHeader(resp.StatusCode)
		w.Header().Set("spin-path-info", r.Header.Get("spin-path-info"))
		w.Header().Set("foo", "bar")
		fmt.Fprintln(w, "Hello world!")
	})
}

func main() {}
