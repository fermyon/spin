# ----------------------------------------------------------------------
# Test
# ----------------------------------------------------------------------
.PHONY: test
test: test-integration
	tinygo test -target=wasi -gc=leaking -v ./http
	tinygo test -target=wasi -gc=leaking -v ./redis

.PHONY: test-integration
test-integration: http/testdata/http-tinygo/main.wasm
	go test -v -count=1 .

http/testdata/http-tinygo/main.wasm: generate
http/testdata/http-tinygo/main.wasm: http/testdata/http-tinygo/main.go
	tinygo build -target=wasi -gc=leaking -no-debug -o http/testdata/http-tinygo/main.wasm http/testdata/http-tinygo/main.go

# ----------------------------------------------------------------------
# Build examples
# ----------------------------------------------------------------------
EXAMPLES_DIR = ../../examples

.PHONY: build-examples
build-examples: generate
build-examples: $(EXAMPLES_DIR)/config-tinygo/main.wasm
build-examples: $(EXAMPLES_DIR)/http-tinygo-outbound-http/main.wasm
build-examples: $(EXAMPLES_DIR)/http-tinygo/main.wasm
build-examples: $(EXAMPLES_DIR)/tinygo-outbound-redis/main.wasm
build-examples: $(EXAMPLES_DIR)/tinygo-redis/main.wasm
build-examples: $(EXAMPLES_DIR)/tinygo-key-value/main.wasm

$(EXAMPLES_DIR)/%/main.wasm: $(EXAMPLES_DIR)/%/main.go
	tinygo build -target=wasi -gc=leaking -no-debug -o $@ $<

# ----------------------------------------------------------------------
# Generate C bindings
# ----------------------------------------------------------------------
GENERATED_SDK		     = generated
GENERATED_SPIN_CONFIG    = config/spin-config.c config/spin-config.h
GENERATED_OUTBOUND_HTTP  = http/wasi-outbound-http.c http/wasi-outbound-http.h
GENERATED_SPIN_HTTP      = http/spin-http.c http/spin-http.h
GENERATED_OUTBOUND_REDIS = redis/outbound-redis.c redis/outbound-redis.h
GENERATED_SPIN_REDIS     = redis/spin-redis.c redis/spin-redis.h
GENERATED_KEY_VALUE      = key_value/key-value.c key_value/key-value.h

SDK_VERSION_SOURCE_FILE  = sdk_version/sdk-version-go-template.c

SDK_VERSION_DEST_FILE   = generated/sdk-version-go.c

.PHONY: generate
generate: $(GENERATED_SDK)
generate: $(SDK_VERSION_DEST_FILE)

$(GENERATED_SDK):
	wit-bindgen tiny-go ../../wit --world reactor --out-dir generated

$(SDK_VERSION_DEST_FILE): $(SDK_VERSION_SOURCE_FILE)
	export version="$$(cd ../rust && cargo run)"; \
	export commit="$$(git rev-parse HEAD)"; \
	sed -e "s/{{VERSION}}/$${version}/" -e "s/{{COMMIT}}/$${commit}/" < $< > $@

# ----------------------------------------------------------------------
# Cleanup
# ----------------------------------------------------------------------
.PHONY: clean
clean:
	rm -rf $(GENERATED_SDK)
	rm -f http/testdata/http-tinygo/main.wasm
	rm -f $(EXAMPLES_DIR)/http-tinygo/main.wasm
	rm -f $(EXAMPLES_DIR)/http-tinygo-outbound-http/main.wasm
	rm -f $(EXAMPLES_DIR)/tinygo-outbound-redis/main.wasm
	rm -f $(EXAMPLES_DIR)/tinygo-redis/main.wasm
	rm -f $(EXAMPLES_DIR)/tinygo-key-value/main.wasm
	rm -f $(SDK_VERSION_DEST_FILE)
