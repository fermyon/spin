package main

import (
	"fmt"
	"net/http"

	spin_http "github.com/fermyon/spin-sdk"
)

func main() {
	spin_http.HandleRequest(func(w http.ResponseWriter, r *http.Request) {
		fmt.Fprintln(w, "Hello, Fermyon!")
	})
}
