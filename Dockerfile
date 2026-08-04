# syntax=docker/dockerfile:1
FROM rust:1-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /build
RUN rustup target add x86_64-unknown-linux-musl

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM scratch AS final
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/honeytrap /honeytrap
COPY templates /templates
EXPOSE 8090
ENTRYPOINT ["/honeytrap", "serve", "--listen", "0.0.0.0:8090", "--templates", "/templates"]
