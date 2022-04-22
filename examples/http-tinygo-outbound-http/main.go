package main

import (
	"bytes"
	"fmt"
	"net/http"
	"os"

	spinhttp "github.com/fermyon/spin/sdk/go/http"
)

func init() {
	spinhttp.Handle(func(w http.ResponseWriter, r *http.Request) {
		r1, _ := spinhttp.Get("https://some-random-api.ml/facts/dog")

		fmt.Fprintln(w, r1.Body)
		fmt.Fprintln(w, r1.Header.Get("content-type"))

		r2, _ := spinhttp.Post("https://postman-echo.com/post", "text/plain", r.Body)
		fmt.Fprintln(w, r2.Body)

		req, _ := http.NewRequest("PUT", "https://postman-echo.com/put", bytes.NewBufferString("General Kenobi!"))
		req.Header.Add("foo", "bar")
		r3, _ := spinhttp.Send(req)

		fmt.Fprintln(w, r3.Body)

		// `spin.toml` is not configured to allow outbound HTTP requests to this host,
		// so this request will fail.
		if _, err := spinhttp.Get("https://fermyon.com"); err != nil {
			fmt.Fprintf(os.Stderr, "Cannot send HTTP request: %v", err)
		}
	})
}

func main() {}
