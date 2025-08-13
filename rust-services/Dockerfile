# Multi-stage Docker build for AlphaPulse Rust services
FROM rust:1.75 as builder

WORKDIR /app

# Copy Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY common/Cargo.toml ./common/
COPY collectors/Cargo.toml ./collectors/
COPY api-server/Cargo.toml ./api-server/

# Create empty source files to build dependencies
RUN mkdir -p common/src collectors/src api-server/src && \
    echo "fn main() {}" > collectors/src/main.rs && \
    echo "fn main() {}" > api-server/src/main.rs && \
    echo "// placeholder" > common/src/lib.rs

# Build dependencies
RUN cargo build --release

# Copy source code
COPY common/src ./common/src/
COPY collectors/src ./collectors/src/
COPY api-server/src ./api-server/src/

# Build applications
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN groupadd -r alphapulse && useradd -r -g alphapulse alphapulse

# Copy binaries
COPY --from=builder /app/target/release/alphapulse-collectors /usr/local/bin/
COPY --from=builder /app/target/release/alphapulse-api-server /usr/local/bin/

# Switch to app user
USER alphapulse

# Default command (can be overridden)
CMD ["alphapulse-collectors"]

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3001/health || exit 1