use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, Response, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde_json::json;
use tokio::fs;
use tracing::warn;
use uuid::Uuid;

use crate::{
    artifact_store::delete_file_if_exists,
    auth::verify_bearer,
    models::{ExportAcceptedResponse, ExportRequest, JobRecord, JobState},
    AppState,
};

pub async fn healthz() -> impl IntoResponse {
    Json(json!({ "ok": true, "timestamp": Utc::now() }))
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
    };

    {
        let mut jobs = state.jobs.write().await;
        jobs.insert(job_id.clone(), record);
    }

    if state.queue_tx.send(job_id.clone()).await.is_err() {
        let mut jobs = state.jobs.write().await;
        jobs.remove(&job_id);
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

    let response = ExportAcceptedResponse {
        job_id,
        status: JobState::Queued,
        progress: 0.0,
        message: Some("Queued for export".to_string()),
        created_at: now,
    };

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
            job.status = JobState::Canceled;
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

        job.to_response()
    };

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
