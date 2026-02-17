use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::{Context, Result};
use chrono::{Duration as ChronoDuration, Utc};
use serde_json::{json, Value};
use tokio::{fs, sync::mpsc, time};
use tracing::{error, info, warn};

use crate::{
    artifact_store::{build_job_file_name, delete_file_if_exists, ensure_job_dir, job_output_path},
    config::ExportBackendMode,
    mcp_index::build_and_persist_from_request,
    memvid_writer::write_mv2,
    models::{
        ExportArtifact, ExportErrorPayload, ExportEventType, ExportLogEvent, ExportRequest,
        ExportStage, JobState,
    },
    runpod::{RunpodClient, RunpodJobInput, RunpodPolicy, RunpodRunRequest},
    transform::build_frame_documents,
    AppState,
};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(2);
const MAX_JOB_EVENTS: usize = 5_000;
const WRITE_STAGE_START: f64 = 60.0;
const WRITE_STAGE_END: f64 = 79.0;
const SIDECAR_STAGE_START: f64 = 79.0;
const SIDECAR_STAGE_END: f64 = 90.0;

fn stage_emoji(stage: &ExportStage, event_type: &ExportEventType) -> &'static str {
    match event_type {
        ExportEventType::JobCompleted => "✅",
        ExportEventType::JobFailed => "🔥",
        ExportEventType::JobCanceled => "🛑",
        ExportEventType::JobExpired => "⏳",
        ExportEventType::StageHeartbeat => "⏳",
        ExportEventType::JobStarted | ExportEventType::StageProgress => match stage {
            ExportStage::Queued => "🧾",
            ExportStage::Transform => "🧠",
            ExportStage::FramePrep => "🧱",
            ExportStage::WriteCapsule => "💾",
            ExportStage::BuildSidecar => "🗂️",
            ExportStage::Finalize => "📦",
            ExportStage::DownloadReady => "✅",
            ExportStage::Failed => "🔥",
            ExportStage::Canceled => "🛑",
            ExportStage::Expired => "⏳",
        },
    }
}

fn lerp_stage_progress(stage_progress: f64, start: f64, end: f64) -> f64 {
    let t = (stage_progress / 100.0).clamp(0.0, 1.0);
    (start + (end - start) * t).clamp(0.0, 100.0)
}

pub async fn append_job_event(
    state: &AppState,
    job_id: &str,
    event_type: ExportEventType,
    stage: ExportStage,
    progress: f64,
    stage_progress: Option<f64>,
    message: impl Into<String>,
    meta: Option<Value>,
) -> Result<Option<ExportLogEvent>> {
    let message = message.into();
    let now = Utc::now();

    let event = {
        let mut jobs = state.jobs.write().await;
        let Some(job) = jobs.get_mut(job_id) else {
            return Ok(None);
        };

        if matches!(job.status, JobState::Canceled | JobState::Expired)
            && !matches!(
                event_type,
                ExportEventType::JobCanceled | ExportEventType::JobExpired
            )
        {
            return Ok(None);
        }

        let stage_progress = stage_progress
            .unwrap_or(job.stage_progress)
            .clamp(0.0, 100.0);
        let progress = progress.clamp(0.0, 100.0);

        job.progress = progress;
        job.current_stage = stage.clone();
        job.stage_progress = stage_progress;
        job.message = Some(message.clone());
        job.updated_at = now;
        job.last_event_at = now;

        let event = ExportLogEvent {
            seq: job.next_seq,
            ts: now,
            job_id: job_id.to_string(),
            event_type: event_type.clone(),
            stage: stage.clone(),
            progress,
            stage_progress: Some(stage_progress),
            emoji: stage_emoji(&stage, &event_type).to_string(),
            message: message.clone(),
            meta,
        };
        job.next_seq = job.next_seq.saturating_add(1);
        job.events.push_back(event.clone());
        while job.events.len() > MAX_JOB_EVENTS {
            job.events.pop_front();
        }
        event
    };

    let sender = {
        let buses = state.event_buses.read().await;
        buses.get(job_id).cloned()
    };
    if let Some(sender) = sender {
        let _ = sender.send(event.clone());
    }

    info!(
        job_id = %job_id,
        seq = event.seq,
        stage = %format!("{:?}", event.stage),
        progress = event.progress,
        stage_progress = event.stage_progress.unwrap_or_default(),
        message = %event.message,
        "Export progress update"
    );

    Ok(Some(event))
}

