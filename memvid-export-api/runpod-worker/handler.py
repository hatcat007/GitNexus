import json
import os
import subprocess
from typing import Any, Dict

import runpod


RUST_EXECUTABLE = os.getenv("RUNPOD_RUST_EXECUTABLE", "/usr/local/bin/memvid-export-api")


def _required(input_data: Dict[str, Any], key: str) -> str:
    value = input_data.get(key)
    if value is None or str(value).strip() == "":
        raise ValueError(f"Missing required input field: {key}")
    return str(value)


def handler(job: Dict[str, Any]) -> Dict[str, Any]:
    job_input = job.get("input", {})
    runpod_job_id = str(job.get("id", "unknown"))

    job_id = _required(job_input, "job_id")
    payload_ref = _required(job_input, "payload_ref")
    output_prefix = _required(job_input, "output_prefix")
    embedding_mode = str(job_input.get("embedding_mode", "external_api"))
    embedding_provider = str(job_input.get("embedding_provider", "nvidia"))

    cmd = [
        RUST_EXECUTABLE,
        "runpod-execute",
        "--job-id",
        job_id,
        "--payload-ref",
        payload_ref,
        "--output-prefix",
        output_prefix,
        "--embedding-mode",
        embedding_mode,
        "--embedding-provider",
        embedding_provider,
    ]

    env = os.environ.copy()
    ollama_host = job_input.get("ollama_host")
    if ollama_host:
        env["OLLAMA_HOST"] = str(ollama_host)

    proc = subprocess.run(
        cmd,
        text=True,
        capture_output=True,
        env=env,
        check=False,
    )

    if proc.returncode != 0:
        return {
            "error": {
                "code": "RUST_EXECUTION_FAILED",
                "message": "Rust export pipeline execution failed",
                "details": proc.stderr[-4000:],
            },
            "runpodJobId": runpod_job_id,
            "jobId": job_id,
        }

    try:
        parsed = json.loads(proc.stdout.strip())
    except json.JSONDecodeError as err:
        return {
            "error": {
                "code": "RUST_OUTPUT_INVALID_JSON",
                "message": f"Rust runner returned invalid JSON: {err}",
                "stdout": proc.stdout[-4000:],
                "stderr": proc.stderr[-4000:],
            },
            "runpodJobId": runpod_job_id,
            "jobId": job_id,
        }

    parsed["runpodJobId"] = runpod_job_id
    parsed["adapter"] = "python-runpod-handler"
    return parsed


if __name__ == "__main__":
    runpod.serverless.start({"handler": handler})
