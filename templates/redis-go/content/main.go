package main

import (
	"fmt"

	"github.com/fermyon/spin/sdk/go/v2/redis"
)

func init() {
	// redis.Handle() must be called in the init() function.
	redis.Handle(func(payload []byte) error {
		fmt.Println("Payload::::")
		fmt.Println(string(payload))
		return nil
	})
}

// main functiion must be included for the compiler but is not executed.
func main() {}
