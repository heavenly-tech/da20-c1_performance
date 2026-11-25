# One-time CI base for Woodpecker agents (build and push before enabling pipelines).
# Pins: Rust 1.89, trunk 0.22.1, wasm-bindgen-cli 0.2.130 (matches Cargo.lock + Dockerfile).
#
# Example:
#   docker build -t registry.heavenly.cl/da20-ci:1.89-0.22.1-0.2.130 -f ci/Dockerfile ci/
#   docker push registry.heavenly.cl/da20-ci:1.89-0.22.1-0.2.130

FROM rust:1.89-bookworm

RUN rustup component add rustfmt clippy rust-src \
    && rustup target add wasm32-unknown-unknown

RUN cargo install trunk --locked --version 0.22.1 \
    && cargo install wasm-bindgen-cli --locked --version 0.2.130

RUN apt-get update \
    && apt-get install -y --no-install-recommends nodejs npm \
    && rm -rf /var/lib/apt/lists/*

ENV PATH="/root/.cargo/bin:${PATH}"