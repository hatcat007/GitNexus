use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunpodPolicy {
    pub execution_timeout: u64,
    pub ttl: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunpodJobInput {
    pub job_id: String,
    pub payload_ref: String,
    pub output_prefix: String,
    pub embedding_mode: String,
    pub embedding_provider: String,
    pub embedding_model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ollama_host: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunpodRunRequest {
    pub input: RunpodJobInput,
    pub policy: RunpodPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunpodRunResponse {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunpodStatusResponse {
    pub id: String,
    pub status: String,
    #[serde(default)]
    pub output: Option<Value>,
    #[serde(default)]
    pub error: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct RunpodClient {
    base_url: String,
    endpoint_id: String,
    api_key: String,
}

impl RunpodClient {
    pub fn new(base_url: String, endpoint_id: String, api_key: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            endpoint_id,
            api_key,
        }
    }

    pub async fn submit_job(&self, payload: &RunpodRunRequest) -> Result<RunpodRunResponse> {
        let url = format!("{}/{}/run", self.base_url, self.endpoint_id);
        let body =
            serde_json::to_string(payload).context("Failed to encode Runpod /run payload")?;
        let parsed = execute_curl_json("POST", &url, &self.api_key, Some(body)).await?;
        serde_json::from_value::<RunpodRunResponse>(parsed)
            .context("Failed to decode Runpod /run response")
    }

    pub async fn get_status(&self, runpod_job_id: &str) -> Result<RunpodStatusResponse> {
        let url = format!(
            "{}/{}/status/{}",
            self.base_url, self.endpoint_id, runpod_job_id
        );
        let parsed = execute_curl_json("GET", &url, &self.api_key, None).await?;
        serde_json::from_value::<RunpodStatusResponse>(parsed)
            .context("Failed to decode Runpod /status response")
    }

    pub async fn cancel_job(&self, runpod_job_id: &str) -> Result<Value> {
        let url = format!(
            "{}/{}/cancel/{}",
            self.base_url, self.endpoint_id, runpod_job_id
        );
        let payload = execute_curl_json("POST", &url, &self.api_key, None)
            .await
            .unwrap_or_else(|_| {
                json!({
                    "id": runpod_job_id,
                    "status": "CANCEL_REQUESTED"
                })
            });
        Ok(payload)
    }
}

async fn execute_curl_json(
    method: &str,
    url: &str,
    api_key: &str,
    body: Option<String>,
) -> Result<Value> {
    let mut command = Command::new("curl");
    command
        .arg("-sS")
        .arg("-X")
        .arg(method)
        .arg(url)
        .arg("-H")
        .arg(format!("Authorization: Bearer {}", api_key))
        .arg("-H")
        .arg("Content-Type: application/json");

    if let Some(body) = body {
        command.arg("-d").arg(body);
    }

    let output = command
        .output()
        .await
        .with_context(|| format!("Failed to execute curl for {}", url))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Runpod curl request failed: {}", stderr.trim());
    }

    let stdout = String::from_utf8(output.stdout).context("Runpod response was not valid UTF-8")?;
    serde_json::from_str::<Value>(&stdout).with_context(|| {
        format!(
            "Failed to parse Runpod JSON response for {}: {}",
            url,
            stdout.trim()
        )
    })
}
