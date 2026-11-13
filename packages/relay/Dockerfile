# Build from the repository root:
#   docker build -f packages/relay/Dockerfile -t noverterm-relay .
FROM rust:1-slim-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build -p relay --release

FROM debian:bookworm-slim
COPY --from=build /src/target/release/noverterm-relay /usr/local/bin/noverterm-relay
VOLUME /data
EXPOSE 2222 8080 8081
ENTRYPOINT ["noverterm-relay"]
