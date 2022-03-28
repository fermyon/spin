package main

import (
	"fmt"
	"net/http"

	spin "github.com/fermyon/spin/sdk/go/http"
)

func main() {
	spin.HandleRequest(func(w http.ResponseWriter, r *http.Request) {
		fmt.Fprintln(w, "Hello, Fermyon!")
	})
}
