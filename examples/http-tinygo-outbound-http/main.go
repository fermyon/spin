package main

import (
	"bytes"
	"fmt"
	"net/http"
	"os"

	spin_http "github.com/fermyon/spin-sdk/http"
)

func main() {
	spin_http.HandleRequest(func(w http.ResponseWriter, r *http.Request) {
		r1, _ := spin_http.Get("https://some-random-api.ml/facts/dog")

		fmt.Fprintln(w, r1.Body)
		fmt.Fprintln(w, r1.Header.Get("content-type"))

		r2, _ := spin_http.Post("https://postman-echo.com/post", "text/plain", r.Body)
		fmt.Fprintln(w, r2.Body)

		req, _ := http.NewRequest("PUT", "https://postman-echo.com/put", bytes.NewBufferString("General Kenobi!"))
		req.Header.Add("foo", "bar")
		r3, _ := spin_http.Send(req)

		fmt.Fprintln(w, r3.Body)

		// `spin.toml` is not configured to allow outbound HTTP requests to this host,
		// so this request will fail.
		_, err := spin_http.Get("https://fermyon.com")
		if err != nil {
			fmt.Fprintf(os.Stderr, "Cannot send HTTP request: %v", err)
		}
	})
}
