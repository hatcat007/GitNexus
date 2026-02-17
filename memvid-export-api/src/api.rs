use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, Response, StatusCode},
    response::{sse::Event, sse::KeepAlive, IntoResponse, Sse},
    Json,
};
use chrono::Utc;
use serde_json::json;
use std::{convert::Infallible, time::Duration};
use tokio::fs;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    artifact_store::delete_file_if_exists,
    auth::verify_bearer,
    config::ExportBackendMode,
    models::{
        ExportAcceptedResponse, ExportEventType, ExportEventsResponse, ExportLogEvent,
        ExportRequest, ExportStage, JobBackendMetadata, JobRecord, JobState,
    },
    queue::append_job_event,
    AppState,
};

const EVENTS_DEFAULT_LIMIT: usize = 200;
const EVENTS_MAX_LIMIT: usize = 2_000;

pub async fn healthz() -> impl IntoResponse {
    Json(json!({ "ok": true, "timestamp": Utc::now() }))
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsQueryParams {
    #[serde(default, rename = "sinceSeq")]
    pub since_seq: Option<u64>,
    #[serde(default)]
    pub limit: Option<usize>,
}

pub async fn create_export(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ExportRequest>,
) -> impl IntoResponse {
    if let Err(err) = verify_bearer(&headers, &state.config.api_key) {
        return err.into_response();
    }

    if payload.nodes.is_empty() || payload.relationships.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": {
                    "code": "INVALID_EXPORT_REQUEST",
                    "message": "Request must include graph nodes and relationships."
                }
            })),
        )
            .into_response();
    }

    let now = Utc::now();
    let job_id = Uuid::new_v4().to_string();
    let session_id = payload.session_id.clone();
    let project_name = payload.project_name.clone();
    let node_count = payload.nodes.len();
    let relation_count = payload.relationships.len();
    let file_count = payload.file_contents.len();

    let record = JobRecord {
        job_id: job_id.clone(),
        created_at: now,
        updated_at: now,
        status: JobState::Queued,
        progress: 0.0,
        message: Some("Queued for export".to_string()),
        request: Some(payload),
        artifact: None,
        error: None,
        artifact_path: None,
        events: std::collections::VecDeque::new(),
        next_seq: 1,
        current_stage: ExportStage::Queued,
        stage_progress: 0.0,
        last_event_at: now,
        metadata: Some(JobBackendMetadata {
            backend: match state.config.backend_mode {
                ExportBackendMode::LegacyVps => "legacy_vps".to_string(),
                ExportBackendMode::RunpodQueue => "runpod_queue".to_string(),
            },
            runpod_job_id: None,
            payload_ref: None,
            artifact_ref: None,
            worker_metrics: None,
        }),
    };

    {
        let mut jobs = state.jobs.write().await;
        jobs.insert(job_id.clone(), record);
    }
    {
        let mut buses = state.event_buses.write().await;
        let (sender, _) = tokio::sync::broadcast::channel(512);
        buses.insert(job_id.clone(), sender);
    }

    if state.queue_tx.send(job_id.clone()).await.is_err() {
        let mut jobs = state.jobs.write().await;
        jobs.remove(&job_id);
        let mut buses = state.event_buses.write().await;
        buses.remove(&job_id);
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": {
                    "code": "QUEUE_UNAVAILABLE",
                    "message": "Export queue is unavailable."
                }
            })),
        )
            .into_response();
    }

    info!(
        job_id = %job_id,
        session_id = %session_id,
        project = %project_name,
        nodes = node_count,
        relationships = relation_count,
        files = file_count,
        "Export job queued"
    );

    let response = ExportAcceptedResponse {
        job_id: job_id.clone(),
        status: JobState::Queued,
        progress: 0.0,
        current_stage: ExportStage::Queued,
        stage_progress: 0.0,
        message: Some("Queued for export".to_string()),
        created_at: now,
    };

    let _ = append_job_event(
        &state,
        &job_id,
        ExportEventType::StageProgress,
        ExportStage::Queued,
        0.0,
        Some(0.0),
        "Queued for export",
        Some(json!({
            "nodes": node_count,
            "relationships": relation_count,
            "files": file_count
        })),
    )
    .await;

    (StatusCode::ACCEPTED, Json(response)).into_response()
}

