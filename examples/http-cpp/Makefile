# curl -OL https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-14/wasi-sdk-14.0-linux.tar.gz
# tar xf wasi-sdk-14.0-linux.tar.gz
# sudo mv wasi-sdk-14.0 /opt/wasi-sdk
WASI_CC       ?= /opt/wasi-sdk/bin/clang

# cargo install --git https://github.com/bytecodealliance/wit-bindgen --rev dde4694aaa6acf9370206527a798ac4ba6a8c5b8 wit-bindgen-cli
WIT_BINDGEN   ?= wit-bindgen

lib.wasm: lib.o spin-http.o
	$(WASI_CC) -Wall -mexec-model=reactor -o $@ $^

bindings/spin-http.c bindings/spin-http.h: ../../wit/ephemeral/spin-http.wit
	$(WIT_BINDGEN) c --export $< --out-dir bindings

spin-http.o: bindings/spin-http.c bindings/spin-http.h
	$(WASI_CC) -Wall -I . -I bindings -c -o $@ $<

lib.o: lib.cpp bindings/spin-http.h
	$(WASI_CC) -Wall -I . -I bindings -c -o $@ $<

.PHONY: clean
clean:
	rm -rf bindings *.o *.wasm
