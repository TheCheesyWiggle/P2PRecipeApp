# ---- builder ----
FROM rust:1.82-slim AS builder

RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev build-essential && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

COPY Cargo.toml ./

COPY src ./src

RUN cargo fetch

RUN cargo build --release

# ---- runtime ----
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

RUN mkdir -p /app/data

COPY --from=builder /usr/src/app/target/release/p2p_recipe /app/p2p-recipe

RUN chmod +x /app/p2p-recipe

EXPOSE 4001

ENV STORAGE_FILE_PATH=/app/data/recipes.json

CMD ["/app/p2p-recipe"]