pub async fn get_export(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    if let Err(err) = verify_bearer(&headers, &state.config.api_key) {
        return err.into_response();
    }

    let jobs = state.jobs.read().await;
    let Some(job) = jobs.get(&job_id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": {
                    "code": "JOB_NOT_FOUND",
                    "message": "Export job not found."
                }
            })),
        )
            .into_response();
    };

    (StatusCode::OK, Json(job.to_response())).into_response()
}

pub async fn cancel_export(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    if let Err(err) = verify_bearer(&headers, &state.config.api_key) {
        return err.into_response();
    }

    let mut artifact_to_delete = None;
    let mut became_canceled = false;
    let response = {
        let mut jobs = state.jobs.write().await;
        let Some(job) = jobs.get_mut(&job_id) else {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": {
                        "code": "JOB_NOT_FOUND",
                        "message": "Export job not found."
                    }
                })),
            )
                .into_response();
        };

        if matches!(job.status, JobState::Queued | JobState::Running) {
            became_canceled = true;
            job.status = JobState::Canceled;
            job.current_stage = ExportStage::Canceled;
            job.stage_progress = 100.0;
            job.progress = 100.0;
            job.updated_at = Utc::now();
            job.message = Some("Export canceled".to_string());
            job.error = None;
            job.request = None;
            if let Some(path) = &job.artifact_path {
                artifact_to_delete = Some(path.clone());
            }
            job.artifact = None;
            job.artifact_path = None;
        }

        info!(job_id = %job_id, status = ?job.status, "Export job cancel request handled");

        job.to_response()
    };

    if became_canceled {
        let _ = append_job_event(
            &state,
            &job_id,
            ExportEventType::JobCanceled,
            ExportStage::Canceled,
            100.0,
            Some(100.0),
            "Export canceled",
            None,
        )
        .await;
    }

    if let Some(path) = artifact_to_delete {
        if let Err(err) = delete_file_if_exists(&path).await {
            warn!(
                "Failed removing artifact during cancel {}: {err:#}",
                path.display()
            );
        }
    }

    (StatusCode::OK, Json(response)).into_response()
}

pub async fn get_export_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(job_id): Path<String>,
    Query(params): Query<EventsQueryParams>,
) -> impl IntoResponse {
    if let Err(err) = verify_bearer(&headers, &state.config.api_key) {
        return err.into_response();
    }

    let since_seq = params.since_seq.unwrap_or(0);
    let limit = params
        .limit
        .unwrap_or(EVENTS_DEFAULT_LIMIT)
        .clamp(1, EVENTS_MAX_LIMIT);

    let jobs = state.jobs.read().await;
    let Some(job) = jobs.get(&job_id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": {
                    "code": "JOB_NOT_FOUND",
                    "message": "Export job not found."
                }
            })),
        )
            .into_response();
    };

    if matches!(job.status, JobState::Expired) {
        return (
            StatusCode::GONE,
            Json(json!({
                "error": {
                    "code": "JOB_EXPIRED",
                    "message": "Export job has expired."
                }
            })),
        )
            .into_response();
    }

    let mut events = Vec::with_capacity(limit);
    for event in job.events.iter().filter(|event| event.seq > since_seq) {
        events.push(event.clone());
        if events.len() >= limit {
            break;
        }
    }

    let response = ExportEventsResponse {
        job_id: job_id.clone(),
        events,
        next_seq: job.next_seq,
        status_snapshot: job.to_response(),
    };

    (StatusCode::OK, Json(response)).into_response()
}

