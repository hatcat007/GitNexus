use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{Duration as ChronoDuration, Utc};
use tokio::{fs, sync::mpsc, time};
use tracing::{error, info, warn};

use crate::{
    artifact_store::{build_job_file_name, delete_file_if_exists, ensure_job_dir, job_output_path},
    mcp_index::build_and_persist_from_request,
    memvid_writer::write_mv2,
    models::{ExportArtifact, ExportErrorPayload, JobState},
    transform::build_frame_documents,
    AppState,
};

pub fn spawn_export_worker(state: AppState, mut queue_rx: mpsc::Receiver<String>) {
    tokio::spawn(async move {
        while let Some(job_id) = queue_rx.recv().await {
            info!(job_id = %job_id, "Worker picked export job");
            if let Err(err) = process_export_job(state.clone(), &job_id).await {
                error!("Export job {job_id} failed: {err:#}");
                let mut jobs = state.jobs.write().await;
                if let Some(job) = jobs.get_mut(&job_id) {
                    job.status = JobState::Failed;
                    job.progress = 100.0;
                    job.updated_at = Utc::now();
                    job.error = Some(ExportErrorPayload {
                        code: "EXPORT_FAILED".to_string(),
                        message: err.to_string(),
                    });
                    job.message = Some("Export failed".to_string());
                    job.request = None;
                }
            }
        }
    });
}

pub fn spawn_cleanup_worker(state: AppState) {
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(err) = cleanup_expired_artifacts(&state).await {
                warn!("Cleanup worker error: {err:#}");
            }
        }
    });
}

async fn set_job_progress(
    state: &AppState,
    job_id: &str,
    progress: f64,
    message: impl Into<String>,
) -> Result<bool> {
    let message = message.into();
    let mut jobs = state.jobs.write().await;
    let Some(job) = jobs.get_mut(job_id) else {
        anyhow::bail!("Unknown job id: {job_id}");
    };

    if matches!(job.status, JobState::Canceled) {
        info!(job_id = %job_id, progress, "Skipping progress update: job canceled");
        return Ok(false);
    }

    job.progress = progress;
    job.message = Some(message.clone());
    job.updated_at = Utc::now();

    info!(job_id = %job_id, progress, message = %message, "Export progress update");
    Ok(true)
}

