FROM rust:1.85-slim AS builder
WORKDIR /app

COPY Cargo.toml ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/memvid-export-api /usr/local/bin/memvid-export-api

ENV MEMVID_EXPORT_BIND_ADDR=0.0.0.0:8080
ENV MEMVID_EXPORT_ROOT=/data/exports
ENV MEMVID_EXPORT_RETENTION_SECONDS=86400

VOLUME ["/data/exports"]
EXPOSE 8080

CMD ["memvid-export-api"]
