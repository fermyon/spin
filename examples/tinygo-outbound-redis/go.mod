module github.com/fermyon/spin/templates/spin-http-tinygo-outbound-http

go 1.17

require github.com/fermyon/spin/sdk/go v0.0.0

require (
	github.com/julienschmidt/httprouter v1.3.0 // indirect
	golang.org/x/exp v0.0.0-20230213192124-5e25df0256eb
)

replace github.com/fermyon/spin/sdk/go v0.0.0 => ../../sdk/go/
