package main

import (
	"fmt"
	"net/http"

	spinhttp "github.com/fermyon/spin/sdk/go/http"
)

func init() {
	spinhttp.Handle(func(w http.ResponseWriter, r *http.Request) {
		r1, err := spinhttp.Get("/hello")
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		fmt.Fprintln(w, r1.Body)
		fmt.Fprintln(w, r1.Header.Get("content-type"))

		// r2, err := spinhttp.Post("https://postman-echo.com/post", "text/plain", r.Body)
		// if err != nil {
		// 	http.Error(w, err.Error(), http.StatusInternalServerError)
		// 	return
		// }
		// fmt.Fprintln(w, r2.Body)

		// req, err := http.NewRequest("PUT", "https://postman-echo.com/put", bytes.NewBufferString("General Kenobi!"))
		// if err != nil {
		// 	http.Error(w, err.Error(), http.StatusInternalServerError)
		// 	return
		// }
		// req.Header.Add("foo", "bar")
		// r3, err := spinhttp.Send(req)
		// if err != nil {
		// 	http.Error(w, err.Error(), http.StatusInternalServerError)
		// 	return
		// }

		// fmt.Fprintln(w, r3.Body)

		// // `spin.toml` is not configured to allow outbound HTTP requests to this host,
		// // so this request will fail.
		// if _, err := spinhttp.Get("https://fermyon.com"); err != nil {
		// 	fmt.Fprintf(os.Stderr, "Cannot send HTTP request: %v", err)
		// }
	})
}

func main() {}
