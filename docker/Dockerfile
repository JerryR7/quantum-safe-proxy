# Build stage
FROM rust:1.86-slim-bullseye as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create a new empty project
WORKDIR /app
RUN USER=root cargo new --bin quantum-proxy
WORKDIR /app/quantum-proxy

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Build dependencies to cache them
RUN cargo build --release
RUN rm src/*.rs

# Copy source code
COPY src ./src
COPY examples ./examples
COPY tests ./tests

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /app/quantum-proxy/target/release/quantum-proxy /app/quantum-proxy

# Create directories
RUN mkdir -p /app/certs

# Set the entrypoint
ENTRYPOINT ["/app/quantum-proxy"]

# Default command
CMD ["--help"]

# Metadata
LABEL org.opencontainers.image.title="Quantum Proxy"
LABEL org.opencontainers.image.description="PQC-Enabled Sidecar with Hybrid Certificate Support"
LABEL org.opencontainers.image.source="https://github.com/yourusername/quantum-proxy"
LABEL org.opencontainers.image.version="0.1.0"
LABEL org.opencontainers.image.licenses="MIT"
