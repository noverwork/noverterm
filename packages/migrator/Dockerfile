FROM rust:1.85-bookworm AS tools

RUN cargo install diesel_cli --no-default-features --features postgres \
    && cargo install cargo-make

WORKDIR /app
COPY . .

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libpq5 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=tools /usr/local/cargo/bin/diesel /usr/local/bin/diesel
COPY --from=tools /usr/local/cargo/bin/cargo-make /usr/local/bin/cargo-make
COPY --from=tools /app /app
