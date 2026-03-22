.PHONY: check fmt clippy test test-all audit deny build doc clean

check: fmt clippy test audit

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --all-targets -- -D warnings
	cargo clippy --all-features --all-targets -- -D warnings

test:
	cargo test

test-all:
	cargo test --no-default-features
	cargo test --features http
	cargo test --features ws
	cargo test --features unix
	cargo test --all-features

audit:
	cargo audit

deny:
	cargo deny check

build:
	cargo build --release

doc:
	cargo doc --no-deps --all-features

clean:
	cargo clean
