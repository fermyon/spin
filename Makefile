LOG_LEVEL ?= spin=trace
CERT_NAME ?= local
SPIN_DOC_NAME ?= new-doc.md

ARCH = $(shell uname -p)

## dependencies for e2e-tests
E2E_VOLUME_MOUNT     ?=
E2E_BUILD_SPIN       ?= false
E2E_TESTS_DOCKERFILE ?= e2e-tests.Dockerfile
MYSQL_IMAGE          ?= mysql:8.0.22
REDIS_IMAGE          ?= redis:7.0.8-alpine3.17
POSTGRES_IMAGE       ?= postgres:14.7-alpine

## overrides for aarch64
ifneq ($(ARCH),x86_64)
	MYSQL_IMAGE 		 = arm64v8/mysql:8.0.32
	REDIS_IMAGE 		 = arm64v8/redis:6.0-alpine3.17
	POSTGRES_IMAGE 		 = arm64v8/postgres:14.7
	E2E_TESTS_DOCKERFILE = e2e-tests-aarch64.Dockerfile
endif

.PHONY: build
build:
	cargo build --release

.PHONY: test
test: lint test-unit test-integration

.PHONY: lint
lint:
	cargo clippy --all-targets --all-features -- -D warnings
	cargo fmt --all -- --check

.PHONY: check-rust-examples
check-rust-examples:
	for manifest_path in examples/*/Cargo.toml; do \
		cargo clippy --manifest-path "$${manifest_path}" -- -D warnings || exit 1 ; \
	done

.PHONY: test-unit
test-unit:
	RUST_LOG=$(LOG_LEVEL) cargo test --all --no-fail-fast -- --skip integration_tests --skip spinup_tests --skip cloud_tests --nocapture

.PHONY: test-integration
test-integration:
	RUST_LOG=$(LOG_LEVEL) cargo test --test integration --no-fail-fast -- --skip spinup_tests --skip cloud_tests --nocapture

.PHONY: test-fermyon-platform
test-fermyon-platform:
	RUST_LOG=$(LOG_LEVEL) cargo test --test integration --features fermyon-platform --no-fail-fast -- integration_tests::test_dependencies --skip spinup_tests --nocapture
	RUST_LOG=$(LOG_LEVEL) cargo test --test integration --features fermyon-platform --no-fail-fast -- --skip integration_tests::test_dependencies --skip spinup_tests --nocapture

.PHONY: test-spin-up
test-spin-up:
	docker build -t spin-e2e-tests --build-arg BUILD_SPIN=$(E2E_BUILD_SPIN) -f $(E2E_TESTS_DOCKERFILE) .
	REDIS_IMAGE=$(REDIS_IMAGE) MYSQL_IMAGE=$(MYSQL_IMAGE) POSTGRES_IMAGE=$(POSTGRES_IMAGE) \
	docker compose -f e2e-tests-docker-compose.yml run $(E2E_VOLUME_MOUNT) e2e-tests 

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
