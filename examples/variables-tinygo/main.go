package main

import (
	"fmt"
	"net/http"

	spinhttp "github.com/fermyon/spin/sdk/go/v2/http"
	"github.com/fermyon/spin/sdk/go/v2/variables"
)

func init() {
	spinhttp.Handle(func(w http.ResponseWriter, r *http.Request) {

		// Get variable value `message` defined in spin.toml.
		val, err := variables.Get("message")
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		fmt.Fprintln(w, "message: ", val)
	})
}

func main() {}