pub async fn stream_export_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(job_id): Path<String>,
    Query(params): Query<EventsQueryParams>,
) -> impl IntoResponse {
    if let Err(err) = verify_bearer(&headers, &state.config.api_key) {
        return err.into_response();
    }

    let since_seq = params.since_seq.unwrap_or(0);
    let replay_events = {
        let jobs = state.jobs.read().await;
        let Some(job) = jobs.get(&job_id) else {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": {
                        "code": "JOB_NOT_FOUND",
                        "message": "Export job not found."
                    }
                })),
            )
                .into_response();
        };

        if matches!(job.status, JobState::Expired) {
            return (
                StatusCode::GONE,
                Json(json!({
                    "error": {
                        "code": "JOB_EXPIRED",
                        "message": "Export job has expired."
                    }
                })),
            )
                .into_response();
        }

        job.events
            .iter()
            .filter(|event| event.seq > since_seq)
            .cloned()
            .collect::<Vec<_>>()
    };

    let rx = {
        let buses = state.event_buses.read().await;
        let Some(sender) = buses.get(&job_id) else {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": {
                        "code": "EVENT_STREAM_NOT_FOUND",
                        "message": "Event stream unavailable for this export job."
                    }
                })),
            )
                .into_response();
        };
        sender.subscribe()
    };

    let (tx, out_rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(256);
    tokio::spawn(async move {
        for event in replay_events {
            if tx.send(Ok(to_sse_event(&event))).await.is_err() {
                return;
            }
        }

        let mut rx = rx;
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if tx.send(Ok(to_sse_event(&event))).await.is_err() {
                        return;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
            }
        }
    });

    let stream = ReceiverStream::new(out_rx);
    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(2))
                .text("ping"),
        )
        .into_response()
}

fn to_sse_event(event: &ExportLogEvent) -> Event {
    Event::default()
        .id(event.seq.to_string())
        .event(event.event_type.as_str())
        .data(serde_json::to_string(event).unwrap_or_else(|_| {
            "{\"type\":\"stage_heartbeat\",\"message\":\"encode_error\"}".to_string()
        }))
}

pub async fn download_export(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    if let Err(err) = verify_bearer(&headers, &state.config.api_key) {
        return err.into_response();
    }

    let (path, artifact_file_name, status) = {
        let jobs = state.jobs.read().await;
        let Some(job) = jobs.get(&job_id) else {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": {
                        "code": "JOB_NOT_FOUND",
                        "message": "Export job not found."
                    }
                })),
            )
                .into_response();
        };

        if !matches!(job.status, JobState::Completed) {
            return (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": {
                        "code": "ARTIFACT_NOT_READY",
                        "message": "Export artifact is not ready for download."
                    }
                })),
            )
                .into_response();
        }

        let Some(path) = &job.artifact_path else {
            return (
                StatusCode::GONE,
                Json(json!({
                    "error": {
                        "code": "ARTIFACT_MISSING",
                        "message": "Export artifact has been removed."
                    }
                })),
            )
                .into_response();
        };

        let file_name = job
            .artifact
            .as_ref()
            .map(|a| a.file_name.clone())
            .unwrap_or_else(|| format!("{job_id}.mv2"));

        (path.clone(), file_name, job.status.clone())
    };

    if !matches!(status, JobState::Completed) {
        return (
            StatusCode::CONFLICT,
            Json(json!({
                "error": {
                    "code": "ARTIFACT_NOT_READY",
                    "message": "Export artifact is not ready for download."
                }
            })),
        )
            .into_response();
    }

    let bytes = match fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return (
                StatusCode::GONE,
                Json(json!({
                    "error": {
                        "code": "ARTIFACT_MISSING",
                        "message": "Export artifact no longer exists."
                    }
                })),
            )
                .into_response();
        }
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": {
                        "code": "ARTIFACT_READ_FAILED",
                        "message": format!("Failed to read artifact: {err}")
                    }
                })),
            )
                .into_response();
        }
    };

    let content_disposition = format!("attachment; filename=\"{artifact_file_name}\"");

    info!(
        job_id = %job_id,
        artifact = %artifact_file_name,
        bytes = bytes.len(),
        "Export artifact downloaded"
    );

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_DISPOSITION, content_disposition)
        .body(Body::from(bytes))
        .unwrap_or_else(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": {
                        "code": "RESPONSE_BUILD_FAILED",
                        "message": "Failed to build download response."
                    }
                })),
            )
                .into_response()
        })
}
