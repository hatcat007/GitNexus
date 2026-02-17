use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::json;
use tokio::process::Command;

use crate::{
    artifact_store::{build_job_file_name, ensure_job_dir},
    config::Config,
    embedding::{default_model_for_provider, EmbeddingRuntimeConfig},
    mcp_index::build_and_persist_from_request,
    memvid_writer::write_mv2_core_only,
    models::ExportRequest,
    transform::build_frame_documents,
};

#[derive(Debug, Clone)]
struct RunnerArgs {
    job_id: String,
    payload_ref: String,
    output_prefix: String,
    embedding_mode: String,
    embedding_provider: String,
    embedding_model: String,
}

pub async fn maybe_run_from_cli(args: &[String]) -> Result<bool> {
    if args.len() < 2 || args[1] != "runpod-execute" {
        return Ok(false);
    }

    let parsed = parse_args(args).context("Invalid runpod-execute arguments")?;
    let payload = load_payload(&parsed.payload_ref).await?;
    let request: ExportRequest =
        serde_json::from_slice(&payload).context("Failed to decode staged payload JSON")?;

    let docs = build_frame_documents(&request);
    let output_dir = resolve_output_prefix(&parsed.output_prefix)?;
    tokio::fs::create_dir_all(&output_dir)
        .await
        .with_context(|| format!("Failed to create output directory {}", output_dir.display()))?;

    let date_stamp = Utc::now().format("%Y-%m-%d").to_string();
    let file_name = build_job_file_name(&request.source.base_name, &date_stamp);
    let output_path = output_dir.join(&file_name);
    ensure_job_dir(&output_path).await?;
    let env_config = Config::from_env().context("Failed to load embedding env config")?;
    let embedding_config = if request.options.semantic_enabled {
        Some(EmbeddingRuntimeConfig::new(
            &parsed.embedding_mode,
            &parsed.embedding_provider,
            &parsed.embedding_model,
            env_config.nvidia_api_key.clone(),
            env_config.openai_api_key.clone(),
            env_config.voyage_api_key.clone(),
            env_config.ollama_host.clone(),
            env_config.nvidia_embed_base_url.clone(),
            env_config.openai_embed_base_url.clone(),
            env_config.voyage_embed_base_url.clone(),
            env_config.voyage_input_type.clone(),
            env_config.voyage_output_dimension,
            env_config.voyage_output_dtype.clone(),
            env_config.voyage_truncation,
            env_config.embed_request_timeout_seconds,
        )?)
    } else {
        None
    };

    write_mv2_core_only(
        &output_path,
        &docs,
        request.options.semantic_enabled,
        embedding_config,
        |_written, _total| {},
    )?;

    let sidecar_status = match build_and_persist_from_request(&request, &docs, &output_path) {
        Ok(index) => json!({
            "status": "ready",
            "sidecarPath": index.sidecar_path,
            "nodes": index.nodes.len(),
            "edges": index.edges.len()
        }),
        Err(err) => json!({
            "status": "skipped",
            "reason": err.to_string()
        }),
    };

    let artifact_meta = tokio::fs::metadata(&output_path)
        .await
        .with_context(|| format!("Failed to stat {}", output_path.display()))?;

    let result = json!({
        "backend": "rust-memvid-core",
        "jobId": parsed.job_id,
        "fileName": file_name,
        "artifactPath": output_path,
        "artifactRef": format!("file://{}", output_path.display()),
        "sizeBytes": artifact_meta.len(),
        "embeddingMode": parsed.embedding_mode,
        "embeddingProvider": parsed.embedding_provider,
        "embeddingModel": parsed.embedding_model,
        "sidecar": sidecar_status
    });
    println!("{}", serde_json::to_string(&result)?);
    Ok(true)
}

fn parse_args(args: &[String]) -> Result<RunnerArgs> {
    let mut job_id = None;
    let mut payload_ref = None;
    let mut output_prefix = None;
    let mut embedding_mode = None;
    let mut embedding_provider = None;
    let mut embedding_model = None;

    let mut i = 2usize;
    while i < args.len() {
        let key = args[i].as_str();
        let val = args.get(i + 1).cloned();
        match (key, val) {
            ("--job-id", Some(v)) => {
                job_id = Some(v);
                i += 2;
            }
            ("--payload-ref", Some(v)) => {
                payload_ref = Some(v);
                i += 2;
            }
            ("--output-prefix", Some(v)) => {
                output_prefix = Some(v);
                i += 2;
            }
            ("--embedding-mode", Some(v)) => {
                embedding_mode = Some(v);
                i += 2;
            }
            ("--embedding-provider", Some(v)) => {
                embedding_provider = Some(v);
                i += 2;
            }
            ("--embedding-model", Some(v)) => {
                embedding_model = Some(v);
                i += 2;
            }
            _ => {
                anyhow::bail!("Unknown or incomplete argument near `{}`", key);
            }
        }
    }

    let embedding_mode = embedding_mode.unwrap_or_else(|| "external_api".to_string());
    let embedding_provider = embedding_provider.unwrap_or_else(|| "nvidia".to_string());
    let embedding_model = embedding_model.unwrap_or_else(|| {
        let provider = embedding_provider.to_ascii_lowercase();
        default_model_for_provider(&provider)
            .unwrap_or("nvidia/nv-embed-v1")
            .to_string()
    });

    Ok(RunnerArgs {
        job_id: job_id.context("--job-id is required")?,
        payload_ref: payload_ref.context("--payload-ref is required")?,
        output_prefix: output_prefix.context("--output-prefix is required")?,
        embedding_mode,
        embedding_provider,
        embedding_model,
    })
}

fn resolve_file_ref_path(file_ref: &str) -> PathBuf {
    if let Some(stripped) = file_ref.strip_prefix("file://") {
        PathBuf::from(stripped)
    } else {
        PathBuf::from(file_ref)
    }
}

async fn load_payload(payload_ref: &str) -> Result<Vec<u8>> {
    if payload_ref.starts_with("http://") || payload_ref.starts_with("https://") {
        let output = Command::new("curl")
            .arg("-sS")
            .arg(payload_ref)
            .output()
            .await
            .with_context(|| format!("Failed to download payload from {payload_ref}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Payload download failed: {}", stderr.trim());
        }
        return Ok(output.stdout);
    }

    let payload_path = resolve_file_ref_path(payload_ref);
    tokio::fs::read(&payload_path)
        .await
        .with_context(|| format!("Failed reading staged payload {}", payload_path.display()))
}

fn resolve_output_prefix(prefix: &str) -> Result<PathBuf> {
    let path = if prefix.starts_with("http://") || prefix.starts_with("https://") {
        anyhow::bail!("HTTP output prefixes are not supported in runpod-execute mode");
    } else if prefix.starts_with("file://") {
        resolve_file_ref_path(prefix)
    } else {
        PathBuf::from(prefix)
    };

    if path == Path::new("/") {
        anyhow::bail!("Refusing to write output to filesystem root");
    }
    Ok(path)
}
