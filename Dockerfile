# Builder stage
FROM rust:1.75 AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    cmake build-essential libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy source
COPY Cargo.toml Cargo.toml
COPY src src
COPY scripts scripts

# Build release
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install only runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates bash \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/audio-transcriber /usr/local/bin/

# Create models directory
RUN mkdir -p /app/models

# Copy download script
COPY scripts/download_model.sh /app/scripts/download_model.sh
RUN chmod +x /app/scripts/download_model.sh

# Set working directory for audio files
WORKDIR /audio

# Default: show help
ENTRYPOINT ["audio-transcriber"]
CMD ["--help"]
