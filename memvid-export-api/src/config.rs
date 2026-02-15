use std::{env, net::SocketAddr, path::PathBuf};

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
        let bind_addr = env::var("MEMVID_EXPORT_BIND_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
            .parse::<SocketAddr>()
            .context("Invalid MEMVID_EXPORT_BIND_ADDR")?;

        let api_key =
            env::var("MEMVID_EXPORT_API_KEY").context("MEMVID_EXPORT_API_KEY is required")?;

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
