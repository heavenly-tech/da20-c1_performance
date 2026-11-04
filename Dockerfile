# syntax=docker/dockerfile:1

FROM rust:1-alpine AS builder
ARG TARGETARCH
RUN apk add --no-cache musl-dev openssl-dev pkgconfig git nodejs npm curl bash
WORKDIR /app
COPY . .
RUN case "${TARGETARCH}" in \
      amd64) MUSL_TARGET=x86_64-unknown-linux-musl ;; \
      arm64) MUSL_TARGET=aarch64-unknown-linux-musl ;; \
      *) echo "unsupported TARGETARCH: ${TARGETARCH}" >&2; exit 1 ;; \
    esac \
    && echo "${MUSL_TARGET}" > /tmp/musl_target \
    && rustup target add "$(cat /tmp/musl_target)" wasm32-unknown-unknown
# Trunk downloads glibc wasm-bindgen binaries that fail on Alpine (ENOENT). Install musl-native CLI.
RUN cargo install trunk --locked \
    && cargo install wasm-bindgen-cli --version 0.2.130 --locked
ENV PATH="/root/.cargo/bin:${PATH}"
WORKDIR /app/crates/da20-ui
RUN trunk build --release
WORKDIR /app
RUN MUSL_TARGET="$(cat /tmp/musl_target)" \
    && RUSTFLAGS="-C target-feature=+crt-static" cargo build --release -p da20-server --target "${MUSL_TARGET}" \
    && cp "/app/target/${MUSL_TARGET}/release/da20-performance" /app/da20-performance

FROM scratch
COPY --from=builder /app/da20-performance /da20-performance
ENV PORT=8080
EXPOSE 8080
USER 65534:65534
ENTRYPOINT ["/da20-performance"]
