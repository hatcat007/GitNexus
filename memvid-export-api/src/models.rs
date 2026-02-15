use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

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
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub artifact: Option<ExportArtifact>,
    pub error: Option<ExportErrorPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportAcceptedResponse {
    pub job_id: String,
    pub status: JobState,
    pub progress: f64,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
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
}

impl JobRecord {
    pub fn to_response(&self) -> ExportJobResponse {
        ExportJobResponse {
            job_id: self.job_id.clone(),
            status: self.status.clone(),
            progress: self.progress,
            message: self.message.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            artifact: self.artifact.clone(),
            error: self.error.clone(),
        }
    }
}
