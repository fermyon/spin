package main

import (
	"fmt"
	"net/http"

	spinhttp "github.com/fermyon/spin/sdk/go/v2/http"
)

func init() {
	spinhttp.Handle(func(w http.ResponseWriter, r *http.Request) {
		// Because we included self in `allowed_http_hosts`, we can make outbound
		// HTTP requests to our own app using a relative path.
		resp, err := spinhttp.Get("/hello")
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		fmt.Fprintln(w, resp.Body)
		fmt.Fprintln(w, resp.Header.Get("content-type"))
	})
}

func main() {}
