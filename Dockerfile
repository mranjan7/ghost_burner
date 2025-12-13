FROM rust:latest

FROM debian:bookworm-slim
WORKDIR /home/dailyuser/projects/solana/ghost-burner

COPY . .

RUN cargo build