# Runpod Worker Adapter

This adapter is a thin Runpod queue handler that delegates export execution to the Rust binary:

- Python handler: `runpod-worker/handler.py`
- Rust executable: `/usr/local/bin/memvid-export-api`
- Rust runner mode: `runpod-execute`

The adapter does **not** implement an alternate writer. The `.mv2` generation path is the same Rust `memvid-core` pipeline used by the API worker.

## Build

```bash
cd memvid-export-api
docker build -f runpod-worker/Dockerfile -t memvid-export-runpod-worker:latest .
```

## Expected Runpod Input Payload

```json
{
  "input": {
    "job_id": "uuid",
    "payload_ref": "file:///workspace/staging/payloads/<job_id>.json",
    "output_prefix": "file:///workspace/staging/outputs/<job_id>",
    "embedding_mode": "external_api",
    "embedding_provider": "nvidia",
    "ollama_host": "http://127.0.0.1:11434"
  }
}
```

## Notes

- Use a shared network volume for both `payload_ref` and `output_prefix` paths when the control plane and Runpod workers need to share files.
- If you choose `embedding_mode=runpod_gpu`, set `ollama_host` or `OLLAMA_HOST` in worker env.
