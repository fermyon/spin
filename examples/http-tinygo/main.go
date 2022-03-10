package main

import (
	"fmt"
	"net/http"

	spin_http "github.com/fermyon/spin-sdk"
)

func main() {
	spin_http.HandleRequest(func(w http.ResponseWriter, r *http.Request) {
		header := w.Header()
		header.Set("Content-Type", "text/plain; charset=utf-8")
		fmt.Fprintf(w, "\n")

		fmt.Fprintln(w, "Hello, world!")
	})
}
