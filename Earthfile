VERSION 0.8
IMPORT github.com/earthly/lib/rust:3.0.1 AS rust

FROM rust:slim-bookworm
WORKDIR /dotsamaworld
ENV CARGO_TERM_COLOR=always

prep:
    RUN apt-get update && apt-get install curl gcc pkg-config openssl libasound2-dev cmake build-essential python3 libfreetype6-dev libexpat1-dev libxcb-composite0-dev libssl-dev libx11-dev libfontconfig1-dev -y -qq
    # CARGO function adds caching to cargo runs.
    # See https://github.com/earthly/lib/tree/main/rust
    DO rust+INIT --keep_fingerprints=true
    RUN rustup target add wasm32-unknown-unknown
    RUN rustup component add rustfmt clippy
    RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
    RUN cargo binstall trunk -y
    COPY --keep-ts --dir assets src crates index.html fixup.sh Cargo.lock Cargo.toml .

build:
    FROM +prep
    DO rust+CARGO --args="build --release --workspace --target wasm32-unknown-unknown" --output="release/[^/\.]+"

fmt:
    FROM +prep
    DO rust+CARGO --args="fmt --check --all"

check:
    FROM +prep
    DO rust+CARGO --args="clippy --target wasm32-unknown-unknown --workspace"

test:
    FROM +prep
    DO rust+CARGO --args="test --workspace --locked --verbose"

dist:
    FROM +prep
    RUN trunk build --release
    RUN ./fixup.sh
