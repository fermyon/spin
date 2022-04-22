.PHONY: generate
generate:
	wit-bindgen c --import ../../../wit/ephemeral/wasi-outbound-http.wit
	wit-bindgen c --export ../../../wit/ephemeral/spin-http.wit

.PHONY: clean
clean:
	rm spin-http.h spin-http.c wasi-outbound-http.h wasi-outbound-http.c

.PHONY: test
test:
	tinygo test -wasm-abi=generic -target=wasi -gc=leaking -v
