# Run all CI checks locally: make check
.PHONY: check fmt clippy test test-all audit deny build doc coverage bench clean

check: fmt clippy test audit

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --all-targets -- -D warnings
	cargo clippy --no-default-features --all-targets -- -D warnings
	cargo clippy --all-features --all-targets -- -D warnings

test:
	cargo test

test-all:
	cargo test --no-default-features
	cargo test --features http
	cargo test --features ws
	cargo test --features unix
	cargo test --features audit
	cargo test --features events
	cargo test --all-features

audit:
	cargo audit

deny:
	cargo deny check

build:
	cargo build --release

doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

coverage:
	cargo llvm-cov --all-features --html --output-dir coverage/

bench:
	bash scripts/bench-log.sh

clean:
	cargo clean
	rm -rf coverage/
