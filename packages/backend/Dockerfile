# syntax=docker/dockerfile:1

FROM rust:1.88-slim-bookworm AS builder

WORKDIR /app

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo build --release --package backend

RUN strip /app/target/release/backend

FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    libpq5 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/backend /usr/local/bin/backend

COPY --from=builder /app/packages/migrator/migrations /app/migrations

RUN useradd -r -s /bin/false appuser && \
    chown -R appuser:appuser /app
USER appuser

ENV APP_HOST=0.0.0.0
ENV APP_PORT=3000
ENV RUST_LOG=info
ENV DATABASE_URL=postgresql://postgres:postgres@localhost:5432/app

EXPOSE 3000

ENTRYPOINT ["backend"]
