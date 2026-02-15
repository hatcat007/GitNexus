use std::{env, fs, net::SocketAddr, path::PathBuf};

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub api_key: String,
    pub export_root: PathBuf,
    pub retention_seconds: u64,
    pub queue_capacity: usize,
    pub mcp_response_budget_bytes: usize,
    pub mcp_rate_limit_per_minute: u32,
    pub mcp_rate_limit_burst: u32,
    pub mcp_dev_log_payloads: bool,
    pub mcp_allow_external_capsules: bool,
    pub mcp_cache_capacity: usize,
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

        let api_key = match env::var("MEMVID_EXPORT_API_KEY") {
            Ok(value) if !value.trim().is_empty() => value,
            _ => {
                let key_file = env::var("MEMVID_EXPORT_API_KEY_FILE")
                    .context("Set MEMVID_EXPORT_API_KEY or MEMVID_EXPORT_API_KEY_FILE")?;
                let key = fs::read_to_string(&key_file).with_context(|| {
                    format!("Failed to read MEMVID_EXPORT_API_KEY_FILE at {key_file}")
                })?;
                let trimmed = key.trim().to_string();
                if trimmed.is_empty() {
                    anyhow::bail!("MEMVID_EXPORT_API_KEY_FILE is empty: {key_file}");
                }
                trimmed
            }
        };

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

        Ok(Self {
            bind_addr,
            api_key,
            export_root,
            retention_seconds,
            queue_capacity,
            mcp_response_budget_bytes,
            mcp_rate_limit_per_minute,
            mcp_rate_limit_burst,
            mcp_dev_log_payloads,
            mcp_allow_external_capsules,
            mcp_cache_capacity,
        })
    }
}
