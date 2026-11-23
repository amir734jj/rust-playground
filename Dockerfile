FROM rust:1.100-alpine AS builder
RUN apk add --no-cache build-base
WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --locked --release --bin rust-api

FROM alpine:3.24 AS runtime
RUN addgroup -g 10001 -S app \
    && adduser -u 10001 -S -D -H -G app -s /sbin/nologin app \
    && mkdir /data \
    && chown app:app /data

COPY --from=builder /build/target/release/rust-api /usr/local/bin/rust-api
WORKDIR /app

ENV BIND_ADDR=0.0.0.0:3000 \
    DATABASE_URL=/data/app.sqlite
EXPOSE 3000
VOLUME ["/data"]
USER app

ENTRYPOINT ["/usr/local/bin/rust-api"]
