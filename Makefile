LOG_LEVEL ?= spin=trace
CERT_NAME ?= local
SPIN_DOC_NAME ?= new-doc.md
export PATH := target/debug:target/release:$(HOME)/.cargo/bin:$(PATH)

ARCH = $(shell uname -p)

## dependencies for e2e-tests
E2E_BUILD_SPIN                  ?= false
E2E_FETCH_SPIN                  ?= true
E2E_TESTS_DOCKERFILE            ?= e2e-tests.Dockerfile
MYSQL_IMAGE                     ?= mysql:8.0.22
REDIS_IMAGE                     ?= redis:7.0.8-alpine3.17
POSTGRES_IMAGE                  ?= postgres:14.7-alpine
REGISTRY_IMAGE                  ?= registry:2
E2E_SPIN_RELEASE_VOLUME_MOUNT   ?=
E2E_SPIN_DEBUG_VOLUME_MOUNT     ?=

## overrides for aarch64
ifneq ($(ARCH),x86_64)
	MYSQL_IMAGE             = arm64v8/mysql:8.0.32
	REDIS_IMAGE             = arm64v8/redis:6.0-alpine3.17
	POSTGRES_IMAGE          = arm64v8/postgres:14.7
	REGISTRY_IMAGE          = arm64v8/registry:2
	E2E_TESTS_DOCKERFILE    = e2e-tests-aarch64.Dockerfile
endif

ifneq (,$(wildcard $(shell pwd)/target/release/spin))
	E2E_SPIN_RELEASE_VOLUME_MOUNT = -v $(shell pwd)/target/release/spin:/from-host/target/release/spin
endif

ifneq (,$(wildcard $(shell pwd)/target/debug/spin))
	E2E_SPIN_DEBUG_VOLUME_MOUNT = -v $(shell pwd)/target/debug/spin:/from-host/target/debug/spin
endif

## Reset volume mounts for e2e-tests if Darwin because the
## spin binaries built on macOS won't run in the docker container
ifeq ($(shell uname -s),Darwin)
	E2E_SPIN_RELEASE_VOLUME_MOUNT = 
	E2E_SPIN_DEBUG_VOLUME_MOUNT =
	E2E_BUILD_SPIN = true
endif

.PHONY: build
build:
	cargo build --release

.PHONY: install
install:
	cargo install --path . --locked

.PHONY: test
test: lint test-unit test-integration

.PHONY: lint
lint: lint-rust-examples-and-testcases
	cargo clippy --all --all-targets --all-features -- -D warnings
	cargo fmt --all -- --check

.PHONY: lint-rust-examples-and-testcases
lint-rust-examples-and-testcases:
	for manifest_path in examples/*/Cargo.toml tests/testcases/*/Cargo.toml; do \
		cargo clippy --manifest-path "$${manifest_path}" -- -D warnings \
		&& cargo fmt --manifest-path "$${manifest_path}" -- --check \
		|| exit 1 ; \
	done

.PHONY: test-unit
test-unit:
	RUST_LOG=$(LOG_LEVEL) cargo test --all --no-fail-fast -- --skip integration_tests --skip spinup_tests --skip cloud_tests --nocapture

.PHONY: test-integration
test-integration: test-kv test-sqlite
	RUST_LOG=$(LOG_LEVEL) cargo test --test integration --no-fail-fast -- --skip spinup_tests --skip cloud_tests --nocapture

.PHONY: test-spin-up
test-spin-up: build-test-spin-up run-test-spin-up

.PHONY: build-test-spin-up
build-test-spin-up:
	docker build -t spin-e2e-tests --build-arg FETCH_SPIN=$(E2E_FETCH_SPIN) --build-arg BUILD_SPIN=$(E2E_BUILD_SPIN) -f $(E2E_TESTS_DOCKERFILE) .

.PHONY: run-test-spin-up
run-test-spin-up:
	REDIS_IMAGE=$(REDIS_IMAGE) MYSQL_IMAGE=$(MYSQL_IMAGE) POSTGRES_IMAGE=$(POSTGRES_IMAGE) \
	BUILD_SPIN=$(E2E_BUILD_SPIN) \
	docker compose -f e2e-tests-docker-compose.yml run $(E2E_SPIN_RELEASE_VOLUME_MOUNT) $(E2E_SPIN_DEBUG_VOLUME_MOUNT) e2e-tests

.PHONY: test-kv
test-kv: build
	PATH=$$(pwd)/target/release:$$PATH RUST_LOG=$(LOG_LEVEL) cargo test --test spinup_tests --features e2e-tests --no-fail-fast -- spinup_tests::key_value --nocapture

.PHONY: test-sqlite
test-sqlite: build
	PATH=$$(pwd)/target/release:$$PATH RUST_LOG=$(LOG_LEVEL) cargo test --test spinup_tests --features e2e-tests --no-fail-fast -- spinup_tests::sqlite --nocapture

.PHONY: test-outbound-redis
test-outbound-redis:
	RUST_LOG=$(LOG_LEVEL) cargo test --test integration --features outbound-redis-tests --no-fail-fast -- --nocapture

.PHONY: test-config-provider
test-config-provider:
	RUST_LOG=$(LOG_LEVEL) cargo test --test integration --features config-provider-tests --no-fail-fast -- integration_tests::config_provider_tests --nocapture

.PHONY: test-sdk-go
test-sdk-go:
	$(MAKE) -C sdk/go test

# simple convenience for developing with TLS
.PHONY: tls
tls: ${CERT_NAME}.crt.pem

$(CERT_NAME).crt.pem:
	openssl req -newkey rsa:2048 -nodes -keyout $(CERT_NAME).key.pem -x509 -days 365 -out $(CERT_NAME).crt.pem
