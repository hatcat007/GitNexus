# syntax=docker/dockerfile:1.7
FROM rust:1.89-slim-bookworm AS rust-builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && printf "fn main() {}\n" > src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=memvid-cargo-registry \
  --mount=type=cache,target=/usr/local/cargo/git,id=memvid-cargo-git \
  --mount=type=cache,target=/app/target,id=memvid-target \
  cargo build --release --locked

COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=memvid-cargo-registry \
  --mount=type=cache,target=/usr/local/cargo/git,id=memvid-cargo-git \
  --mount=type=cache,target=/app/target,id=memvid-target \
  cargo build --release --locked \
  && cp /app/target/release/memvid-export-api /usr/local/bin/memvid-export-api

FROM python:3.11-slim
WORKDIR /app

COPY runpod-worker/requirements.txt /app/requirements.txt
RUN pip install --no-cache-dir -r /app/requirements.txt

COPY --from=rust-builder /usr/local/bin/memvid-export-api /usr/local/bin/memvid-export-api
COPY runpod-worker/handler.py /app/handler.py

ENV RUNPOD_RUST_EXECUTABLE=/usr/local/bin/memvid-export-api
CMD ["python", "-u", "/app/handler.py"]