pub fn spawn_export_worker(state: AppState, mut queue_rx: mpsc::Receiver<String>) {
    tokio::spawn(async move {
        while let Some(job_id) = queue_rx.recv().await {
            info!(job_id = %job_id, "Worker picked export job");
            let process_result = match state.config.backend_mode {
                ExportBackendMode::LegacyVps => {
                    process_export_job_legacy(state.clone(), &job_id).await
                }
                ExportBackendMode::RunpodQueue => {
                    process_export_job_runpod(state.clone(), &job_id).await
                }
            };

            if let Err(err) = process_result {
                error!("Export job {job_id} failed: {err:#}");
                let error_message = err.to_string();
                {
                    let mut jobs = state.jobs.write().await;
                    if let Some(job) = jobs.get_mut(&job_id) {
                        job.status = JobState::Failed;
                        job.progress = 100.0;
                        job.current_stage = ExportStage::Failed;
                        job.stage_progress = 100.0;
                        job.updated_at = Utc::now();
                        job.error = Some(ExportErrorPayload {
                            code: "EXPORT_FAILED".to_string(),
                            message: error_message.clone(),
                        });
                        job.message = Some("Export failed".to_string());
                        job.request = None;
                    }
                }
                let _ = append_job_event(
                    &state,
                    &job_id,
                    ExportEventType::JobFailed,
                    ExportStage::Failed,
                    100.0,
                    Some(100.0),
                    "Export failed",
                    Some(json!({ "error": error_message })),
                )
                .await;
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

async fn is_canceled(state: &AppState, job_id: &str) -> bool {
    let jobs = state.jobs.read().await;
    jobs.get(job_id)
        .map(|job| matches!(job.status, JobState::Canceled))
        .unwrap_or(true)
}

async fn process_export_job_legacy(state: AppState, job_id: &str) -> Result<()> {
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
        job.current_stage = ExportStage::Transform;
        job.stage_progress = 0.0;
        job.message = Some("Transforming graph data".to_string());
        job.updated_at = Utc::now();
        job.error = None;

        job.request.clone().context("Missing request payload")?
    };

    let _ = append_job_event(
        &state,
        job_id,
        ExportEventType::JobStarted,
        ExportStage::Transform,
        5.0,
        Some(0.0),
        "Transforming graph data",
        None,
    )
    .await?;

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

    if is_canceled(&state, job_id).await {
        return Ok(());
    }

    let _ = append_job_event(
        &state,
        job_id,
        ExportEventType::StageProgress,
        ExportStage::FramePrep,
        20.0,
        Some(0.0),
        "Preparing frame documents",
        None,
    )
    .await?;

    let docs = build_frame_documents(&request);
    info!(job_id = %job_id, frames = docs.len(), "Frame documents prepared");

    let _ = append_job_event(
        &state,
        job_id,
        ExportEventType::StageProgress,
        ExportStage::FramePrep,
        45.0,
        Some(100.0),
        format!("Prepared {} frames", docs.len()),
        Some(json!({ "frames": docs.len() })),
    )
    .await?;

    if is_canceled(&state, job_id).await {
        return Ok(());
    }

    let total_frames = docs.len().max(1);
    let _ = append_job_event(
        &state,
        job_id,
        ExportEventType::StageProgress,
        ExportStage::WriteCapsule,
        WRITE_STAGE_START,
        Some(0.0),
        format!("Writing {} frames to capsule", docs.len()),
        Some(json!({ "totalFrames": docs.len() })),
    )
    .await?;

    let written_frames = Arc::new(AtomicUsize::new(0));
    let write_done = Arc::new(AtomicBool::new(false));
    let hb_state = state.clone();
    let hb_job_id = job_id.to_string();
    let hb_written = Arc::clone(&written_frames);
    let hb_done = Arc::clone(&write_done);
    let write_heartbeat = tokio::spawn(async move {
        loop {
            time::sleep(HEARTBEAT_INTERVAL).await;
            if hb_done.load(Ordering::Relaxed) {
                break;
            }
            let written = hb_written.load(Ordering::Relaxed);
            let stage_progress = ((written as f64 / total_frames as f64) * 100.0).clamp(0.0, 99.5);
            let global_progress =
                lerp_stage_progress(stage_progress, WRITE_STAGE_START, WRITE_STAGE_END);
            let _ = append_job_event(
                &hb_state,
                &hb_job_id,
                ExportEventType::StageHeartbeat,
                ExportStage::WriteCapsule,
                global_progress,
                Some(stage_progress),
                format!("Writing capsule frames {written}/{total_frames}"),
                Some(json!({
                    "writtenFrames": written,
                    "totalFrames": total_frames
                })),
            )
            .await;
        }
    });

    let output_path_for_write = output_path.clone();
    let docs_for_write = docs.clone();
    let semantic_enabled = request.options.semantic_enabled;
    let embedding_config = if semantic_enabled {
        Some(state.config.embedding_runtime_config()?)
    } else {
        None
    };
    let written_for_write = Arc::clone(&written_frames);

    let write_result = tokio::task::spawn_blocking(move || {
        write_mv2(
            &output_path_for_write,
            &docs_for_write,
            semantic_enabled,
            embedding_config,
            move |written, _total| {
                written_for_write.store(written, Ordering::Relaxed);
            },
        )
    })
    .await;

    write_done.store(true, Ordering::Relaxed);
    let _ = write_heartbeat.await;

    match write_result {
        Ok(Ok(())) => {}
        Ok(Err(err)) => return Err(err),
        Err(err) => return Err(anyhow::anyhow!("MV2 writer task join error: {err}")),
    }

    info!(
        job_id = %job_id,
        output_path = %output_path.display(),
        "MV2 artifact write finished"
    );

    let _ = append_job_event(
        &state,
        job_id,
        ExportEventType::StageProgress,
        ExportStage::WriteCapsule,
        WRITE_STAGE_END,
        Some(100.0),
        "Capsule write complete",
        None,
    )
    .await?;

    if is_canceled(&state, job_id).await {
        delete_file_if_exists(&output_path).await?;
        return Ok(());
    }

    let _ = append_job_event(
        &state,
        job_id,
        ExportEventType::StageProgress,
        ExportStage::BuildSidecar,
        SIDECAR_STAGE_START,
        Some(0.0),
        "Building sidecar index",
        None,
    )
    .await?;

    let request_for_index = request.clone();
    let docs_for_index = docs.clone();
    let output_path_for_index = output_path.clone();
    let sidecar_job_id = job_id.to_string();

    let mut sidecar_task = tokio::task::spawn_blocking(move || {
        build_and_persist_from_request(&request_for_index, &docs_for_index, &output_path_for_index)
    });
    let sidecar_start = std::time::Instant::now();

    let sidecar_result = loop {
        tokio::select! {
            res = &mut sidecar_task => break res,
            _ = time::sleep(HEARTBEAT_INTERVAL) => {
                let elapsed = sidecar_start.elapsed().as_secs_f64();
                let stage_progress = (elapsed / 24.0 * 75.0).clamp(0.0, 75.0);
                let checkpoint = if stage_progress >= 50.0 {
                    "Persisting sidecar index"
                } else if stage_progress >= 25.0 {
                    "Deriving hotspot and process indexes"
                } else {
                    "Parsing capsule metadata"
                };
                let _ = append_job_event(
                    &state,
                    &sidecar_job_id,
                    ExportEventType::StageHeartbeat,
                    ExportStage::BuildSidecar,
                    lerp_stage_progress(stage_progress, SIDECAR_STAGE_START, SIDECAR_STAGE_END),
                    Some(stage_progress),
                    checkpoint,
                    Some(json!({ "checkpoint": checkpoint })),
                ).await;
            }
        }
    };

    let sidecar_message = match sidecar_result {
        Ok(Ok(index)) => {
            info!(
                job_id = %job_id,
                sidecar = %index.sidecar_path.display(),
                nodes = index.nodes.len(),
                edges = index.edges.len(),
                "Sidecar MCP index built"
            );
            "Sidecar index ready".to_string()
        }
        Ok(Err(err)) => {
            warn!(job_id = %job_id, "Failed to build sidecar MCP index: {err:#}");
            "Sidecar index skipped (non-blocking error)".to_string()
        }
        Err(err) => {
            warn!(job_id = %job_id, "Sidecar MCP index task join error: {err:#}");
            "Sidecar index skipped (worker join error)".to_string()
        }
    };

    let _ = append_job_event(
        &state,
        job_id,
        ExportEventType::StageProgress,
        ExportStage::BuildSidecar,
        SIDECAR_STAGE_END,
        Some(100.0),
        sidecar_message,
        None,
    )
    .await?;

    if is_canceled(&state, job_id).await {
        delete_file_if_exists(&output_path).await?;
        return Ok(());
    }

    let _ = append_job_event(
        &state,
        job_id,
        ExportEventType::StageProgress,
        ExportStage::Finalize,
        90.0,
        Some(20.0),
        "Finalizing artifact metadata",
        None,
    )
    .await?;

    let metadata = fs::metadata(&output_path)
        .await
        .with_context(|| format!("Failed to stat {}", output_path.display()))?;

    let now = Utc::now();
    let expires_at = now + ChronoDuration::seconds(state.config.retention_seconds as i64);

    let _ = append_job_event(
        &state,
        job_id,
        ExportEventType::StageProgress,
        ExportStage::Finalize,
        96.0,
        Some(70.0),
        "Preparing download artifact",
        None,
    )
    .await?;

    {
        let mut jobs = state.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            if matches!(job.status, JobState::Canceled) {
                delete_file_if_exists(&output_path).await?;
                return Ok(());
            }

            let artifact = ExportArtifact {
                file_name: file_name.clone(),
                download_url: format!("/v1/exports/{job_id}/download"),
                expires_at,
                size_bytes: metadata.len(),
            };

            job.status = JobState::Completed;
            job.progress = 100.0;
            job.current_stage = ExportStage::DownloadReady;
            job.stage_progress = 100.0;
            job.message = Some("Export completed".to_string());
            job.updated_at = now;
            job.artifact = Some(artifact);
            job.artifact_path = Some(output_path.clone());
            job.error = None;
            job.request = None;
        }
    }

    let _ = append_job_event(
        &state,
        job_id,
        ExportEventType::JobCompleted,
        ExportStage::DownloadReady,
        100.0,
        Some(100.0),
        "Export completed",
        Some(json!({
            "artifact": file_name,
            "sizeBytes": metadata.len()
        })),
    )
    .await?;

    info!(
        job_id = %job_id,
        artifact = %file_name,
        size_bytes = metadata.len(),
        "Export job completed"
    );

    Ok(())
}

fn runpod_client_from_state(state: &AppState) -> Result<RunpodClient> {
    let endpoint_id = state
        .config
        .runpod_endpoint_id
        .clone()
        .context("RUNPOD_ENDPOINT_ID must be set when backend mode is runpod_queue")?;
    let api_key = state
        .config
        .runpod_api_key
        .clone()
        .context("RUNPOD_API_KEY must be set when backend mode is runpod_queue")?;
    Ok(RunpodClient::new(
        state.config.runpod_api_base.clone(),
        endpoint_id,
        api_key,
    ))
}

async fn stage_request_payload(
    state: &AppState,
    job_id: &str,
    request: &ExportRequest,
) -> Result<(String, String, PathBuf)> {
    let payload_dir = state.config.staging_root.join("payloads");
    let output_dir = state.config.staging_root.join("outputs").join(job_id);
    fs::create_dir_all(&payload_dir).await.with_context(|| {
        format!(
            "Failed to create payload staging dir {}",
            payload_dir.display()
        )
    })?;
    fs::create_dir_all(&output_dir).await.with_context(|| {
        format!(
            "Failed to create output staging dir {}",
            output_dir.display()
        )
    })?;

    let payload_path = payload_dir.join(format!("{job_id}.json"));
    let payload_bytes =
        serde_json::to_vec(request).context("Failed to serialize export payload")?;
    fs::write(&payload_path, payload_bytes)
        .await
        .with_context(|| format!("Failed to write staged payload {}", payload_path.display()))?;

    let payload_ref = format!("file://{}", payload_path.display());
    let output_prefix = format!("file://{}", output_dir.display());
    Ok((payload_ref, output_prefix, output_dir))
}

fn resolve_runpod_artifact_path(output: &Value) -> Option<PathBuf> {
    output
        .get("artifactPath")
        .and_then(|v| v.as_str())
        .map(|value| {
            if let Some(stripped) = value.strip_prefix("file://") {
                PathBuf::from(stripped)
            } else {
                PathBuf::from(value)
            }
        })
}

async fn process_export_job_runpod(state: AppState, job_id: &str) -> Result<()> {
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
        job.progress = 2.0;
        job.current_stage = ExportStage::Transform;
        job.stage_progress = 0.0;
        job.message = Some("Preparing staged payload for Runpod".to_string());
        job.updated_at = Utc::now();
        job.error = None;
        job.request.clone().context("Missing request payload")?
    };

    let _ = append_job_event(
        &state,
        job_id,
        ExportEventType::JobStarted,
        ExportStage::Transform,
        2.0,
        Some(0.0),
        "Preparing staged payload for Runpod",
        None,
    )
    .await?;

    let (payload_ref, output_prefix, _output_dir) =
        stage_request_payload(&state, job_id, &request).await?;

    {
        let mut jobs = state.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            if let Some(meta) = job.metadata.as_mut() {
                meta.payload_ref = Some(payload_ref.clone());
            }
            job.request = None;
            job.updated_at = Utc::now();
        }
    }

    let client = runpod_client_from_state(&state)?;
    let run_request = RunpodRunRequest {
        input: RunpodJobInput {
            job_id: job_id.to_string(),
            payload_ref: payload_ref.clone(),
            output_prefix,
            embedding_mode: state.config.embedding_mode.as_str().to_string(),
            embedding_provider: state.config.embedding_provider.clone(),
            embedding_model: state.config.embedding_model.clone(),
            ollama_host: state.config.ollama_host.clone(),
        },
        policy: RunpodPolicy {
            execution_timeout: state.config.runpod_execution_timeout_ms,
            ttl: state.config.runpod_ttl_ms,
        },
    };

    let _ = append_job_event(
        &state,
        job_id,
        ExportEventType::StageProgress,
        ExportStage::WriteCapsule,
        8.0,
        Some(0.0),
        "Submitting Runpod queue job",
        None,
    )
    .await?;

    let submitted = client.submit_job(&run_request).await?;
    {
        let mut jobs = state.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            if let Some(meta) = job.metadata.as_mut() {
                meta.runpod_job_id = Some(submitted.id.clone());
            }
            job.message = Some("Runpod job submitted".to_string());
            job.updated_at = Utc::now();
        }
    }

    let _ = append_job_event(
        &state,
        job_id,
        ExportEventType::StageProgress,
        ExportStage::WriteCapsule,
        12.0,
        Some(5.0),
        format!("Runpod job submitted ({})", submitted.id),
        Some(json!({ "runpodJobId": submitted.id })),
    )
    .await?;

    let mut last_status = String::new();
    loop {
        if is_canceled(&state, job_id).await {
            if let Some(runpod_job_id) = {
                let jobs = state.jobs.read().await;
                jobs.get(job_id)
                    .and_then(|job| job.metadata.as_ref())
                    .and_then(|meta| meta.runpod_job_id.clone())
            } {
                let _ = client.cancel_job(&runpod_job_id).await;
            }
            return Ok(());
        }

        let runpod_job_id = {
            let jobs = state.jobs.read().await;
            jobs.get(job_id)
                .and_then(|job| job.metadata.as_ref())
                .and_then(|meta| meta.runpod_job_id.clone())
                .context("Missing runpod job id while polling")?
        };

        let status = client.get_status(&runpod_job_id).await?;
        if status.status != last_status {
            let (progress, stage_progress, msg) = match status.status.as_str() {
                "IN_QUEUE" => (15.0, 10.0, "Runpod job in queue"),
                "IN_PROGRESS" => (45.0, 55.0, "Runpod worker processing export"),
                "COMPLETED" => (95.0, 100.0, "Runpod worker completed export"),
                "CANCELLED" => (100.0, 100.0, "Runpod worker canceled export"),
                "FAILED" => (100.0, 100.0, "Runpod worker failed export"),
                "TIMED_OUT" => (100.0, 100.0, "Runpod worker timed out"),
                _ => (40.0, 40.0, "Runpod status update"),
            };
            let _ = append_job_event(
                &state,
                job_id,
                ExportEventType::StageHeartbeat,
                ExportStage::WriteCapsule,
                progress,
                Some(stage_progress),
                msg,
                Some(json!({
                    "runpodStatus": status.status,
                    "runpodJobId": runpod_job_id,
                })),
            )
            .await;
            last_status = status.status.clone();
        }

        match status.status.as_str() {
            "IN_QUEUE" | "IN_PROGRESS" => {
                time::sleep(Duration::from_secs(
                    state.config.runpod_poll_interval_seconds,
                ))
                .await;
            }
            "COMPLETED" => {
                let output = status
                    .output
                    .clone()
                    .context("Runpod completed without output payload")?;
                let artifact_path = resolve_runpod_artifact_path(&output)
                    .context("Runpod output missing artifactPath")?;
                let metadata = fs::metadata(&artifact_path)
                    .await
                    .with_context(|| format!("Failed to stat {}", artifact_path.display()))?;
                let file_name = output
                    .get("fileName")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string())
                    .or_else(|| {
                        artifact_path
                            .file_name()
                            .map(|f| f.to_string_lossy().to_string())
                    })
                    .unwrap_or_else(|| format!("{job_id}.mv2"));
                let now = Utc::now();
                let expires_at =
                    now + ChronoDuration::seconds(state.config.retention_seconds as i64);

                {
                    let mut jobs = state.jobs.write().await;
                    if let Some(job) = jobs.get_mut(job_id) {
                        if let Some(meta) = job.metadata.as_mut() {
                            meta.artifact_ref = output
                                .get("artifactRef")
                                .and_then(|v| v.as_str())
                                .map(|v| v.to_string());
                            meta.worker_metrics = Some(json!({
                                "backend": "runpod_queue",
                                "runpodStatus": status.status,
                                "embeddingMode": state.config.embedding_mode.as_str(),
                                "embeddingProvider": state.config.embedding_provider.as_str(),
                                "embeddingModel": state.config.embedding_model.as_str(),
                            }));
                        }
                        job.status = JobState::Completed;
                        job.progress = 100.0;
                        job.current_stage = ExportStage::DownloadReady;
                        job.stage_progress = 100.0;
                        job.message = Some("Export completed".to_string());
                        job.updated_at = now;
                        job.artifact = Some(ExportArtifact {
                            file_name: file_name.clone(),
                            download_url: format!("/v1/exports/{job_id}/download"),
                            expires_at,
                            size_bytes: metadata.len(),
                        });
                        job.artifact_path = Some(artifact_path.clone());
                        job.error = None;
                    }
                }

                let _ = append_job_event(
                    &state,
                    job_id,
                    ExportEventType::JobCompleted,
                    ExportStage::DownloadReady,
                    100.0,
                    Some(100.0),
                    "Export completed",
                    Some(json!({
                        "artifact": file_name,
                        "sizeBytes": metadata.len(),
                        "runpodJobId": runpod_job_id,
                    })),
                )
                .await?;
                return Ok(());
            }
            "CANCELLED" => {
                anyhow::bail!("Runpod job was cancelled");
            }
            "FAILED" => {
                anyhow::bail!(
                    "Runpod job failed: {}",
                    status
                        .error
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "unknown error".to_string())
                );
            }
            "TIMED_OUT" => {
                anyhow::bail!("Runpod job timed out");
            }
            other => {
                anyhow::bail!("Unknown Runpod status: {other}");
            }
        }
    }
}

async fn cleanup_expired_artifacts(state: &AppState) -> Result<()> {
    let now = Utc::now();
    let mut files_to_delete = Vec::new();
    let mut expired_job_ids = Vec::new();

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
                job.progress = 100.0;
                job.current_stage = ExportStage::Expired;
                job.stage_progress = 100.0;
                job.message = Some("Artifact expired and removed".to_string());
                job.artifact = None;
                job.artifact_path = None;
                job.events.clear();
                job.next_seq = 1;
                job.last_event_at = now;
                expired_job_ids.push(job.job_id.clone());
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

    if !expired_job_ids.is_empty() {
        let mut buses = state.event_buses.write().await;
        for job_id in expired_job_ids {
            buses.remove(&job_id);
        }
    }

    Ok(())
}
