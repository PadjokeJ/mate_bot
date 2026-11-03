FROM rust:1.99.1

WORKDIR /app

RUN apt update && apt install sqlite3

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

COPY . .

RUN cargo build --release

ENTRYPOINT ["./target/release/mate_bot"]
