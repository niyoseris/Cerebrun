# syntax=docker/dockerfile:1

FROM rust:1.94.1-slim AS builder
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential pkg-config ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./

# Build dependencies once with a stub binary so source-only changes do not
# recompile the whole dependency graph.
RUN mkdir -p src \
    && printf 'fn main() {}\n' > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY src ./src
COPY migrations ./migrations
COPY static ./static
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl passwd \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 cerebrun

WORKDIR /app
COPY --from=builder /app/target/release/user-context-mcp /usr/local/bin/user-context-mcp

USER cerebrun
EXPOSE 5050
ENV SERVER_HOST=0.0.0.0 \
    SERVER_PORT=5050

HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD curl -fsS http://127.0.0.1:5050/health || exit 1

ENTRYPOINT ["/usr/local/bin/user-context-mcp"]
