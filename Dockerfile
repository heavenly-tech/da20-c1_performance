# syntax=docker/dockerfile:1

FROM rust:1-alpine AS builder
RUN apk add --no-cache musl-dev openssl-dev pkgconfig git nodejs npm curl bash
WORKDIR /app
COPY . .
RUN rustup target add x86_64-unknown-linux-musl wasm32-unknown-unknown
# Trunk downloads glibc wasm-bindgen binaries that fail on Alpine (ENOENT). Install musl-native CLI.
RUN cargo install trunk --locked \
    && cargo install wasm-bindgen-cli --version 0.2.130 --locked
ENV PATH="/root/.cargo/bin:${PATH}"
WORKDIR /app/crates/da20-ui
RUN trunk build --release
WORKDIR /app
RUN RUSTFLAGS="-C target-feature=+crt-static" cargo build --release -p da20-server --target x86_64-unknown-linux-musl

FROM scratch
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/da20-performance /da20-performance
ENV PORT=8080
EXPOSE 8080
USER 65534:65534
ENTRYPOINT ["/da20-performance"]
