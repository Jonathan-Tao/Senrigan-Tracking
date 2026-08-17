default:
    @just --list

fmt:
    cargo fmt --all -- --check

check:
    cargo check --workspace --all-targets

test:
    cargo test --workspace --all-targets

lint:
    cargo clippy --workspace --all-targets -- -D warnings

ci: fmt check test lint
