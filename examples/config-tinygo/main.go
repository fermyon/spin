package main

import (
	"fmt"
	"net/http"

	"github.com/fermyon/spin/sdk/go/config"
	spinhttp "github.com/fermyon/spin/sdk/go/http"
)

func init() {
	spinhttp.Handle(func(w http.ResponseWriter, r *http.Request) {

		// Get config value `message` defined in spin.toml.
		val, err := config.Get("message")
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		fmt.Fprintln(w, "message: ", val)
	})
}

func main() {}
