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
GENERATED_SPIN_CONFIG    = config/spin-config.c config/spin-config.h
GENERATED_OUTBOUND_HTTP  = http/wasi-outbound-http.c http/wasi-outbound-http.h
GENERATED_SPIN_HTTP      = http/spin-http.c http/spin-http.h
GENERATED_OUTBOUND_REDIS = redis/outbound-redis.c redis/outbound-redis.h
GENERATED_SPIN_REDIS     = redis/spin-redis.c redis/spin-redis.h
GENERATED_KEY_VALUE      = key_value/key-value.c key_value/key-value.h

SDK_VERSION_SOURCE_FILE  = sdk_version/sdk-version-go-template.c

# NOTE: Please update this list if you add a new directory to the SDK:
SDK_VERSION_DEST_FILES   = config/sdk-version-go.c http/sdk-version-go.c \
			   key_value/sdk-version-go.c redis/sdk-version-go.c

.PHONY: generate
generate: $(GENERATED_OUTBOUND_HTTP) $(GENERATED_SPIN_HTTP)
generate: $(GENERATED_OUTBOUND_REDIS) $(GENERATED_SPIN_REDIS)
generate: $(GENERATED_SPIN_CONFIG) $(GENERATED_KEY_VALUE)
generate: $(SDK_VERSION_DEST_FILES)

$(SDK_VERSION_DEST_FILES): $(SDK_VERSION_SOURCE_FILE)
	export version="$$(cd ../rust && cargo run)"; \
	export commit="$$(git rev-parse HEAD)"; \
	sed -e "s/{{VERSION}}/$${version}/" -e "s/{{COMMIT}}/$${commit}/" < $< > $@

$(GENERATED_SPIN_CONFIG):
	wit-bindgen c --import ../../wit/ephemeral/spin-config.wit --out-dir ./config

$(GENERATED_OUTBOUND_HTTP):
	wit-bindgen c --import ../../wit/ephemeral/wasi-outbound-http.wit --out-dir ./http

$(GENERATED_SPIN_HTTP):
	wit-bindgen c --export ../../wit/ephemeral/spin-http.wit --out-dir ./http

$(GENERATED_OUTBOUND_REDIS):
	wit-bindgen c --import ../../wit/ephemeral/outbound-redis.wit --out-dir ./redis

$(GENERATED_SPIN_REDIS):
	wit-bindgen c --export ../../wit/ephemeral/spin-redis.wit --out-dir ./redis

$(GENERATED_KEY_VALUE):
	wit-bindgen c --import ../../wit/ephemeral/key-value.wit --out-dir ./key_value

# ----------------------------------------------------------------------
# Cleanup
# ----------------------------------------------------------------------
.PHONY: clean
clean:
	rm -f $(GENERATED_SPIN_CONFIG)
	rm -f $(GENERATED_OUTBOUND_HTTP) $(GENERATED_SPIN_HTTP)
	rm -f $(GENERATED_OUTBOUND_REDIS) $(GENERATED_SPIN_REDIS)
	rm -f $(GENERATED_KEY_VALUE) $(GENERATED_SDK_VERSION)
	rm -f http/testdata/http-tinygo/main.wasm
	rm -f $(EXAMPLES_DIR)/http-tinygo/main.wasm
	rm -f $(EXAMPLES_DIR)/http-tinygo-outbound-http/main.wasm
	rm -f $(EXAMPLES_DIR)/tinygo-outbound-redis/main.wasm
	rm -f $(EXAMPLES_DIR)/tinygo-redis/main.wasm
	rm -f $(EXAMPLES_DIR)/tinygo-key-value/main.wasm
	rm -f $(SDK_VERSION_DEST_FILES)
