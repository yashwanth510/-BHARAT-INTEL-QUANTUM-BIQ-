# ─── Stage 1: Dependency cache layer ─────────────────────────────────────────
FROM rust:bookworm AS deps

WORKDIR /usr/src/app

RUN apt-get update && apt-get install -y \
    cmake \
    pkg-config \
    libssl-dev \
    build-essential \
    curl \
    libsasl2-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests only — cache dependency compilation
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

# ─── Stage 2: Build full binary ───────────────────────────────────────────────
FROM deps AS builder

COPY . .
# Touch main.rs to force rebuild (avoid cached dummy build)
RUN touch src/main.rs && cargo build --release

# ─── Stage 3: Minimal runtime image ──────────────────────────────────────────
FROM debian:bookworm-slim AS runner

# Security: create non-root user
RUN groupadd -r biq --gid=1001 && useradd -r -g biq --uid=1001 biq

RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/app/target/release/quantum-api ./quantum-api
RUN chown -R biq:biq /app

USER biq

EXPOSE 8000

HEALTHCHECK --interval=15s --timeout=5s --start-period=30s --retries=5 \
  CMD curl -sf http://localhost:8000/health || exit 1

CMD ["./quantum-api"]
