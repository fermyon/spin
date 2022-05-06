.PHONY: generate
generate:
	wit-bindgen c --import ../../../wit/ephemeral/spin-config.wit

.PHONY: clean
clean:
	rm spin-config.h spin-config.c

.PHONY: test
test:
	tinygo test -wasm-abi=generic -target=wasi -gc=leaking -v
