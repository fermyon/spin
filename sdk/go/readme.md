# The (Tiny)Go SDK for Spin

This package contains an SDK that facilitates building Spin components in
(Tiny)Go. It allows building HTTP components that target the Spin
executor.

```go
import (
	"fmt"
	spinhttp "github.com/fermyon/spin/sdk/go/v2/http"
)

func init() {
    // call the Handle function
    spinhttp.Handle(func(w http.ResponseWriter, r *http.Request) {
        fmt.Fprintln(w, "Hello, Fermyon!")
    })
}

func main() {}
```
