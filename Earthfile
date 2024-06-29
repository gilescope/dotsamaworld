VERSION 0.8
IMPORT github.com/earthly/lib/rust:3.0.1 AS rust

FROM rust:slim-bookworm
WORKDIR /dotsamaworld
ENV CARGO_TERM_COLOR=always

prep:
    # CARGO function adds caching to cargo runs.
    # See https://github.com/earthly/lib/tree/main/rust
    DO rust+INIT --keep_fingerprints=true
    COPY --keep-ts --dir assets src crates Cargo.lock Cargo.toml .
    RUN rustup target add wasm32-unknown-unknown
    RUN rustup component add rustfmt clippy

build:
    FROM +prep
    DO rust+CARGO --args="build --release --target wasm32-unknown-unknown" --output="release/[^/\.]+"

fmt:
    FROM +prep
    DO rust+CARGO --args="fmt --check --all"

check:
    FROM +prep
    DO rust+CARGO --args="clippy --target wasm32-unknown-unknown"

test:
    FROM +prep
    RUN apt-get update && apt-get install gcc pkg-config openssl libasound2-dev cmake build-essential python3 libfreetype6-dev libexpat1-dev libxcb-composite0-dev libssl-dev libx11-dev libfontconfig1-dev -y -qq
    DO rust+CARGO --args="test --locked --verbose"
