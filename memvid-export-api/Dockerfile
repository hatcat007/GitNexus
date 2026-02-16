# syntax=docker/dockerfile:1.7
FROM rust:1.89-slim-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./

# Prime dependency cache: only reruns when Cargo.toml/Cargo.lock changes.
RUN mkdir -p src && printf "fn main() {}\n" > src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=memvid-cargo-registry \
  --mount=type=cache,target=/usr/local/cargo/git,id=memvid-cargo-git \
  --mount=type=cache,target=/app/target,id=memvid-target \
  cargo build --release --locked

COPY src ./src

# Reuse cached deps and compile the real binary.
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=memvid-cargo-registry \
  --mount=type=cache,target=/usr/local/cargo/git,id=memvid-cargo-git \
  --mount=type=cache,target=/app/target,id=memvid-target \
  cargo build --release --locked \
  && cp /app/target/release/memvid-export-api /usr/local/bin/memvid-export-api

FROM debian:bookworm-slim
RUN apt-get update \
&& apt-get install -y --no-install-recommends ca-certificates curl wget \
&& rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/local/bin/memvid-export-api /usr/local/bin/memvid-export-api
RUN mkdir -p /data/exports

ENV MEMVID_EXPORT_BIND_ADDR=0.0.0.0:8080
ENV MEMVID_EXPORT_ROOT=/data/exports
ENV MEMVID_EXPORT_RETENTION_SECONDS=86400

EXPOSE 8080
# Coolify rolling updates rely on health status and often require curl/wget in-image.
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=5 \
  CMD ["/usr/bin/curl", "-fsS", "http://127.0.0.1:8080/healthz"]

CMD ["/usr/local/bin/memvid-export-api"]
