mod api;
mod artifact_store;
mod auth;
mod config;
mod mcp_api;
mod mcp_index;
mod memvid_writer;
mod models;
mod queue;
mod rate_limit;
mod transform;

use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use config::Config;
use mcp_api::{new_query_cache, QueryCache};
use mcp_index::CapsuleIndex;
use models::JobRecord;
use rate_limit::RateLimiter;
use tokio::sync::{mpsc, RwLock};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;

const MAX_EXPORT_BODY_BYTES: usize = 500 * 1024 * 1024;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub jobs: Arc<RwLock<HashMap<String, JobRecord>>>,
    pub queue_tx: mpsc::Sender<String>,
    pub mcp_indexes: Arc<RwLock<HashMap<String, Arc<CapsuleIndex>>>>,
    pub mcp_cache: Arc<tokio::sync::Mutex<QueryCache>>,
    pub rate_limiter: Arc<RateLimiter>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "memvid_export_api=info,tower_http=info".into()),
        )
        .init();

    let config = Config::from_env()?;
    artifact_store::ensure_export_root(&config.export_root).await?;

    let (queue_tx, queue_rx) = mpsc::channel(config.queue_capacity);
    let state = AppState {
        config: config.clone(),
        jobs: Arc::new(RwLock::new(HashMap::new())),
        queue_tx,
        mcp_indexes: Arc::new(RwLock::new(HashMap::new())),
        mcp_cache: Arc::new(tokio::sync::Mutex::new(new_query_cache(
            config.mcp_cache_capacity,
        ))),
        rate_limiter: Arc::new(RateLimiter::new(
            config.mcp_rate_limit_per_minute,
            config.mcp_rate_limit_burst,
        )),
    };

    queue::spawn_export_worker(state.clone(), queue_rx);
    queue::spawn_cleanup_worker(state.clone());

    let app = Router::new()
        .route("/healthz", get(api::healthz))
        .route("/mcp", post(mcp_api::mcp))
        .route("/v1/exports", post(api::create_export))
        .route(
            "/v1/exports/{job_id}",
            get(api::get_export).delete(api::cancel_export),
        )
        .route("/v1/exports/{job_id}/download", get(api::download_export))
        .layer(DefaultBodyLimit::max(MAX_EXPORT_BODY_BYTES))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    info!("memvid-export-api listening on {}", config.bind_addr);
    axum::serve(listener, app).await?;
    Ok(())
}
