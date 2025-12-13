FROM docker.io/rust:latest AS builder
WORKDIR /app

COPY . .
RUN cargo build

FROM docker.io/debian:bookworm-slim

COPY --from=builder /app/target/debug/ghost /usr/local/bin/ghost

