use std::process::Command;

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingModeKind {
    ExternalApi,
    RunpodGpu,
}

impl EmbeddingModeKind {
    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "external_api" => Ok(Self::ExternalApi),
            "runpod_gpu" => Ok(Self::RunpodGpu),
            other => bail!(
                "Unsupported MEMVID_EMBEDDING_MODE `{other}`. Supported: external_api, runpod_gpu."
            ),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ExternalApi => "external_api",
            Self::RunpodGpu => "runpod_gpu",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingProviderKind {
    Nvidia,
    Openai,
    Voyage,
    Ollama,
}

impl EmbeddingProviderKind {
    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "nvidia" => Ok(Self::Nvidia),
            "openai" => Ok(Self::Openai),
            "voyage" | "voyageai" => Ok(Self::Voyage),
            "ollama" | "local" => Ok(Self::Ollama),
            other => bail!(
                "Unsupported MEMVID_EMBED_PROVIDER `{other}`. Supported: nvidia, openai, voyage/voyageai, ollama/local."
            ),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Nvidia => "nvidia",
            Self::Openai => "openai",
            Self::Voyage => "voyage",
            Self::Ollama => "ollama",
        }
    }
}

pub fn default_model_for_provider(provider: &str) -> Result<&'static str> {
    match EmbeddingProviderKind::parse(provider)? {
        EmbeddingProviderKind::Nvidia => Ok("nvidia/nv-embed-v1"),
        EmbeddingProviderKind::Openai => Ok("text-embedding-3-large"),
        EmbeddingProviderKind::Voyage => Ok("voyage-code-3"),
        EmbeddingProviderKind::Ollama => Ok("nomic-embed-text"),
    }
}

#[derive(Debug, Clone)]
pub struct EmbeddingRuntimeConfig {
    pub mode: EmbeddingModeKind,
    pub provider: EmbeddingProviderKind,
    pub model: String,
    pub nvidia_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub voyage_api_key: Option<String>,
    pub ollama_host: Option<String>,
    pub nvidia_embed_base_url: String,
    pub openai_embed_base_url: String,
    pub voyage_embed_base_url: String,
    pub voyage_input_type: String,
    pub voyage_output_dimension: Option<u16>,
    pub voyage_output_dtype: String,
    pub voyage_truncation: bool,
    pub request_timeout_seconds: u64,
}

