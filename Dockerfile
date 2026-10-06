# syntax=docker/dockerfile:1
FROM rust:1.85-bookworm AS builder

WORKDIR /usr/src/zapfast

# Install build dependencies: CMake, Perl, and OpenSSL
RUN apt-get update && apt-get install -y --no-install-recommends \
    cmake \
    perl \
    build-essential \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace source code
COPY . .

# Build the headless ZapFast Server binary in release mode
RUN cargo build --release --bin zapfast-server

# Runtime Stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime shared libraries
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

# Copy compiled binary from builder
COPY --from=builder /usr/src/zapfast/target/release/zapfast-server /app/zapfast-server

# Persistent volume for accounts, sessions, and databases
VOLUME ["/app/data"]

# Expose HTTP / WebSocket API & Web UI (8080) and UDP Auto-Discovery (47120)
EXPOSE 8080
EXPOSE 47120/udp

ENV RUST_LOG=info

ENTRYPOINT ["/app/zapfast-server", "--port", "8080", "--data-dir", "/app/data"]
