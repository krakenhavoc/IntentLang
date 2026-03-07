FROM rust:1.86-slim AS builder

WORKDIR /build
COPY . .
RUN cargo build --release --bin intent

FROM debian:bookworm-slim
COPY --from=builder /build/target/release/intent /usr/local/bin/intent
ENTRYPOINT ["intent"]
