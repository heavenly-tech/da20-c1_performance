# syntax=docker/dockerfile:1

# --- Base: OS deps + pinned WASM toolchain (cached unless versions change) ---
FROM rust:1-alpine AS chef
RUN apk add --no-cache musl-dev openssl-dev pkgconfig git nodejs npm curl bash
RUN cargo install cargo-chef --locked --version 0.1.71 \
    && cargo install trunk --locked --version 0.22.1 \
    && cargo install wasm-bindgen-cli --locked --version 0.2.130
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup target add wasm32-unknown-unknown

# --- Dependency recipe (invalidates only when Cargo.toml / Cargo.lock change) ---
FROM chef AS planner
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/da20-core/Cargo.toml crates/da20-core/Cargo.toml
COPY crates/da20-server/Cargo.toml crates/da20-server/Cargo.toml
COPY crates/da20-ui/Cargo.toml crates/da20-ui/Cargo.toml
RUN mkdir -p crates/da20-core/src crates/da20-ui/src crates/da20-server/src \
    && printf '%s\n' 'pub fn _docker_dep_stub() {}' > crates/da20-core/src/lib.rs \
    && printf '%s\n' 'pub fn _docker_dep_stub() {}' > crates/da20-ui/src/lib.rs \
    && printf '%s\n' 'fn main() {}' > crates/da20-server/src/main.rs \
    && mkdir -p dist && printf '%s\n' '<!doctype html><html></html>' > dist/index.html
RUN cargo chef prepare --recipe-path recipe.json

# --- Compile and cache dependency layers (per TARGETARCH musl + wasm32) ---
FROM chef AS cacher
ARG TARGETARCH
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
# BuildKit (buildx --platform) sets TARGETARCH; plain docker build may not — fall back to host.
RUN set -eu; \
    arch="${TARGETARCH:-}"; \
    if [ -z "${arch}" ]; then \
      case "$(uname -m)" in \
        x86_64) arch=amd64 ;; \
        aarch64|arm64) arch=arm64 ;; \
        *) echo "unsupported uname -m for TARGETARCH fallback: $(uname -m)" >&2; exit 1 ;; \
      esac; \
      echo "TARGETARCH unset — defaulting to host ${arch} (use: docker buildx build --platform linux/amd64[,linux/arm64] .)" >&2; \
    fi; \
    case "${arch}" in \
      amd64) MUSL_TARGET=x86_64-unknown-linux-musl ;; \
      arm64) MUSL_TARGET=aarch64-unknown-linux-musl ;; \
      *) echo "unsupported TARGETARCH: ${arch}" >&2; exit 1 ;; \
    esac; \
    echo "${MUSL_TARGET}" > /tmp/musl_target; \
    rustup target add "${MUSL_TARGET}" wasm32-unknown-unknown
RUN MUSL_TARGET="$(cat /tmp/musl_target)" \
    && cargo chef cook --release --recipe-path recipe.json --target "${MUSL_TARGET}" -p da20-server \
    && cargo chef cook --release --recipe-path recipe.json --target wasm32-unknown-unknown -p da20-ui

# --- Application build (source changes only rebuild this stage + above invalidation) ---
FROM cacher AS builder
WORKDIR /app
COPY . .
RUN rustup target add wasm32-unknown-unknown
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
