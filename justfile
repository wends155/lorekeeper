set shell := ["pwsh.exe", "-c"]

fmt:
    cargo fmt --all -- --check

clippy:
    cargo clippy --all-targets --all-features -- -D warnings

test:
    cargo test --all-features

check: fmt clippy test

build:
    cargo build

install:
    cargo install --path .

doc:
    cargo doc --no-deps --all-features