impl EmbeddingRuntimeConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mode: &str,
        provider: &str,
        model: &str,
        nvidia_api_key: Option<String>,
        openai_api_key: Option<String>,
        voyage_api_key: Option<String>,
        ollama_host: Option<String>,
        nvidia_embed_base_url: String,
        openai_embed_base_url: String,
        voyage_embed_base_url: String,
        voyage_input_type: String,
        voyage_output_dimension: Option<u16>,
        voyage_output_dtype: String,
        voyage_truncation: bool,
        request_timeout_seconds: u64,
    ) -> Result<Self> {
        let mode = EmbeddingModeKind::parse(mode)?;
        let provider = EmbeddingProviderKind::parse(provider)?;
        let model = model.trim().to_string();
        if model.is_empty() {
            bail!("MEMVID_EMBED_MODEL must be non-empty.");
        }

        let config = Self {
            mode,
            provider,
            model,
            nvidia_api_key,
            openai_api_key,
            voyage_api_key,
            ollama_host,
            nvidia_embed_base_url: nvidia_embed_base_url.trim_end_matches('/').to_string(),
            openai_embed_base_url: openai_embed_base_url.trim_end_matches('/').to_string(),
            voyage_embed_base_url: voyage_embed_base_url.trim_end_matches('/').to_string(),
            voyage_input_type: voyage_input_type.trim().to_ascii_lowercase(),
            voyage_output_dimension,
            voyage_output_dtype: voyage_output_dtype.trim().to_ascii_lowercase(),
            voyage_truncation,
            request_timeout_seconds: request_timeout_seconds.max(1),
        };
        config.validate_runtime_requirements()?;
        Ok(config)
    }

    pub fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        self.validate_runtime_requirements()?;
        match self.provider {
            EmbeddingProviderKind::Nvidia => self.embed_openai_compatible(
                format!("{}/embeddings", self.nvidia_embed_base_url),
                self.nvidia_api_key
                    .as_deref()
                    .context("NVIDIA_API_KEY is required for provider=nvidia")?,
                text,
            ),
            EmbeddingProviderKind::Openai => self.embed_openai_compatible(
                format!("{}/embeddings", self.openai_embed_base_url),
                self.openai_api_key
                    .as_deref()
                    .context("OPENAI_API_KEY is required for provider=openai")?,
                text,
            ),
            EmbeddingProviderKind::Voyage => self.embed_voyage(
                format!("{}/embeddings", self.voyage_embed_base_url),
                self.voyage_api_key
                    .as_deref()
                    .context("VOYAGE_API_KEY is required for provider=voyage/voyageai")?,
                text,
            ),
            EmbeddingProviderKind::Ollama => self.embed_ollama(
                format!(
                    "{}/api/embeddings",
                    self.ollama_host
                        .as_deref()
                        .context("OLLAMA_HOST is required for provider=ollama/local")?
                        .trim_end_matches('/')
                ),
                text,
            ),
        }
    }

    fn validate_runtime_requirements(&self) -> Result<()> {
        match self.mode {
            EmbeddingModeKind::ExternalApi => match self.provider {
                EmbeddingProviderKind::Nvidia => {
                    if self
                        .nvidia_api_key
                        .as_deref()
                        .unwrap_or_default()
                        .trim()
                        .is_empty()
                    {
                        bail!(
                            "Embedding misconfiguration: provider=nvidia requires NVIDIA_API_KEY."
                        );
                    }
                }
                EmbeddingProviderKind::Openai => {
                    if self
                        .openai_api_key
                        .as_deref()
                        .unwrap_or_default()
                        .trim()
                        .is_empty()
                    {
                        bail!(
                            "Embedding misconfiguration: provider=openai requires OPENAI_API_KEY."
                        );
                    }
                }
                EmbeddingProviderKind::Voyage => {
                    if self
                        .voyage_api_key
                        .as_deref()
                        .unwrap_or_default()
                        .trim()
                        .is_empty()
                    {
                        bail!(
                            "Embedding misconfiguration: provider=voyage/voyageai requires VOYAGE_API_KEY."
                        );
                    }
                    if !matches!(self.voyage_input_type.as_str(), "document" | "query") {
                        bail!(
                            "Embedding misconfiguration: VOYAGE_INPUT_TYPE must be `document` or `query`."
                        );
                    }
                    if !matches!(
                        self.voyage_output_dtype.as_str(),
                        "float" | "int8" | "uint8" | "binary" | "ubinary"
                    ) {
                        bail!(
                            "Embedding misconfiguration: VOYAGE_OUTPUT_DTYPE must be one of float,int8,uint8,binary,ubinary."
                        );
                    }
                    if let Some(dim) = self.voyage_output_dimension {
                        if self.model.trim().eq_ignore_ascii_case("voyage-code-3")
                            && !matches!(dim, 256 | 512 | 1024 | 2048)
                        {
                            bail!(
                                "Embedding misconfiguration: voyage-code-3 supports VOYAGE_OUTPUT_DIMENSION values 256,512,1024,2048."
                            );
                        }
                    }
                }
                EmbeddingProviderKind::Ollama => {
                    bail!(
                        "Embedding misconfiguration: provider=ollama/local is not valid with mode=external_api."
                    );
                }
            },
            EmbeddingModeKind::RunpodGpu => {
                if self.provider != EmbeddingProviderKind::Ollama {
                    bail!(
                        "Embedding misconfiguration: mode=runpod_gpu currently supports provider=ollama/local only."
                    );
                }
                if self
                    .ollama_host
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
                {
                    bail!(
                        "Embedding misconfiguration: mode=runpod_gpu requires OLLAMA_HOST for provider=ollama/local."
                    );
                }
            }
        }

        if self.model.trim().is_empty() {
            bail!("Embedding misconfiguration: MEMVID_EMBED_MODEL is required.");
        }
        Ok(())
    }

    fn embed_openai_compatible(&self, url: String, api_key: &str, text: &str) -> Result<Vec<f32>> {
        let body = json!({
            "model": self.model,
            "input": [text]
        });
        let response = execute_curl_json(
            &url,
            Some(format!("Authorization: Bearer {}", api_key)),
            self.request_timeout_seconds,
            body.to_string(),
        )?;
        extract_embedding_from_response(&response, &url)
    }

    fn embed_ollama(&self, url: String, text: &str) -> Result<Vec<f32>> {
        let body = json!({
            "model": self.model,
            "prompt": text
        });
        let response =
            execute_curl_json(&url, None, self.request_timeout_seconds, body.to_string())?;
        extract_embedding_from_response(&response, &url)
    }

    fn embed_voyage(&self, url: String, api_key: &str, text: &str) -> Result<Vec<f32>> {
        let mut body = json!({
            "model": self.model,
            "input": [text],
            "input_type": self.voyage_input_type,
            "output_dtype": self.voyage_output_dtype,
            "truncation": self.voyage_truncation
        });
        if let Some(dim) = self.voyage_output_dimension {
            body["output_dimension"] = json!(dim);
        }

        let response = execute_curl_json(
            &url,
            Some(format!("Authorization: Bearer {}", api_key)),
            self.request_timeout_seconds,
            body.to_string(),
        )?;
        extract_embedding_from_response(&response, &url)
    }
}

