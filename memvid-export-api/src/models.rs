use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::VecDeque, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSourceDescriptor {
    pub r#type: String,
    pub base_name: String,
    pub display_name: String,
    pub url: Option<String>,
    pub branch: Option<String>,
    pub original_file_name: Option<String>,
    pub folder_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportOptions {
    pub semantic_enabled: bool,
    pub max_snippet_chars: usize,
    pub max_node_frames: usize,
    pub max_relation_frames: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeProperties {
    pub name: String,
    pub file_path: String,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
    pub language: Option<String>,
    pub is_exported: Option<bool>,
    pub heuristic_label: Option<String>,
    pub cohesion: Option<f64>,
    pub symbol_count: Option<usize>,
    pub keywords: Option<Vec<String>>,
    pub description: Option<String>,
    pub enriched_by: Option<String>,
    pub process_type: Option<String>,
    pub step_count: Option<usize>,
    pub communities: Option<Vec<String>>,
    pub entry_point_id: Option<String>,
    pub terminal_id: Option<String>,
    pub entry_point_score: Option<f64>,
    pub entry_point_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub properties: NodeProperties,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphRelationship {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub r#type: String,
    pub confidence: f64,
    pub reason: String,
    pub step: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub session_id: String,
    pub project_name: String,
    pub source: ExportSourceDescriptor,
    pub nodes: Vec<GraphNode>,
    pub relationships: Vec<GraphRelationship>,
    pub file_contents: std::collections::HashMap<String, String>,
    pub options: ExportOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobState {
    Queued,
    Running,
    Completed,
    Failed,
    Canceled,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportStage {
    Queued,
    Transform,
    FramePrep,
    WriteCapsule,
    BuildSidecar,
    Finalize,
    DownloadReady,
    Failed,
    Canceled,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportEventType {
    JobStarted,
    StageProgress,
    StageHeartbeat,
    JobCompleted,
    JobFailed,
    JobCanceled,
    JobExpired,
}

impl ExportEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::JobStarted => "job_started",
            Self::StageProgress => "stage_progress",
            Self::StageHeartbeat => "stage_heartbeat",
            Self::JobCompleted => "job_completed",
            Self::JobFailed => "job_failed",
            Self::JobCanceled => "job_canceled",
            Self::JobExpired => "job_expired",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportLogEvent {
    pub seq: u64,
    pub ts: DateTime<Utc>,
    pub job_id: String,
    #[serde(rename = "type")]
    pub event_type: ExportEventType,
    pub stage: ExportStage,
    pub progress: f64,
    pub stage_progress: Option<f64>,
    pub emoji: String,
    pub message: String,
    pub meta: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportArtifact {
    pub file_name: String,
    pub download_url: String,
    pub expires_at: DateTime<Utc>,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportErrorPayload {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportJobResponse {
    pub job_id: String,
    pub status: JobState,
    pub progress: f64,
    pub current_stage: ExportStage,
    pub stage_progress: f64,
    pub elapsed_ms: u64,
    pub last_event_seq: u64,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub artifact: Option<ExportArtifact>,
    pub error: Option<ExportErrorPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JobBackendMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportAcceptedResponse {
    pub job_id: String,
    pub status: JobState,
    pub progress: f64,
    pub current_stage: ExportStage,
    pub stage_progress: f64,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportEventsResponse {
    pub job_id: String,
    pub events: Vec<ExportLogEvent>,
    pub next_seq: u64,
    pub status_snapshot: ExportJobResponse,
}

#[derive(Debug, Clone)]
pub struct FrameDocument {
    pub title: String,
    pub label: String,
    pub text: String,
    pub uri: String,
    pub track: String,
    pub tags: Vec<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone)]
pub struct JobRecord {
    pub job_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: JobState,
    pub progress: f64,
    pub message: Option<String>,
    pub request: Option<ExportRequest>,
    pub artifact: Option<ExportArtifact>,
    pub error: Option<ExportErrorPayload>,
    pub artifact_path: Option<PathBuf>,
    pub events: VecDeque<ExportLogEvent>,
    pub next_seq: u64,
    pub current_stage: ExportStage,
    pub stage_progress: f64,
    pub last_event_at: DateTime<Utc>,
    pub metadata: Option<JobBackendMetadata>,
}

impl JobRecord {
    pub fn to_response(&self) -> ExportJobResponse {
        let elapsed = (Utc::now() - self.created_at).num_milliseconds().max(0) as u64;
        ExportJobResponse {
            job_id: self.job_id.clone(),
            status: self.status.clone(),
            progress: self.progress,
            current_stage: self.current_stage.clone(),
            stage_progress: self.stage_progress,
            elapsed_ms: elapsed,
            last_event_seq: self.next_seq.saturating_sub(1),
            message: self.message.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            artifact: self.artifact.clone(),
            error: self.error.clone(),
            metadata: self.metadata.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobBackendMetadata {
    pub backend: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runpod_job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_metrics: Option<Value>,
}
