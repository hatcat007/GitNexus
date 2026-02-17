use std::{env, fs, net::SocketAddr, path::PathBuf};

use anyhow::Result;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportBackendMode {
    LegacyVps,
    RunpodQueue,
}

impl ExportBackendMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LegacyVps => "legacy_vps",
            Self::RunpodQueue => "runpod_queue",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbeddingMode {
    ExternalApi,
    RunpodGpu,
}

impl EmbeddingMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ExternalApi => "external_api",
            Self::RunpodGpu => "runpod_gpu",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub api_key: String,
    pub api_key_is_fallback: bool,
    pub export_root: PathBuf,
    pub retention_seconds: u64,
    pub queue_capacity: usize,
    pub mcp_response_budget_bytes: usize,
    pub mcp_rate_limit_per_minute: u32,
    pub mcp_rate_limit_burst: u32,
    pub mcp_dev_log_payloads: bool,
    pub mcp_allow_external_capsules: bool,
    pub mcp_cache_capacity: usize,
    pub backend_mode: ExportBackendMode,
    pub runpod_api_base: String,
    pub runpod_endpoint_id: Option<String>,
    pub runpod_api_key: Option<String>,
    pub runpod_region_scope: String,
    pub runpod_poll_interval_seconds: u64,
    pub runpod_execution_timeout_ms: u64,
    pub runpod_ttl_ms: u64,
    pub staging_root: PathBuf,
    pub embedding_mode: EmbeddingMode,
    pub embedding_provider: String,
    pub nvidia_api_key: Option<String>,
    pub ollama_host: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let bind_raw =
            env::var("MEMVID_EXPORT_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
        let bind_normalized = bind_raw
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();
        let bind_addr = bind_normalized
            .parse::<SocketAddr>()
            .unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], 8080)));

        let (api_key, api_key_is_fallback) = resolve_api_key();

        let export_root = PathBuf::from(
            env::var("MEMVID_EXPORT_ROOT").unwrap_or_else(|_| "/data/exports".to_string()),
        );

        let retention_seconds = env::var("MEMVID_EXPORT_RETENTION_SECONDS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(24 * 60 * 60);

        let queue_capacity = env::var("MEMVID_EXPORT_QUEUE_CAPACITY")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(128);

        let mcp_response_budget_bytes = env::var("MEMVID_MCP_RESPONSE_BUDGET_BYTES")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(64 * 1024);

        let mcp_rate_limit_per_minute = env::var("MEMVID_MCP_RATE_LIMIT_PER_MINUTE")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(120);

        let mcp_rate_limit_burst = env::var("MEMVID_MCP_RATE_LIMIT_BURST")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(60);

        let mcp_dev_log_payloads = env::var("MEMVID_MCP_DEV_LOG_PAYLOADS")
            .ok()
            .map(|v| {
                matches!(
                    v.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(false);

        let mcp_allow_external_capsules = env::var("MEMVID_MCP_ALLOW_EXTERNAL_CAPSULES")
            .ok()
            .map(|v| {
                matches!(
                    v.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(false);

        let mcp_cache_capacity = env::var("MEMVID_MCP_CACHE_CAPACITY")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(256);

        let backend_mode = match env::var("MEMVID_EXPORT_BACKEND_MODE")
            .unwrap_or_else(|_| "legacy_vps".to_string())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "runpod_queue" => ExportBackendMode::RunpodQueue,
            _ => ExportBackendMode::LegacyVps,
        };

        let runpod_api_base = env::var("RUNPOD_API_BASE")
            .unwrap_or_else(|_| "https://api.runpod.ai/v2".to_string())
            .trim_end_matches('/')
            .to_string();
        let runpod_endpoint_id = env::var("RUNPOD_ENDPOINT_ID")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let runpod_api_key = env::var("RUNPOD_API_KEY")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let runpod_region_scope = env::var("RUNPOD_REGION_SCOPE")
            .unwrap_or_else(|_| "EU-primary".to_string())
            .trim()
            .to_string();
        let runpod_poll_interval_seconds = env::var("RUNPOD_POLL_INTERVAL_SECONDS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(5)
            .max(1);
        let runpod_execution_timeout_ms = env::var("RUNPOD_EXECUTION_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(600_000)
            .max(5_000);
        let runpod_ttl_ms = env::var("RUNPOD_TTL_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(86_400_000)
            .max(10_000);
        let staging_root = PathBuf::from(
            env::var("MEMVID_EXPORT_STAGING_ROOT")
                .unwrap_or_else(|_| "/data/exports/staging".to_string()),
        );

        let embedding_mode = match env::var("MEMVID_EMBEDDING_MODE")
            .unwrap_or_else(|_| "external_api".to_string())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "runpod_gpu" => EmbeddingMode::RunpodGpu,
            _ => EmbeddingMode::ExternalApi,
        };
        let embedding_provider = env::var("MEMVID_EMBED_PROVIDER")
            .unwrap_or_else(|_| "nvidia".to_string())
            .trim()
            .to_string();
        let nvidia_api_key = env::var("NVIDIA_API_KEY")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let ollama_host = env::var("OLLAMA_HOST")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());

        Ok(Self {
            bind_addr,
            api_key,
            api_key_is_fallback,
            export_root,
            retention_seconds,
            queue_capacity,
            mcp_response_budget_bytes,
            mcp_rate_limit_per_minute,
            mcp_rate_limit_burst,
            mcp_dev_log_payloads,
            mcp_allow_external_capsules,
            mcp_cache_capacity,
            backend_mode,
            runpod_api_base,
            runpod_endpoint_id,
            runpod_api_key,
            runpod_region_scope,
            runpod_poll_interval_seconds,
            runpod_execution_timeout_ms,
            runpod_ttl_ms,
            staging_root,
            embedding_mode,
            embedding_provider,
            nvidia_api_key,
            ollama_host,
        })
    }

    pub fn runpod_enabled(&self) -> bool {
        matches!(self.backend_mode, ExportBackendMode::RunpodQueue)
            && self.runpod_endpoint_id.is_some()
            && self.runpod_api_key.is_some()
    }
}

fn resolve_api_key() -> (String, bool) {
    if let Ok(value) = env::var("MEMVID_EXPORT_API_KEY") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return (trimmed.to_string(), false);
        }
    }

    if let Ok(key_file) = env::var("MEMVID_EXPORT_API_KEY_FILE") {
        match fs::read_to_string(&key_file) {
            Ok(raw) => {
                let trimmed = raw.trim();
                if !trimmed.is_empty() {
                    return (trimmed.to_string(), false);
                }
                eprintln!(
                    "[memvid-export-api] MEMVID_EXPORT_API_KEY_FILE is empty: {}. Falling back to generated key.",
                    key_file
                );
            }
            Err(err) => {
                eprintln!(
                    "[memvid-export-api] Failed reading MEMVID_EXPORT_API_KEY_FILE at {}: {}. Falling back to generated key.",
                    key_file, err
                );
            }
        }
    } else {
        eprintln!(
            "[memvid-export-api] MEMVID_EXPORT_API_KEY not set. Falling back to generated key."
        );
    }

    let generated = format!("fallback-{}", Uuid::new_v4());
    (generated, true)
}
