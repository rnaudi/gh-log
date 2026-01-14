FROM rust:1.92-alpine3.21 AS builder

WORKDIR /app

RUN apk add --no-cache musl-dev

COPY Cargo.toml Cargo.lock ./
COPY . .

RUN cargo build --release --locked

FROM alpine:3.21

RUN apk add --no-cache ca-certificates && \
    addgroup -g 1000 appuser && \
    adduser -D -u 1000 -G appuser appuser

COPY --from=builder /app/target/release/gh-log /usr/local/bin/gh-log

USER appuser

ENTRYPOINT ["gh-log"]
