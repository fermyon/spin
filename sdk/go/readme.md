# The (Tiny)Go SDK for Spin

This package contains an SDK that facilitates building Spin components in
(Tiny)Go. It currently allows building HTTP components that target the Wagi
executor.

```go
package main

import (
 "fmt"
 "net/http"

 // import this SDK
 spin_http "github.com/fermyon/spin-sdk"
)

func main() {
 // call the HandleRequest function
 spin_http.HandleRequest(func(w http.ResponseWriter, r *http.Request) {
  fmt.Fprintln(w, "Hello, Fermyon!")
 })
}
```
