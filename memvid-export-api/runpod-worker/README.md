# Runpod Worker Adapter

This worker is a thin Queue Serverless adapter. It does not implement export logic itself.

- Python handler: `runpod-worker/handler.py`
- Rust binary: `/usr/local/bin/memvid-export-api`
- Rust mode invoked by worker: `runpod-execute`

`.mv2` generation is performed by Rust `memvid-core` through the `runpod-execute` path.

## Architecture

1. Control plane (`memvid-export-api`) receives full export payload at `/v1/exports`.
2. Control plane stages payload to storage and submits a compact pointer payload to Runpod `/run`.
3. Runpod worker receives job, calls Rust runner, writes `.mv2` and sidecar.
4. Control plane polls Runpod `/status/{id}`, maps statuses to existing job/events, and serves download from `/v1/exports/{id}/download`.

## Build And Push Worker Image

```bash
cd /Users/buddythacat/Documents/TOOLS/gitnexus/memvid-export-api
docker build -f runpod-worker/Dockerfile -t <registry>/memvid-export-runpod-worker:<tag> .
docker push <registry>/memvid-export-runpod-worker:<tag>
```

## Create Queue Endpoint In Runpod (Console)

1. Open Runpod Console -> `Serverless` -> `New Endpoint`.
2. Choose Docker image and use your pushed worker image.
3. Select `Queue` endpoint type.
4. Apply reliability-first settings from:
   `runpod-worker/runpod-endpoint.update.example.json`
5. Set storage so worker can access staged payload/artifact paths.

Recommended baseline:
- `workersMin=1`
- `scalerType=QUEUE_DELAY`
- `flashboot=true`
- high-end GPU first with fallback list enabled
- `executionTimeoutMs` >= worst-case export runtime

## Required API-Side Env (Control Plane)

Set in `/Users/buddythacat/Documents/TOOLS/gitnexus/memvid-export-api/.env.example` pattern:

```dotenv
MEMVID_EXPORT_BACKEND_MODE=runpod_queue
RUNPOD_API_BASE=https://api.runpod.ai/v2
RUNPOD_ENDPOINT_ID=<endpoint-id>
RUNPOD_API_KEY=<api-key>
MEMVID_EXPORT_STAGING_ROOT=/runpod-volume/memvid/staging
```

Important:
- Current flow uses `file://` payload/artifact refs for Runpod jobs.
- `MEMVID_EXPORT_STAGING_ROOT` must resolve to a shared path visible from both control plane and worker.

## Optional Worker Env

Worker supports:

```dotenv
RUNPOD_RUST_EXECUTABLE=/usr/local/bin/memvid-export-api
OLLAMA_HOST=http://127.0.0.1:11434
NVIDIA_API_KEY=<if using external_api with nvidia provider>
OPENAI_API_KEY=<if using external_api with openai provider>
VOYAGE_API_KEY=<if using external_api with voyage provider>
VOYAGE_EMBED_BASE_URL=https://api.voyageai.com/v1
VOYAGE_INPUT_TYPE=document
VOYAGE_OUTPUT_DIMENSION=1024
VOYAGE_OUTPUT_DTYPE=float
VOYAGE_TRUNCATION=true
```

## Runpod Job Input Contract

```json
{
  "input": {
    "job_id": "uuid",
    "payload_ref": "file:///runpod-volume/memvid/staging/payloads/<job_id>.json",
    "output_prefix": "file:///runpod-volume/memvid/staging/outputs/<job_id>",
    "embedding_mode": "external_api",
    "embedding_provider": "voyage",
    "embedding_model": "voyage-code-3",
    "ollama_host": "http://127.0.0.1:11434"
  },
  "policy": {
    "executionTimeout": 600000,
    "ttl": 86400000
  }
}
```

## Status Flow Mapping

Runpod -> control-plane event semantics:

- `IN_QUEUE` -> queued heartbeat
- `IN_PROGRESS` -> processing heartbeat
- `COMPLETED` -> mark completed, register artifact path, expose download URL
- `FAILED` / `TIMED_OUT` / `CANCELLED` -> fail/cancel export job

## Validation Checklist

1. Submit export via API `/v1/exports`.
2. Confirm job metadata includes `backend=runpod_queue` and `runpodJobId`.
3. Confirm Runpod request transitions `IN_QUEUE` -> `IN_PROGRESS` -> `COMPLETED`.
4. Confirm output contains `artifactPath`/`artifactRef`.
5. Download artifact from `/v1/exports/{job_id}/download`.
6. Open artifact and verify expected `.mv2` content and sidecar generation.

## Troubleshooting

- `Missing required input field`:
  Runpod job input is missing `job_id`, `payload_ref`, or `output_prefix`.
- `Rust execution failed`:
  Rust binary returned non-zero. Check endpoint logs and stderr.
- `Runpod completed without output`:
  Worker returned malformed JSON or crashed before publishing result.
- `Failed to stat artifact`:
  Control plane cannot access path returned by worker. Check shared volume mount/path parity.
- `HTTP output prefixes are not supported`:
  `runpod-execute` currently supports filesystem output paths (`file://` or local path), not HTTP uploads.
