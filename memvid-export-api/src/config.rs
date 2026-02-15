use std::{env, fs, net::SocketAddr, path::PathBuf};

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub api_key: String,
    pub export_root: PathBuf,
    pub retention_seconds: u64,
    pub queue_capacity: usize,
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

        Ok(Self {
            bind_addr,
            api_key,
            export_root,
            retention_seconds,
            queue_capacity,
        })
    }
}
