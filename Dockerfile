FROM rust:1.99.1 AS builder

WORKDIR /app

RUN apt update && apt install sqlite3

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

COPY . .

RUN cargo build --release

FROM debian:bookworm AS runtime

RUN apt update && apt install -y sqlite3 openssl ca-certificates

RUN mkdir /app

WORKDIR /app

COPY --from=builder /app/target/release/mate_bot /app/mate_bot

USER 1000

CMD ["/app/mate_bot"]