fn execute_curl_json(
    url: &str,
    auth_header: Option<String>,
    timeout_seconds: u64,
    body: String,
) -> Result<Value> {
    let mut command = Command::new("curl");
    command
        .arg("-sS")
        .arg("--max-time")
        .arg(timeout_seconds.to_string())
        .arg("-X")
        .arg("POST")
        .arg(url)
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("-d")
        .arg(body);

    if let Some(header) = auth_header {
        command.arg("-H").arg(header);
    }

    let output = command
        .output()
        .with_context(|| format!("Failed to execute curl for embedding request to {url}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Embedding request failed: {}", stderr.trim());
    }

    let stdout =
        String::from_utf8(output.stdout).context("Embedding provider response was not UTF-8")?;
    let parsed = serde_json::from_str::<Value>(&stdout).with_context(|| {
        format!(
            "Embedding provider response was not valid JSON for {url}: {}",
            stdout.trim()
        )
    })?;

    if let Some(error) = parsed.get("error") {
        bail!("Embedding provider returned error: {error}");
    }

    Ok(parsed)
}

fn extract_embedding_from_response(response: &Value, url: &str) -> Result<Vec<f32>> {
    if let Some(arr) = response.get("embedding").and_then(Value::as_array) {
        return parse_embedding_array(arr, url);
    }

    if let Some(arr) = response
        .get("data")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|first| first.get("embedding"))
        .and_then(Value::as_array)
    {
        return parse_embedding_array(arr, url);
    }

    bail!("Embedding response from {url} did not contain an `embedding` vector.");
}

fn parse_embedding_array(items: &[Value], url: &str) -> Result<Vec<f32>> {
    if items.is_empty() {
        bail!("Embedding response from {url} returned an empty embedding vector.");
    }

    let mut embedding = Vec::with_capacity(items.len());
    for item in items {
        let value = item
            .as_f64()
            .context("Embedding vector contained a non-numeric value")?;
        embedding.push(value as f32);
    }
    Ok(embedding)
}
