# syntax=docker/dockerfile:1

FROM rust:1-alpine AS builder
RUN apk add --no-cache musl-dev openssl-dev pkgconfig git nodejs npm curl bash
WORKDIR /app
COPY . .
RUN rustup target add wasm32-unknown-unknown
RUN cargo install trunk --locked
WORKDIR /app/crates/da20-ui
RUN trunk build --release
WORKDIR /app
RUN cargo build --release -p da20-server

FROM alpine:3.20 AS runtime
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/release/da20-performance /da20-performance
ENV PORT=8080
EXPOSE 8080
USER nobody
ENTRYPOINT ["/da20-performance"]

# Optional scratch stage (same binary, no shell)
FROM scratch AS scratch
COPY --from=builder /app/target/release/da20-performance /da20-performance
COPY --from=runtime /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
ENV PORT=8080
EXPOSE 8080
ENTRYPOINT ["/da20-performance"]
