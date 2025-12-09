# ---- builder ----
FROM rust:1.82-slim AS builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev build-essential && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy source to cache dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source and build
COPY src ./src
RUN touch src/main.rs && \
    cargo build --release

# ---- runtime ----
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Create data directory for recipes
RUN mkdir -p /app/data

# Copy binary from builder
COPY --from=builder /usr/src/app/target/release/P2PRecipe /app/p2p-recipe

# Set proper permissions
RUN chmod +x /app/p2p-recipe

# Expose P2P port
EXPOSE 4001

# Set working directory for recipes.json
ENV STORAGE_FILE_PATH=/app/data/recipes.json

# Run the application
CMD ["/app/p2p-recipe"]