async fn process_export_job(state: AppState, job_id: &str) -> Result<()> {
    let request = {
        let mut jobs = state.jobs.write().await;
        let Some(job) = jobs.get_mut(job_id) else {
            anyhow::bail!("Unknown job id: {job_id}");
        };

        if matches!(job.status, JobState::Canceled) {
            info!("Skipping canceled job {job_id}");
            return Ok(());
        }

        job.status = JobState::Running;
        job.progress = 5.0;
        job.message = Some("Transforming graph data".to_string());
        job.updated_at = Utc::now();
        job.error = None;

        job.request.clone().context("Missing request payload")?
    };

    info!(job_id = %job_id, progress = 5.0, message = "Transforming graph data", "Export progress update");

    info!(
        job_id = %job_id,
        session_id = %request.session_id,
        project = %request.project_name,
        nodes = request.nodes.len(),
        relationships = request.relationships.len(),
        files = request.file_contents.len(),
        semantic = request.options.semantic_enabled,
        "Export job started"
    );

    let date_stamp = Utc::now().format("%Y-%m-%d").to_string();
    let file_name = build_job_file_name(&request.source.base_name, &date_stamp);
    let output_path = job_output_path(&state.config.export_root, job_id, &file_name);
    ensure_job_dir(&output_path).await?;

    if !set_job_progress(&state, job_id, 20.0, "Preparing frame documents").await? {
        return Ok(());
    }

    let docs = build_frame_documents(&request);

    info!(
        job_id = %job_id,
        frames = docs.len(),
        "Frame documents prepared"
    );

    if !set_job_progress(
        &state,
        job_id,
        45.0,
        format!("Prepared {} frames", docs.len()),
    )
    .await?
    {
        return Ok(());
    }

    if !set_job_progress(
        &state,
        job_id,
        60.0,
        format!("Writing {} frames to capsule", docs.len()),
    )
    .await?
    {
        return Ok(());
    }

    write_mv2(&output_path, &docs, request.options.semantic_enabled)?;

    info!(
        job_id = %job_id,
        output_path = %output_path.display(),
        "MV2 artifact write finished"
    );

    let request_for_index = request.clone();
    let docs_for_index = docs.clone();
    let output_path_for_index = output_path.clone();
    let job_id_for_index = job_id.to_string();
    match tokio::task::spawn_blocking(move || {
        build_and_persist_from_request(&request_for_index, &docs_for_index, &output_path_for_index)
    })
    .await
    {
        Ok(Ok(index)) => {
            info!(
                job_id = %job_id_for_index,
                sidecar = %index.sidecar_path.display(),
                nodes = index.nodes.len(),
                edges = index.edges.len(),
                "Sidecar MCP index built"
            );
        }
        Ok(Err(err)) => {
            warn!(
                job_id = %job_id_for_index,
                "Failed to build sidecar MCP index: {err:#}"
            );
        }
        Err(err) => {
            warn!(
                job_id = %job_id_for_index,
                "Sidecar MCP index task join error: {err:#}"
            );
        }
    }

    if !set_job_progress(&state, job_id, 80.0, "Finalizing artifact metadata").await? {
        delete_file_if_exists(&output_path).await?;
        return Ok(());
    }

    {
        let jobs = state.jobs.read().await;
        if jobs
            .get(job_id)
            .map(|job| matches!(job.status, JobState::Canceled))
            .unwrap_or(false)
        {
            delete_file_if_exists(&output_path).await?;
            return Ok(());
        }
    }

    let metadata = fs::metadata(&output_path)
        .await
        .with_context(|| format!("Failed to stat {}", output_path.display()))?;

    let now = Utc::now();
    let expires_at = now + ChronoDuration::seconds(state.config.retention_seconds as i64);

    if !set_job_progress(&state, job_id, 90.0, "Preparing download artifact").await? {
        delete_file_if_exists(&output_path).await?;
        return Ok(());
    }

    let artifact = ExportArtifact {
        file_name: file_name.clone(),
        download_url: format!("/v1/exports/{job_id}/download"),
        expires_at,
        size_bytes: metadata.len(),
    };

    let mut jobs = state.jobs.write().await;
    if let Some(job) = jobs.get_mut(job_id) {
        job.status = JobState::Completed;
        job.progress = 100.0;
        job.message = Some("Export completed".to_string());
        job.updated_at = now;
        job.artifact = Some(artifact);
        job.artifact_path = Some(output_path);
        job.error = None;
        job.request = None;
    }

    info!(
        job_id = %job_id,
        artifact = %file_name,
        size_bytes = metadata.len(),
        "Export job completed"
    );

    Ok(())
}

async fn cleanup_expired_artifacts(state: &AppState) -> Result<()> {
    let now = Utc::now();
    let mut files_to_delete = Vec::new();

    {
        let mut jobs = state.jobs.write().await;
        for job in jobs.values_mut() {
            if !matches!(job.status, JobState::Completed) {
                continue;
            }

            let Some(artifact) = &job.artifact else {
                continue;
            };

            if artifact.expires_at <= now {
                if let Some(path) = &job.artifact_path {
                    files_to_delete.push(path.clone());
                }
                info!(job_id = %job.job_id, "Expiring export artifact");
                job.status = JobState::Expired;
                job.updated_at = now;
                job.message = Some("Artifact expired and removed".to_string());
                job.artifact = None;
                job.artifact_path = None;
            }
        }
    }

    for path in files_to_delete {
        if let Err(err) = delete_file_if_exists(&path).await {
            warn!(
                "Failed to delete expired artifact {}: {err:#}",
                path.display()
            );
        }
    }

    Ok(())
}
