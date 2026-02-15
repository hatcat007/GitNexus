mod api;
mod artifact_store;
mod auth;
mod config;
mod memvid_writer;
mod models;
mod queue;
mod transform;

use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use config::Config;
use models::JobRecord;
use tokio::sync::{mpsc, RwLock};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub jobs: Arc<RwLock<HashMap<String, JobRecord>>>,
    pub queue_tx: mpsc::Sender<String>,
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
    };

    queue::spawn_export_worker(state.clone(), queue_rx);
    queue::spawn_cleanup_worker(state.clone());

    let app = Router::new()
        .route("/healthz", get(api::healthz))
        .route("/v1/exports", post(api::create_export))
        .route(
            "/v1/exports/{job_id}",
            get(api::get_export).delete(api::cancel_export),
        )
        .route("/v1/exports/{job_id}/download", get(api::download_export))
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
