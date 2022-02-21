LOG_LEVEL ?= spin=trace

.PHONY: build
build:
	cargo build --release

.PHONY: test
test:
	RUST_LOG=$(LOG_LEVEL) cargo test --all -- --nocapture
	cargo clippy --all-targets --all-features -- -D warnings
	cargo fmt --all -- --check
