LOG_LEVEL ?= spin=trace
CERT_NAME ?= local

.PHONY: build
build:
	cargo build --release

.PHONY: test
test:
	RUST_LOG=$(LOG_LEVEL) cargo test --all -- --nocapture
	cargo clippy --all-targets --all-features -- -D warnings
	cargo fmt --all -- --check

# simple convenience for developing with TLS
.PHONY: tls
tls: ${CERT_NAME}.crt.pem

$(CERT_NAME).crt.pem:
	openssl req -newkey rsa:2048 -nodes -keyout $(CERT_NAME).key.pem -x509 -days 365 -out $(CERT_NAME).crt.pem