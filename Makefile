LOG_LEVEL ?= spin=trace
CERT_NAME ?= local
SPIN_DOC_NAME ?= new-doc.md

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
	RUST_LOG=$(LOG_LEVEL) cargo test --all --no-fail-fast -- --skip integration_tests --nocapture --include-ignored

.PHONY: test-integration
test-integration:
	RUST_LOG=$(LOG_LEVEL) cargo test --test integration --no-fail-fast -- --nocapture --include-ignored

.PHONY: test-e2e
test-e2e:
	RUST_LOG=$(LOG_LEVEL) cargo test --test integration --features e2e-tests --no-fail-fast  -- integration_tests::test_dependencies --nocapture 
	RUST_LOG=$(LOG_LEVEL) cargo test --test integration --features e2e-tests --no-fail-fast -- --skip integration_tests::test_dependencies --nocapture --include-ignored

.PHONY: test-sdk-go
test-sdk-go:
	$(MAKE) -C sdk/go test

# simple convenience for developing with TLS
.PHONY: tls
tls: ${CERT_NAME}.crt.pem

$(CERT_NAME).crt.pem:
	openssl req -newkey rsa:2048 -nodes -keyout $(CERT_NAME).key.pem -x509 -days 365 -out $(CERT_NAME).crt.pem

.PHONY: doc
doc:
	DATE=$(shell date --utc +%Y-%m-%dT%TZ)
	echo "title = \"<insert title here>\"\ntemplate = \"main\"\ndate = \"`date --utc +%Y-%m-%dT%TZ`\"\n" > docs/content/$(SPIN_DOC_NAME)
	echo "[extra]\nurl = \"https://github.com/fermyon/spin/blob/main/docs/content/$(SPIN_DOC_NAME)\"\n\n---\n" >> docs/content/$(SPIN_DOC_NAME)

.PHONY: check-content
check-content:
	cd docs && bart check content/* && bart check content/**/*
