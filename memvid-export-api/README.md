# memvid-export-api

Rust API for exporting GitNexus graph data into Memvid `.mv2` capsules.

## Features

- `POST /v1/exports` enqueue export jobs
- `GET /v1/exports/{jobId}` poll status
- `GET /v1/exports/{jobId}/download` download completed capsule
- `DELETE /v1/exports/{jobId}` cancel queued/running jobs
- Static bearer auth for all `/v1/*` routes
- 24h artifact retention cleanup (configurable)
- `memvid-core` writer with automatic fallback to `memvid` CLI if core write fails at runtime

> CLI fallback requires `memvid` to be installed and available in `PATH`.

## Environment Variables

- `MEMVID_EXPORT_API_KEY` (required): bearer token expected by the API
- `MEMVID_EXPORT_BIND_ADDR` (default `0.0.0.0:8080`)
- `MEMVID_EXPORT_ROOT` (default `/data/exports`)
- `MEMVID_EXPORT_RETENTION_SECONDS` (default `86400`)
- `MEMVID_EXPORT_QUEUE_CAPACITY` (default `128`)

## Run locally

```bash
cd memvid-export-api
export MEMVID_EXPORT_API_KEY=change-me
cargo run
```

Rust toolchain requirement: `>= 1.89`.

## Build Docker image

```bash
docker build -t memvid-export-api .
docker run --rm -p 8080:8080 \
  -e MEMVID_EXPORT_API_KEY=change-me \
  -v $(pwd)/data:/data/exports \
  memvid-export-api
```

For Coolify, you can also use `/memvid-export-api/docker-compose.coolify.yml`.
