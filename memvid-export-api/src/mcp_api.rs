use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, HeaderValue, Response, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::task;
use tracing::debug;
use uuid::Uuid;

use crate::{
    auth::extract_bearer_token,
    mcp_index::{
        build_from_capsule, load_from_sidecar, persist_to_sidecar, sidecar_path_for_capsule,
        CapsuleIndex, MCP_SCHEMA_VERSION,
    },
    models::JobState,
    AppState,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonRpcSuccess {
    jsonrpc: &'static str,
    id: Value,
    result: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonRpcFailure {
    jsonrpc: &'static str,
    id: Value,
    error: JsonRpcError,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonRpcError {
    code: i64,
    message: String,
    data: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolCallParams {
    name: String,
    #[serde(default)]
    arguments: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocatorArgs {
    #[serde(default)]
    job_id: Option<String>,
    #[serde(default)]
    capsule_path: Option<String>,
}

#[derive(Debug, Clone)]
struct ToolError {
    code: &'static str,
    message: String,
    retryable: bool,
    retry_after_ms: Option<u64>,
    http_status: StatusCode,
}

impl ToolError {
    fn invalid_argument(message: impl Into<String>) -> Self {
        Self {
            code: "INVALID_ARGUMENT",
            message: message.into(),
            retryable: false,
            retry_after_ms: None,
            http_status: StatusCode::BAD_REQUEST,
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: "NOT_FOUND",
            message: message.into(),
            retryable: false,
            retry_after_ms: None,
            http_status: StatusCode::NOT_FOUND,
        }
    }

    fn incompatible(message: impl Into<String>) -> Self {
        Self {
            code: "CAPSULE_INCOMPATIBLE",
            message: message.into(),
            retryable: false,
            retry_after_ms: None,
            http_status: StatusCode::BAD_REQUEST,
        }
    }

    fn timeout(message: impl Into<String>) -> Self {
        Self {
            code: "TIMEOUT",
            message: message.into(),
            retryable: true,
            retry_after_ms: Some(250),
            http_status: StatusCode::REQUEST_TIMEOUT,
        }
    }

    fn rate_limited(message: impl Into<String>, retry_after_ms: u64) -> Self {
        Self {
            code: "RATE_LIMITED",
            message: message.into(),
            retryable: true,
            retry_after_ms: Some(retry_after_ms),
            http_status: StatusCode::TOO_MANY_REQUESTS,
        }
    }

    fn truncated(message: impl Into<String>) -> Self {
        Self {
            code: "RESULT_TRUNCATED",
            message: message.into(),
            retryable: true,
            retry_after_ms: Some(0),
            http_status: StatusCode::OK,
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            code: "INTERNAL_ERROR",
            message: message.into(),
            retryable: true,
            retry_after_ms: Some(500),
            http_status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn to_data(&self, trace_id: &str) -> Value {
        json!({
            "code": self.code,
            "traceId": trace_id,
            "retryable": self.retryable,
            "retryAfterMs": self.retry_after_ms,
            "httpStatus": self.http_status.as_u16(),
        })
    }
}

#[derive(Debug, Default)]
pub struct QueryCache {
    capacity: usize,
    order: VecDeque<String>,
    values: HashMap<String, Value>,
}

impl QueryCache {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            order: VecDeque::new(),
            values: HashMap::new(),
        }
    }

    fn get(&mut self, key: &str) -> Option<Value> {
        let value = self.values.get(key).cloned();
        if value.is_some() {
            self.touch(key);
        }
        value
    }

    fn set(&mut self, key: String, value: Value) {
        if self.values.contains_key(&key) {
            self.values.insert(key.clone(), value);
            self.touch(&key);
            return;
        }

        self.values.insert(key.clone(), value);
        self.order.push_back(key);
        self.trim();
    }

    fn touch(&mut self, key: &str) {
        if let Some(pos) = self.order.iter().position(|k| k == key) {
            self.order.remove(pos);
            self.order.push_back(key.to_string());
        }
    }

    fn trim(&mut self) {
        while self.values.len() > self.capacity {
            if let Some(old) = self.order.pop_front() {
                self.values.remove(&old);
            } else {
                break;
            }
        }
    }
}

pub fn new_query_cache(capacity: usize) -> QueryCache {
    QueryCache::with_capacity(capacity)
}

#[derive(Debug, Clone)]
struct ToolContext {
    trace_id: String,
    start: Instant,
}

#[derive(Debug)]
struct RankedItem {
    score: f64,
    key: String,
    payload: Value,
}

#[derive(Debug)]
struct PaginatedResult {
    items: Vec<Value>,
    next_cursor: Option<String>,
    truncated: bool,
}

pub async fn mcp(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let request_id = request.id.clone().unwrap_or(Value::Null);

    if request.jsonrpc != "2.0" {
        return error_response(
            request_id,
            StatusCode::BAD_REQUEST,
            "INVALID_ARGUMENT",
            "jsonrpc must be 2.0",
            json!({"retryable": false}),
            None,
        );
    }

    let token = match extract_bearer_token(&headers) {
        Ok(token) => token,
        Err((status, payload)) => {
            return (status, payload).into_response();
        }
    };

    if token.trim() != state.config.api_key {
        return error_response(
            request_id,
            StatusCode::UNAUTHORIZED,
            "UNAUTHORIZED",
            "Invalid API key",
            json!({"retryable": false}),
            None,
        );
    }

    let rate = state.rate_limiter.check(token.trim()).await;
    if !rate.allowed {
        let retry_after = rate.headers.reset_seconds.saturating_mul(1000);
        let err = ToolError::rate_limited("Rate limit exceeded", retry_after);
        let err_data = err.to_data("rate-limit");
        return jsonrpc_error(
            request_id,
            -32029,
            err.message.clone(),
            err_data,
            err.http_status,
            Some(rate.headers),
        );
    }

    let response = match request.method.as_str() {
        "ping" => jsonrpc_ok(
            request_id,
            json!({
                "schemaVersion": MCP_SCHEMA_VERSION,
                "ok": true,
            }),
            StatusCode::OK,
            Some(rate.headers.clone()),
        ),
        "initialize" => jsonrpc_ok(
            request_id,
            json!({
                "schemaVersion": MCP_SCHEMA_VERSION,
                "server": {
                    "name": "gitnexus-mv2-mcp",
                    "version": "1.0.0"
                },
                "capabilities": {
                    "tools": true,
                    "streaming": false,
                    "pagination": "cursor"
                }
            }),
            StatusCode::OK,
            Some(rate.headers.clone()),
        ),
        "tools/list" => jsonrpc_ok(
            request_id,
            json!({
                "schemaVersion": MCP_SCHEMA_VERSION,
                "tools": tool_definitions(),
            }),
            StatusCode::OK,
            Some(rate.headers.clone()),
        ),
        "tools/call" => {
            let params = match request
                .params
                .as_ref()
                .and_then(|value| serde_json::from_value::<ToolCallParams>(value.clone()).ok())
            {
                Some(params) => params,
                None => {
                    return jsonrpc_error(
                        request_id,
                        -32602,
                        "Invalid tool call parameters",
                        json!({"code": "INVALID_ARGUMENT", "retryable": false}),
                        StatusCode::BAD_REQUEST,
                        Some(rate.headers.clone()),
                    )
                }
            };

            let ctx = ToolContext {
                trace_id: Uuid::new_v4().to_string(),
                start: Instant::now(),
            };

            let output = run_tool(&state, &ctx, &params.name, &params.arguments).await;
            match output {
                Ok((result, pagination, confidence)) => {
                    let elapsed = ctx.start.elapsed().as_millis();
                    let next_cursor = pagination.next_cursor.clone();
                    let truncated = pagination.truncated;
                    let returned = pagination.items.len();
                    let envelope = json!({
                        "schemaVersion": MCP_SCHEMA_VERSION,
                        "traceId": ctx.trace_id,
                        "tool": params.name,
                        "confidence": confidence,
                        "result": result,
                        "pagination": {
                            "nextCursor": next_cursor,
                            "truncated": truncated,
                            "returned": returned,
                        },
                        "timingMs": elapsed,
                    });

                    let bytes = serde_json::to_vec(&envelope).map(|v| v.len()).unwrap_or(0);
                    if bytes > state.config.mcp_response_budget_bytes {
                        let err = ToolError::truncated(format!(
                            "Response size {} bytes exceeds budget {} bytes. Reduce limit or use pagination.",
                            bytes, state.config.mcp_response_budget_bytes
                        ));
                        let err_data = err.to_data(&ctx.trace_id);
                        jsonrpc_error(
                            request_id,
                            -32010,
                            err.message.clone(),
                            err_data,
                            err.http_status,
                            Some(rate.headers.clone()),
                        )
                    } else {
                        if state.config.mcp_dev_log_payloads {
                            debug!(trace_id = %ctx.trace_id, tool = %params.name, response_bytes = bytes, "MCP tool response payload");
                        }
                        jsonrpc_ok(
                            request_id,
                            envelope,
                            StatusCode::OK,
                            Some(rate.headers.clone()),
                        )
                    }
                }
                Err(err) => {
                    let code = match err.code {
                        "INVALID_ARGUMENT" => -32602,
                        "NOT_FOUND" => -32004,
                        "CAPSULE_INCOMPATIBLE" => -32020,
                        "INDEX_BUILD_IN_PROGRESS" => -32021,
                        "RESULT_TRUNCATED" => -32010,
                        "RATE_LIMITED" => -32029,
                        "TIMEOUT" => -32008,
                        _ => -32603,
                    };
                    let err_data = err.to_data("tool-error");

                    jsonrpc_error(
                        request_id,
                        code,
                        err.message.clone(),
                        err_data,
                        err.http_status,
                        Some(rate.headers.clone()),
                    )
                }
            }
        }
        _ => jsonrpc_error(
            request_id,
            -32601,
            "Method not found",
            json!({"code": "INVALID_ARGUMENT", "retryable": false}),
            StatusCode::NOT_FOUND,
            Some(rate.headers.clone()),
        ),
    };

    if state.config.mcp_dev_log_payloads {
        debug!(method = %request.method, "MCP request handled");
    }

    response
}

async fn run_tool(
    state: &AppState,
    ctx: &ToolContext,
    tool: &str,
    args: &Value,
) -> Result<(Value, PaginatedResult, Value), ToolError> {
    let locator = parse_locator(args)?;
    let capsule_path = resolve_capsule_path(state, &locator).await?;
    let index = get_or_load_index(state, &capsule_path).await?;

    let cache_key = format!(
        "{}|{}|{}",
        capsule_path.display(),
        tool,
        serde_json::to_string(args).unwrap_or_else(|_| "{}".to_string())
    );

    if let Some(cached) = state.mcp_cache.lock().await.get(&cache_key) {
        let pagination = PaginatedResult {
            items: cached
                .get("result")
                .and_then(|v| v.get("items"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
            next_cursor: cached
                .get("pagination")
                .and_then(|v| v.get("nextCursor"))
                .and_then(Value::as_str)
                .map(ToString::to_string),
            truncated: cached
                .get("pagination")
                .and_then(|v| v.get("truncated"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
        };

        let confidence = cached
            .get("confidence")
            .cloned()
            .unwrap_or_else(|| confidence_block(0.9, vec!["cache_hit"], Vec::new()));

        return Ok((
            cached.get("result").cloned().unwrap_or_else(|| json!({})),
            pagination,
            confidence,
        ));
    }

    let (result, pagination, confidence) = match tool {
        "symbol_lookup" => tool_symbol_lookup(&index, args),
        "node_get" => tool_node_get(&index, args),
        "neighbors_get" => tool_neighbors_get(&index, args),
        "edge_get" => tool_edge_get(&index, args),
        "text_search" => tool_text_search(&index, args),
        "call_trace" => tool_call_trace(&index, args),
        "callers_of" => tool_callers_of(&index, args),
        "callees_of" => tool_callees_of(&index, args),
        "process_list" => tool_process_list(&index, args),
        "process_get" => tool_process_get(&index, args),
        "impact_analysis" => tool_impact_analysis(&index, args),
        "file_outline" => tool_file_outline(&index, args),
        "file_snippet" => tool_file_snippet(&index, args),
        "community_list" => tool_community_list(&index, args),
        "manifest_get" => tool_manifest_get(&index, args),
        "query_explain" => tool_query_explain(&index, args),
        _ => Err(ToolError::invalid_argument(format!(
            "Unsupported tool: {tool}"
        ))),
    }?;

    let cache_envelope = json!({
        "traceId": ctx.trace_id,
        "tool": tool,
        "confidence": confidence.clone(),
        "result": result.clone(),
        "pagination": {
            "nextCursor": pagination.next_cursor.clone(),
            "truncated": pagination.truncated,
            "returned": pagination.items.len(),
        }
    });
    state.mcp_cache.lock().await.set(cache_key, cache_envelope);

    Ok((result, pagination, confidence))
}

fn parse_locator(args: &Value) -> Result<LocatorArgs, ToolError> {
    let Some(locator_val) = args.get("locator") else {
        return Ok(LocatorArgs {
            job_id: None,
            capsule_path: None,
        });
    };

    serde_json::from_value(locator_val.clone())
        .map_err(|_| ToolError::invalid_argument("Invalid locator object"))
}

async fn resolve_capsule_path(
    state: &AppState,
    locator: &LocatorArgs,
) -> Result<PathBuf, ToolError> {
    if let Some(job_id) = &locator.job_id {
        let jobs = state.jobs.read().await;
        let Some(job) = jobs.get(job_id) else {
            return Err(ToolError::not_found(format!("Unknown jobId: {job_id}")));
        };

        let Some(path) = &job.artifact_path else {
            return Err(ToolError::not_found(format!(
                "Job {job_id} has no available artifact path"
            )));
        };

        if !path.exists() {
            return Err(ToolError::not_found(format!(
                "Artifact not found on disk for job {job_id}"
            )));
        }

        return Ok(path.clone());
    }

    if let Some(capsule_path) = &locator.capsule_path {
        let path = PathBuf::from(capsule_path);
        let resolved = if path.is_absolute() {
            path
        } else {
            state.config.export_root.join(path)
        };

        if !state.config.mcp_allow_external_capsules {
            let canonical_root = state
                .config
                .export_root
                .canonicalize()
                .unwrap_or_else(|_| state.config.export_root.clone());

            let canonical_candidate = resolved.canonicalize().unwrap_or_else(|_| resolved.clone());

            if !canonical_candidate.starts_with(&canonical_root) {
                return Err(ToolError::invalid_argument(
                    "capsulePath must be inside export root unless MEMVID_MCP_ALLOW_EXTERNAL_CAPSULES=true",
                ));
            }
        }

        if !resolved.exists() {
            return Err(ToolError::not_found(format!(
                "capsulePath does not exist: {}",
                resolved.display()
            )));
        }
        return Ok(resolved);
    }

    let jobs = state.jobs.read().await;
    let latest = jobs
        .values()
        .filter(|job| matches!(job.status, JobState::Completed))
        .max_by_key(|job| job.updated_at);

    let Some(job) = latest else {
        return Err(ToolError::not_found(
            "No completed exports found. Provide locator.jobId or locator.capsulePath",
        ));
    };

    let Some(path) = &job.artifact_path else {
        return Err(ToolError::not_found(
            "Latest completed export is missing artifact path",
        ));
    };

    if !path.exists() {
        return Err(ToolError::not_found(format!(
            "Latest artifact missing on disk: {}",
            path.display()
        )));
    }

    Ok(path.clone())
}

async fn get_or_load_index(
    state: &AppState,
    capsule_path: &Path,
) -> Result<Arc<CapsuleIndex>, ToolError> {
    let key = capsule_path.display().to_string();
    {
        let guard = state.mcp_indexes.read().await;
        if let Some(index) = guard.get(&key) {
            return Ok(index.clone());
        }
    }

    let capsule = capsule_path.to_path_buf();
    let loaded = task::spawn_blocking(move || -> anyhow::Result<CapsuleIndex> {
        let sidecar_path = sidecar_path_for_capsule(&capsule);
        if sidecar_path.exists() {
            return match load_from_sidecar(&capsule) {
                Ok(index) => Ok(index),
                Err(_) => {
                    let index = build_from_capsule(&capsule)?;
                    persist_to_sidecar(&index)?;
                    Ok(index)
                }
            };
        }

        let index = build_from_capsule(&capsule)?;
        persist_to_sidecar(&index)?;
        Ok(index)
    })
    .await
    .map_err(|_| ToolError::timeout("Timed out while loading capsule index"))?
    .map_err(|err| ToolError::incompatible(format!("Failed loading capsule index: {err:#}")))?;

    let index = Arc::new(loaded);
    let mut guard = state.mcp_indexes.write().await;
    guard.insert(key, index.clone());
    Ok(index)
}

fn parse_limit(args: &Value, default_limit: usize, max_limit: usize) -> usize {
    args.get("limit")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(default_limit)
        .clamp(1, max_limit)
}

fn parse_cursor(args: &Value) -> Option<String> {
    args.get("cursor")
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn normalize_text(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn compare_ranked(a: &RankedItem, b: &RankedItem) -> Ordering {
    b.score
        .partial_cmp(&a.score)
        .unwrap_or(Ordering::Equal)
        .then(a.key.cmp(&b.key))
}

fn decode_cursor(cursor: &str) -> Option<(f64, String)> {
    let (score, key) = cursor.split_once("::")?;
    Some((score.parse::<f64>().ok()?, key.to_string()))
}

fn encode_cursor(score: f64, key: &str) -> String {
    format!("{score:.6}::{key}")
}

fn paginate_ranked(
    mut rows: Vec<RankedItem>,
    limit: usize,
    cursor: Option<String>,
) -> PaginatedResult {
    rows.sort_by(compare_ranked);

    let start_idx = if let Some(cursor) = cursor {
        if let Some((cursor_score, cursor_key)) = decode_cursor(&cursor) {
            rows.iter()
                .position(|row| {
                    row.score < cursor_score
                        || ((row.score - cursor_score).abs() < 1e-9 && row.key > cursor_key)
                })
                .unwrap_or(rows.len())
        } else {
            0
        }
    } else {
        0
    };

    let end_idx = (start_idx + limit).min(rows.len());
    let selected = &rows[start_idx..end_idx];
    let items = selected
        .iter()
        .map(|row| row.payload.clone())
        .collect::<Vec<_>>();
    let next_cursor = if end_idx < rows.len() {
        rows.get(end_idx - 1)
            .map(|row| encode_cursor(row.score, &row.key))
    } else {
        None
    };

    PaginatedResult {
        items,
        next_cursor,
        truncated: end_idx < rows.len(),
    }
}

fn confidence_block(score: f64, factors: Vec<&str>, warnings: Vec<&str>) -> Value {
    let tier = if score >= 0.85 {
        "high"
    } else if score >= 0.60 {
        "medium"
    } else {
        "low"
    };

    json!({
        "score": (score * 1000.0).round() / 1000.0,
        "tier": tier,
        "factors": factors,
        "warnings": warnings,
    })
}

fn require_str(args: &Value, field: &str) -> Result<String, ToolError> {
    args.get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            ToolError::invalid_argument(format!("Missing required string field: {field}"))
        })
}

fn node_payload(node: &crate::mcp_index::NodeRecord) -> Value {
    json!({
        "id": node.id,
        "label": node.label,
        "name": node.name,
        "filePath": node.file_path,
        "startLine": node.start_line,
        "endLine": node.end_line,
        "language": node.language,
        "uri": node.uri,
    })
}

fn edge_payload(edge: &crate::mcp_index::EdgeRecord) -> Value {
    json!({
        "id": edge.id,
        "type": edge.relation_type,
        "sourceId": edge.source_id,
        "targetId": edge.target_id,
        "confidence": edge.confidence,
        "reason": edge.reason,
        "step": edge.step,
        "uri": edge.uri,
    })
}

fn tool_symbol_lookup(
    index: &CapsuleIndex,
    args: &Value,
) -> Result<(Value, PaginatedResult, Value), ToolError> {
    let query = require_str(args, "query")?;
    let norm = normalize_text(&query);
    if norm.is_empty() {
        return Err(ToolError::invalid_argument(
            "query cannot be empty after normalization",
        ));
    }

    let limit = parse_limit(args, 20, 100);
    let cursor = parse_cursor(args);

    let mut rows = Vec::new();
    for symbol in &index.symbols {
        if !symbol.symbol_norm.contains(&norm) {
            continue;
        }

        let score = if symbol.symbol_norm == norm {
            1.0
        } else if symbol.symbol_norm.starts_with(&norm) {
            0.92
        } else {
            0.78
        };

        let node_uri = index
            .node_by_id
            .get(&symbol.node_id)
            .and_then(|idx| index.nodes.get(*idx))
            .map(|node| node.uri.clone())
            .unwrap_or_default();

        rows.push(RankedItem {
            score,
            key: format!("{}::{}", symbol.symbol_norm, symbol.node_id),
            payload: json!({
                "symbol": symbol.symbol,
                "symbolNorm": symbol.symbol_norm,
                "nodeId": symbol.node_id,
                "filePath": symbol.file_path,
                "nodeLabel": symbol.node_label,
                "nodeUri": node_uri,
                "score": score,
            }),
        });
    }

    let pagination = paginate_ranked(rows, limit, cursor);
    let result = json!({ "items": pagination.items, "query": query });
    let score = if pagination.items.is_empty() {
        0.2
    } else {
        0.92
    };
    Ok((
        result,
        pagination,
        confidence_block(
            score,
            vec!["symbol_norm_match", "deterministic_sort"],
            Vec::new(),
        ),
    ))
}

fn tool_node_get(
    index: &CapsuleIndex,
    args: &Value,
) -> Result<(Value, PaginatedResult, Value), ToolError> {
    let node_id = require_str(args, "nodeId")?;
    let Some(idx) = index.node_by_id.get(&node_id) else {
        return Err(ToolError::not_found(format!("nodeId not found: {node_id}")));
    };

    let node = index
        .nodes
        .get(*idx)
        .ok_or_else(|| ToolError::internal("node index corrupted"))?;
    let out_edges = index
        .edges_out_by_node
        .get(&node.id)
        .map(|v| v.len())
        .unwrap_or(0);
    let in_edges = index
        .edges_in_by_node
        .get(&node.id)
        .map(|v| v.len())
        .unwrap_or(0);

    let result = json!({
        "node": node_payload(node),
        "degree": {
            "out": out_edges,
            "in": in_edges,
            "total": out_edges + in_edges,
        },
        "metadata": node.metadata,
    });

    Ok((
        result,
        PaginatedResult {
            items: vec![node_payload(node)],
            next_cursor: None,
            truncated: false,
        },
        confidence_block(0.99, vec!["exact_id_match"], Vec::new()),
    ))
}

fn tool_neighbors_get(
    index: &CapsuleIndex,
    args: &Value,
) -> Result<(Value, PaginatedResult, Value), ToolError> {
    let node_id = require_str(args, "nodeId")?;
    if !index.node_by_id.contains_key(&node_id) {
        return Err(ToolError::not_found(format!("nodeId not found: {node_id}")));
    }

    let direction = args
        .get("direction")
        .and_then(Value::as_str)
        .unwrap_or("both");

    let filter_types: HashSet<String> = args
        .get("relationTypes")
        .and_then(Value::as_array)
        .map(|types| {
            types
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();

    let limit = parse_limit(args, 25, 150);
    let cursor = parse_cursor(args);

    let mut candidate_edges = Vec::new();
    if matches!(direction, "out" | "both") {
        if let Some(edges) = index.edges_out_by_node.get(&node_id) {
            candidate_edges.extend(edges.iter().copied());
        }
    }
    if matches!(direction, "in" | "both") {
        if let Some(edges) = index.edges_in_by_node.get(&node_id) {
            candidate_edges.extend(edges.iter().copied());
        }
    }

    let mut rows = Vec::new();
    for edge_idx in candidate_edges {
        let Some(edge) = index.edges.get(edge_idx) else {
            continue;
        };
        if !filter_types.is_empty() && !filter_types.contains(&edge.relation_type) {
            continue;
        }

        let neighbor_id = if edge.source_id == node_id {
            edge.target_id.clone()
        } else {
            edge.source_id.clone()
        };

        let neighbor = index
            .node_by_id
            .get(&neighbor_id)
            .and_then(|idx| index.nodes.get(*idx));

        let score = 0.70 + edge.confidence * 0.30;
        rows.push(RankedItem {
            score,
            key: format!("{}::{}", edge.id, neighbor_id),
            payload: json!({
                "edge": edge_payload(edge),
                "neighbor": neighbor.map(node_payload),
                "direction": if edge.source_id == node_id { "out" } else { "in" },
                "score": score,
            }),
        });
    }

    let pagination = paginate_ranked(rows, limit, cursor);
    let result = json!({
        "nodeId": node_id,
        "direction": direction,
        "items": pagination.items,
    });

    Ok((
        result,
        pagination,
        confidence_block(0.9, vec!["graph_adjacency", "edge_confidence"], Vec::new()),
    ))
}

fn tool_edge_get(
    index: &CapsuleIndex,
    args: &Value,
) -> Result<(Value, PaginatedResult, Value), ToolError> {
    let edge_id = require_str(args, "edgeId")?;
    let Some(idx) = index.edge_by_id.get(&edge_id) else {
        return Err(ToolError::not_found(format!("edgeId not found: {edge_id}")));
    };

    let edge = index
        .edges
        .get(*idx)
        .ok_or_else(|| ToolError::internal("edge index corrupted"))?;
    let source = index
        .node_by_id
        .get(&edge.source_id)
        .and_then(|idx| index.nodes.get(*idx))
        .map(node_payload);
    let target = index
        .node_by_id
        .get(&edge.target_id)
        .and_then(|idx| index.nodes.get(*idx))
        .map(node_payload);

    let result = json!({
        "edge": edge_payload(edge),
        "source": source,
        "target": target,
        "metadata": edge.metadata,
    });

    Ok((
        result,
        PaginatedResult {
            items: vec![edge_payload(edge)],
            next_cursor: None,
            truncated: false,
        },
        confidence_block(
            0.98,
            vec!["exact_id_match", "graph_edge_lookup"],
            Vec::new(),
        ),
    ))
}

fn lexical_score(text: &str, normalized_terms: &[String]) -> f64 {
    if normalized_terms.is_empty() {
        return 0.0;
    }

    let normalized_text = normalize_text(text);
    if normalized_text.is_empty() {
        return 0.0;
    }

    let mut matches = 0usize;
    for term in normalized_terms {
        if normalized_text.contains(term) {
            matches += 1;
        }
    }

    if matches == 0 {
        0.0
    } else {
        matches as f64 / normalized_terms.len() as f64
    }
}

fn tool_text_search(
    index: &CapsuleIndex,
    args: &Value,
) -> Result<(Value, PaginatedResult, Value), ToolError> {
    let query = require_str(args, "query")?;
    let scope = args
        .get("scope")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let limit = parse_limit(args, 25, 150);
    let cursor = parse_cursor(args);

    let normalized_terms = normalize_text(&query)
        .split_whitespace()
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    if normalized_terms.is_empty() {
        return Err(ToolError::invalid_argument(
            "query contains no searchable terms",
        ));
    }

    let mut rows = Vec::new();
    for entry in &index.fulltext {
        if let Some(scope) = &scope {
            let scope_lower = scope.to_ascii_lowercase();
            let uri_ok = entry.uri.to_ascii_lowercase().contains(&scope_lower);
            let track_ok = entry.track.to_ascii_lowercase().contains(&scope_lower);
            if !uri_ok && !track_ok {
                continue;
            }
        }

        let score = lexical_score(&entry.text, &normalized_terms);
        if score <= 0.0 {
            continue;
        }

        let preview = if entry.text.chars().count() > 260 {
            format!("{}...", entry.text.chars().take(260).collect::<String>())
        } else {
            entry.text.clone()
        };

        rows.push(RankedItem {
            score,
            key: format!("{}::{}", entry.uri, entry.ref_id),
            payload: json!({
                "refKind": entry.ref_kind,
                "refId": entry.ref_id,
                "uri": entry.uri,
                "track": entry.track,
                "score": score,
                "preview": preview,
            }),
        });
    }

    let pagination = paginate_ranked(rows, limit, cursor);
    let result = json!({
        "query": query,
        "scope": scope,
        "items": pagination.items,
        "semanticUsed": false,
    });

    let confidence_score = if pagination.items.is_empty() {
        0.3
    } else {
        0.86
    };
    Ok((
        result,
        pagination,
        confidence_block(
            confidence_score,
            vec![
                "lexical_match",
                "deterministic_scoring",
                "semantic_fallback_disabled",
            ],
            Vec::new(),
        ),
    ))
}

fn tool_call_trace(
    index: &CapsuleIndex,
    args: &Value,
) -> Result<(Value, PaginatedResult, Value), ToolError> {
    let from_node = require_str(args, "fromNodeId")?;
    let to_node = args
        .get("toNodeId")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let max_depth = args
        .get("maxDepth")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(4)
        .clamp(1, 10);
    let limit_paths = args
        .get("limitPaths")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(3)
        .clamp(1, 20);

    if !index.node_by_id.contains_key(&from_node) {
        return Err(ToolError::not_found(format!(
            "fromNodeId not found: {from_node}"
        )));
    }

    if let Some(target) = &to_node {
        if !index.node_by_id.contains_key(target) {
            return Err(ToolError::not_found(format!(
                "toNodeId not found: {target}"
            )));
        }
    }

    let mut paths: Vec<Vec<String>> = Vec::new();
    let mut queue: VecDeque<Vec<String>> = VecDeque::new();
    queue.push_back(vec![from_node.clone()]);

    while let Some(path) = queue.pop_front() {
        if paths.len() >= limit_paths {
            break;
        }

        let Some(last) = path.last().cloned() else {
            continue;
        };

        if let Some(target) = &to_node {
            if &last == target && path.len() > 1 {
                paths.push(path.clone());
                continue;
            }
        } else if path.len() > 1 {
            paths.push(path.clone());
            continue;
        }

        if path.len() > max_depth {
            continue;
        }

        let Some(out_edges) = index.edges_out_by_node.get(&last) else {
            continue;
        };

        for edge_idx in out_edges {
            let Some(edge) = index.edges.get(*edge_idx) else {
                continue;
            };
            if edge.relation_type != "CALLS" {
                continue;
            }
            let next = edge.target_id.clone();
            if path.contains(&next) {
                continue;
            }
            let mut new_path = path.clone();
            new_path.push(next);
            queue.push_back(new_path);
        }
    }

    let rendered = paths
        .iter()
        .map(|path| {
            let nodes = path
                .iter()
                .map(|id| {
                    index
                        .node_by_id
                        .get(id)
                        .and_then(|idx| index.nodes.get(*idx))
                        .map(node_payload)
                        .unwrap_or_else(|| json!({ "id": id }))
                })
                .collect::<Vec<_>>();
            json!({ "nodeIds": path, "nodes": nodes })
        })
        .collect::<Vec<_>>();

    let result = json!({
        "fromNodeId": from_node,
        "toNodeId": to_node,
        "maxDepth": max_depth,
        "paths": rendered.clone(),
        "pathCount": paths.len(),
    });

    Ok((
        result,
        PaginatedResult {
            items: rendered,
            next_cursor: None,
            truncated: false,
        },
        confidence_block(
            if paths.is_empty() { 0.45 } else { 0.88 },
            vec!["graph_call_edges", "breadth_first_search"],
            if paths.is_empty() {
                vec!["no_path_within_depth"]
            } else {
                Vec::new()
            },
        ),
    ))
}

fn callers_or_callees(
    index: &CapsuleIndex,
    args: &Value,
    incoming: bool,
) -> Result<(Value, PaginatedResult, Value), ToolError> {
    let node_id = require_str(args, "nodeId")?;
    if !index.node_by_id.contains_key(&node_id) {
        return Err(ToolError::not_found(format!("nodeId not found: {node_id}")));
    }

    let limit = parse_limit(args, 25, 150);
    let cursor = parse_cursor(args);

    let edges = if incoming {
        index.edges_in_by_node.get(&node_id)
    } else {
        index.edges_out_by_node.get(&node_id)
    };

    let mut rows = Vec::new();
    for edge_idx in edges.into_iter().flatten() {
        let Some(edge) = index.edges.get(*edge_idx) else {
            continue;
        };
        if edge.relation_type != "CALLS" {
            continue;
        }

        let other_id = if incoming {
            edge.source_id.clone()
        } else {
            edge.target_id.clone()
        };

        let other = index
            .node_by_id
            .get(&other_id)
            .and_then(|idx| index.nodes.get(*idx))
            .map(node_payload);

        rows.push(RankedItem {
            score: 0.7 + edge.confidence * 0.3,
            key: format!("{}::{}", edge.id, other_id),
            payload: json!({
                "edge": edge_payload(edge),
                "node": other,
            }),
        });
    }

    let pagination = paginate_ranked(rows, limit, cursor);
    let result = json!({
        "nodeId": node_id,
        "items": pagination.items,
    });

    Ok((
        result,
        pagination,
        confidence_block(0.9, vec!["graph_call_edges", "edge_confidence"], Vec::new()),
    ))
}

fn tool_callers_of(
    index: &CapsuleIndex,
    args: &Value,
) -> Result<(Value, PaginatedResult, Value), ToolError> {
    callers_or_callees(index, args, true)
}

fn tool_callees_of(
    index: &CapsuleIndex,
    args: &Value,
) -> Result<(Value, PaginatedResult, Value), ToolError> {
    callers_or_callees(index, args, false)
}

fn tool_process_list(
    index: &CapsuleIndex,
    args: &Value,
) -> Result<(Value, PaginatedResult, Value), ToolError> {
    let limit = parse_limit(args, 20, 100);
    let cursor = parse_cursor(args);

    let process_idxs = index
        .nodes_by_label
        .get("Process")
        .cloned()
        .unwrap_or_default();
    let mut rows = Vec::new();
    for idx in process_idxs {
        let Some(node) = index.nodes.get(idx) else {
            continue;
        };

        let steps_count = index
            .process_step_by_process
            .get(&node.id)
            .map(|v| v.len())
            .unwrap_or(0);

        let score = 0.6 + ((steps_count as f64).min(20.0) / 50.0);
        rows.push(RankedItem {
            score,
            key: format!("{}::{}", node.name, node.id),
            payload: json!({
                "processId": node.id,
                "name": node.name,
                "uri": node.uri,
                "stepsCount": steps_count,
            }),
        });
    }

    let pagination = paginate_ranked(rows, limit, cursor);
    let result = json!({ "items": pagination.items });

    Ok((
        result,
        pagination,
        confidence_block(0.92, vec!["process_nodes", "step_index"], Vec::new()),
    ))
}

fn tool_process_get(
    index: &CapsuleIndex,
    args: &Value,
) -> Result<(Value, PaginatedResult, Value), ToolError> {
    let process_id = require_str(args, "processId")?;
    let Some(node_idx) = index.node_by_id.get(&process_id) else {
        return Err(ToolError::not_found(format!(
            "processId not found: {process_id}"
        )));
    };
    let node = index
        .nodes
        .get(*node_idx)
        .ok_or_else(|| ToolError::internal("process node lookup failed"))?;

    if node.label != "Process" {
        return Err(ToolError::invalid_argument(format!(
            "nodeId is not a Process node: {process_id}"
        )));
    }

    let step_idxs = index
        .process_step_by_process
        .get(&process_id)
        .cloned()
        .unwrap_or_default();

    let mut steps = Vec::new();
    for step_idx in step_idxs {
        if let Some(step) = index.process_steps.get(step_idx) {
            let function_node = index
                .node_by_id
                .get(&step.function_id)
                .and_then(|idx| index.nodes.get(*idx))
                .map(node_payload);
            steps.push(json!({
                "step": step.step,
                "functionId": step.function_id,
                "function": function_node,
                "relationUri": step.relation_uri,
            }));
        }
    }

    let result = json!({
        "process": node_payload(node),
        "steps": steps.clone(),
    });

    Ok((
        result,
        PaginatedResult {
            items: steps,
            next_cursor: None,
            truncated: false,
        },
        confidence_block(
            0.94,
            vec!["process_step_index", "exact_process_id"],
            Vec::new(),
        ),
    ))
}

fn tool_impact_analysis(
    index: &CapsuleIndex,
    args: &Value,
) -> Result<(Value, PaginatedResult, Value), ToolError> {
    let node_id = require_str(args, "nodeId")?;
    if !index.node_by_id.contains_key(&node_id) {
        return Err(ToolError::not_found(format!("nodeId not found: {node_id}")));
    }

    let max_depth = args
        .get("maxDepth")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(3)
        .clamp(1, 8);

    let mut visited = HashSet::new();
    let mut frontier = vec![node_id.clone()];
    visited.insert(node_id.clone());

    for _ in 0..max_depth {
        let mut next = Vec::new();
        for current in frontier {
            if let Some(out) = index.edges_out_by_node.get(&current) {
                for edge_idx in out {
                    if let Some(edge) = index.edges.get(*edge_idx) {
                        if visited.insert(edge.target_id.clone()) {
                            next.push(edge.target_id.clone());
                        }
                    }
                }
            }
            if let Some(input) = index.edges_in_by_node.get(&current) {
                for edge_idx in input {
                    if let Some(edge) = index.edges.get(*edge_idx) {
                        if visited.insert(edge.source_id.clone()) {
                            next.push(edge.source_id.clone());
                        }
                    }
                }
            }
        }

        if next.is_empty() {
            break;
        }
        frontier = next;
    }

    let mut impacted_nodes = Vec::new();
    for id in &visited {
        if let Some(idx) = index.node_by_id.get(id) {
            if let Some(node) = index.nodes.get(*idx) {
                impacted_nodes.push(node_payload(node));
            }
        }
    }

    impacted_nodes.sort_by(|a, b| {
        let an = a.get("name").and_then(Value::as_str).unwrap_or_default();
        let bn = b.get("name").and_then(Value::as_str).unwrap_or_default();
        an.cmp(bn)
    });

    let impacted_edges = index
        .edges
        .iter()
        .filter(|edge| visited.contains(&edge.source_id) || visited.contains(&edge.target_id))
        .count();

    let result = json!({
        "rootNodeId": node_id,
        "maxDepth": max_depth,
        "impactedNodeCount": impacted_nodes.len(),
        "impactedEdgeCount": impacted_edges,
        "impactedNodes": impacted_nodes.clone(),
        "hotspots": index.hotspots.iter().take(10).map(|h| json!({
            "filePath": h.file_path,
            "callsCount": h.calls_count,
            "nodeCount": h.node_count,
            "score": h.score,
        })).collect::<Vec<_>>(),
    });

    Ok((
        result,
        PaginatedResult {
            items: impacted_nodes,
            next_cursor: None,
            truncated: false,
        },
        confidence_block(
            0.87,
            vec!["graph_reachability", "deterministic_bfs"],
            Vec::new(),
        ),
    ))
}

fn normalize_path_like(input: &str) -> String {
    input
        .replace('\\', "/")
        .trim()
        .trim_matches('/')
        .to_string()
}

fn tool_file_outline(
    index: &CapsuleIndex,
    args: &Value,
) -> Result<(Value, PaginatedResult, Value), ToolError> {
    let file_path = require_str(args, "filePath")?;
    let normalized = normalize_path_like(&file_path);

    let matching_file = index
        .nodes_by_file
        .keys()
        .find(|candidate| {
            let c = normalize_path_like(candidate);
            c == normalized || c.ends_with(&normalized)
        })
        .cloned()
        .ok_or_else(|| {
            ToolError::not_found(format!("No indexed nodes found for filePath: {file_path}"))
        })?;

    let idxs = index
        .nodes_by_file
        .get(&matching_file)
        .cloned()
        .unwrap_or_default();

    let mut symbols = idxs
        .iter()
        .filter_map(|idx| index.nodes.get(*idx))
        .filter(|node| {
            matches!(
                node.label.as_str(),
                "Function"
                    | "Method"
                    | "Class"
                    | "Interface"
                    | "Type"
                    | "Enum"
                    | "Variable"
                    | "File"
            )
        })
        .map(|node| {
            json!({
                "id": node.id,
                "label": node.label,
                "name": node.name,
                "startLine": node.start_line,
                "endLine": node.end_line,
                "uri": node.uri,
            })
        })
        .collect::<Vec<_>>();

    symbols.sort_by(|a, b| {
        let al = a.get("startLine").and_then(Value::as_u64).unwrap_or(0);
        let bl = b.get("startLine").and_then(Value::as_u64).unwrap_or(0);
        al.cmp(&bl).then(
            a.get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .cmp(b.get("name").and_then(Value::as_str).unwrap_or_default()),
        )
    });

    let result = json!({
        "filePath": matching_file,
        "symbols": symbols.clone(),
        "symbolCount": symbols.len(),
    });

    Ok((
        result,
        PaginatedResult {
            items: symbols,
            next_cursor: None,
            truncated: false,
        },
        confidence_block(0.95, vec!["file_index", "line_sorted_symbols"], Vec::new()),
    ))
}

fn tool_file_snippet(
    index: &CapsuleIndex,
    args: &Value,
) -> Result<(Value, PaginatedResult, Value), ToolError> {
    let max_chars = args
        .get("maxChars")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(1400)
        .clamp(80, 8000);

    let node = if let Some(node_id) = args.get("nodeId").and_then(Value::as_str) {
        let idx = index
            .node_by_id
            .get(node_id)
            .ok_or_else(|| ToolError::not_found(format!("nodeId not found: {node_id}")))?;
        index.nodes.get(*idx)
    } else {
        let file_path = require_str(args, "filePath")?;
        let normalized = normalize_path_like(&file_path);
        let best = index
            .nodes
            .iter()
            .find(|node| {
                let fp = normalize_path_like(&node.file_path);
                fp == normalized || fp.ends_with(&normalized)
            })
            .or_else(|| {
                index.nodes.iter().find(|node| {
                    node.label == "File"
                        && (normalize_path_like(&node.file_path) == normalized
                            || normalize_path_like(&node.file_path).ends_with(&normalized))
                })
            });

        best
    }
    .ok_or_else(|| ToolError::not_found("Could not resolve node/file for snippet"))?;

    let snippet = if node.search_text.chars().count() > max_chars {
        format!(
            "{}\n...[truncated]",
            node.search_text.chars().take(max_chars).collect::<String>()
        )
    } else {
        node.search_text.clone()
    };

    let result = json!({
        "node": node_payload(node),
        "snippet": snippet,
        "maxChars": max_chars,
    });

    Ok((
        result,
        PaginatedResult {
            items: vec![json!({"nodeId": node.id, "snippetChars": snippet.chars().count()})],
            next_cursor: None,
            truncated: false,
        },
        confidence_block(
            0.93,
            vec!["indexed_search_text", "bounded_payload"],
            Vec::new(),
        ),
    ))
}

fn tool_community_list(
    index: &CapsuleIndex,
    args: &Value,
) -> Result<(Value, PaginatedResult, Value), ToolError> {
    let limit = parse_limit(args, 20, 100);
    let cursor = parse_cursor(args);

    let community_idxs = index
        .nodes_by_label
        .get("Community")
        .cloned()
        .unwrap_or_default();

    let mut rows = Vec::new();
    for idx in community_idxs {
        let Some(node) = index.nodes.get(idx) else {
            continue;
        };
        let members = index
            .community_membership
            .iter()
            .filter(|m| m.community_id == node.id)
            .count();
        let score = 0.65 + (members as f64).min(30.0) / 100.0;
        rows.push(RankedItem {
            score,
            key: format!("{}::{}", node.name, node.id),
            payload: json!({
                "communityId": node.id,
                "name": node.name,
                "uri": node.uri,
                "members": members,
            }),
        });
    }

    let pagination = paginate_ranked(rows, limit, cursor);
    let result = json!({ "items": pagination.items });

    Ok((
        result,
        pagination,
        confidence_block(0.9, vec!["community_nodes", "membership_index"], Vec::new()),
    ))
}

fn tool_manifest_get(
    index: &CapsuleIndex,
    _args: &Value,
) -> Result<(Value, PaginatedResult, Value), ToolError> {
    let result = json!({
        "schemaVersion": MCP_SCHEMA_VERSION,
        "indexSchemaVersion": index.schema_version,
        "generatedAt": index.generated_at,
        "capsulePath": index.capsule_path,
        "sidecarPath": index.sidecar_path,
        "manifest": index.manifest,
        "capsuleCapabilities": index.capabilities,
        "counts": {
            "nodes": index.nodes.len(),
            "edges": index.edges.len(),
            "processSteps": index.process_steps.len(),
            "fulltextEntries": index.fulltext.len(),
            "symbols": index.symbols.len(),
            "hotspots": index.hotspots.len(),
        }
    });

    Ok((
        result,
        PaginatedResult {
            items: vec![json!({"kind": "manifest"})],
            next_cursor: None,
            truncated: false,
        },
        confidence_block(0.99, vec!["manifest_frame", "index_metadata"], Vec::new()),
    ))
}

fn tool_query_explain(
    index: &CapsuleIndex,
    args: &Value,
) -> Result<(Value, PaginatedResult, Value), ToolError> {
    let task = args
        .get("task")
        .and_then(Value::as_str)
        .unwrap_or("general")
        .to_ascii_lowercase();
    let query = args
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or_default();

    let recommended_tools = if task.contains("debug") || task.contains("root") {
        vec![
            "text_search",
            "symbol_lookup",
            "call_trace",
            "impact_analysis",
            "file_snippet",
        ]
    } else if task.contains("impact") || task.contains("change") {
        vec![
            "symbol_lookup",
            "node_get",
            "neighbors_get",
            "impact_analysis",
            "callers_of",
            "callees_of",
        ]
    } else if task.contains("arch") || task.contains("subsystem") {
        vec![
            "community_list",
            "process_list",
            "process_get",
            "neighbors_get",
            "manifest_get",
        ]
    } else {
        vec![
            "text_search",
            "symbol_lookup",
            "node_get",
            "neighbors_get",
            "file_outline",
        ]
    };

    let result = json!({
        "task": task,
        "query": query,
        "retrievalLadder": [
            "graph_exact",
            "lexical_search",
            "graph_expansion_rerank",
            "semantic_fallback_if_low_confidence"
        ],
        "rankingSignals": [
            "graph_structural_confidence",
            "lexical_relevance",
            "hotspot_locality",
            "semantic_fallback"
        ],
        "recommendedToolSequence": recommended_tools,
        "capsuleStats": {
            "nodes": index.nodes.len(),
            "edges": index.edges.len(),
            "hotspots": index.hotspots.len(),
        }
    });

    Ok((
        result,
        PaginatedResult {
            items: vec![json!({"kind": "query_explain"})],
            next_cursor: None,
            truncated: false,
        },
        confidence_block(
            0.89,
            vec!["rule_based_routing", "deterministic_playbook"],
            Vec::new(),
        ),
    ))
}

fn tool_definitions() -> Vec<Value> {
    vec![
        tool_def(
            "symbol_lookup",
            "Find symbols by normalized name",
            json!({"type":"object","required":["query"],"properties":{"query":{"type":"string"},"limit":{"type":"integer"},"cursor":{"type":"string"},"locator":{"type":"object"}}}),
        ),
        tool_def(
            "node_get",
            "Get one node by id",
            json!({"type":"object","required":["nodeId"],"properties":{"nodeId":{"type":"string"},"locator":{"type":"object"}}}),
        ),
        tool_def(
            "neighbors_get",
            "Get neighboring nodes and edges",
            json!({"type":"object","required":["nodeId"],"properties":{"nodeId":{"type":"string"},"direction":{"type":"string"},"relationTypes":{"type":"array","items":{"type":"string"}},"limit":{"type":"integer"},"cursor":{"type":"string"},"locator":{"type":"object"}}}),
        ),
        tool_def(
            "edge_get",
            "Get one edge by id",
            json!({"type":"object","required":["edgeId"],"properties":{"edgeId":{"type":"string"},"locator":{"type":"object"}}}),
        ),
        tool_def(
            "text_search",
            "Lexical text search over indexed frames",
            json!({"type":"object","required":["query"],"properties":{"query":{"type":"string"},"scope":{"type":"string"},"limit":{"type":"integer"},"cursor":{"type":"string"},"locator":{"type":"object"}}}),
        ),
        tool_def(
            "call_trace",
            "Trace CALLS paths between nodes",
            json!({"type":"object","required":["fromNodeId"],"properties":{"fromNodeId":{"type":"string"},"toNodeId":{"type":"string"},"maxDepth":{"type":"integer"},"limitPaths":{"type":"integer"},"locator":{"type":"object"}}}),
        ),
        tool_def(
            "callers_of",
            "List incoming CALLS edges",
            json!({"type":"object","required":["nodeId"],"properties":{"nodeId":{"type":"string"},"limit":{"type":"integer"},"cursor":{"type":"string"},"locator":{"type":"object"}}}),
        ),
        tool_def(
            "callees_of",
            "List outgoing CALLS edges",
            json!({"type":"object","required":["nodeId"],"properties":{"nodeId":{"type":"string"},"limit":{"type":"integer"},"cursor":{"type":"string"},"locator":{"type":"object"}}}),
        ),
        tool_def(
            "process_list",
            "List process nodes",
            json!({"type":"object","properties":{"limit":{"type":"integer"},"cursor":{"type":"string"},"locator":{"type":"object"}}}),
        ),
        tool_def(
            "process_get",
            "Get process details and ordered steps",
            json!({"type":"object","required":["processId"],"properties":{"processId":{"type":"string"},"locator":{"type":"object"}}}),
        ),
        tool_def(
            "impact_analysis",
            "Compute graph impact neighborhood",
            json!({"type":"object","required":["nodeId"],"properties":{"nodeId":{"type":"string"},"maxDepth":{"type":"integer"},"locator":{"type":"object"}}}),
        ),
        tool_def(
            "file_outline",
            "List symbols in a file",
            json!({"type":"object","required":["filePath"],"properties":{"filePath":{"type":"string"},"locator":{"type":"object"}}}),
        ),
        tool_def(
            "file_snippet",
            "Get bounded snippet for node/file",
            json!({"type":"object","properties":{"nodeId":{"type":"string"},"filePath":{"type":"string"},"maxChars":{"type":"integer"},"locator":{"type":"object"}}}),
        ),
        tool_def(
            "community_list",
            "List communities with membership counts",
            json!({"type":"object","properties":{"limit":{"type":"integer"},"cursor":{"type":"string"},"locator":{"type":"object"}}}),
        ),
        tool_def(
            "manifest_get",
            "Return manifest and capabilities",
            json!({"type":"object","properties":{"locator":{"type":"object"}}}),
        ),
        tool_def(
            "query_explain",
            "Explain retrieval/ranking and suggest tool sequence",
            json!({"type":"object","properties":{"task":{"type":"string"},"query":{"type":"string"},"locator":{"type":"object"}}}),
        ),
    ]
}

fn tool_def(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
        "outputSchemaRef": format!("#/toolOutputs/{name}"),
    })
}

fn jsonrpc_ok(
    id: Value,
    result: Value,
    status: StatusCode,
    rate: Option<crate::rate_limit::RateLimitHeaders>,
) -> Response<Body> {
    let payload = JsonRpcSuccess {
        jsonrpc: "2.0",
        id,
        result,
    };

    let body = serde_json::to_vec(&payload).unwrap_or_else(|_| b"{}".to_vec());
    let mut builder = Response::builder()
        .status(status)
        .header("content-type", "application/json");

    if let Some(rate) = rate {
        builder = attach_rate_headers(builder, rate);
    }

    builder.body(Body::from(body)).unwrap_or_else(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":{"code":"RESPONSE_BUILD_FAILED","message":"Failed building MCP response"}})),
        )
            .into_response()
    })
}

fn jsonrpc_error(
    id: Value,
    code: i64,
    message: impl Into<String>,
    data: Value,
    status: StatusCode,
    rate: Option<crate::rate_limit::RateLimitHeaders>,
) -> Response<Body> {
    let payload = JsonRpcFailure {
        jsonrpc: "2.0",
        id,
        error: JsonRpcError {
            code,
            message: message.into(),
            data,
        },
    };

    let body = serde_json::to_vec(&payload).unwrap_or_else(|_| b"{}".to_vec());
    let mut builder = Response::builder()
        .status(status)
        .header("content-type", "application/json");

    if let Some(rate) = rate {
        builder = attach_rate_headers(builder, rate);
    }

    builder.body(Body::from(body)).unwrap_or_else(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":{"code":"RESPONSE_BUILD_FAILED","message":"Failed building MCP response"}})),
        )
            .into_response()
    })
}

fn error_response(
    id: Value,
    status: StatusCode,
    code: &str,
    message: &str,
    data: Value,
    rate: Option<crate::rate_limit::RateLimitHeaders>,
) -> Response<Body> {
    jsonrpc_error(
        id,
        -32000,
        message,
        json!({ "code": code, "detail": data }),
        status,
        rate,
    )
}

fn attach_rate_headers(
    mut builder: axum::http::response::Builder,
    headers: crate::rate_limit::RateLimitHeaders,
) -> axum::http::response::Builder {
    builder = builder.header(
        "X-RateLimit-Limit",
        HeaderValue::from_str(&headers.limit.to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    );
    builder = builder.header(
        "X-RateLimit-Remaining",
        HeaderValue::from_str(&headers.remaining.to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    );
    builder.header(
        "X-RateLimit-Reset",
        HeaderValue::from_str(&headers.reset_seconds.to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    )
}

#[cfg(test)]
mod tests {
    use super::{decode_cursor, encode_cursor, normalize_text};

    #[test]
    fn cursor_roundtrip() {
        let encoded = encode_cursor(0.912345, "abc::1");
        let (score, key) = decode_cursor(&encoded).expect("cursor parse");
        assert!((score - 0.912345).abs() < 0.0001);
        assert_eq!(key, "abc::1");
    }

    #[test]
    fn normalize_text_is_deterministic() {
        assert_eq!(normalize_text("Foo::Bar-baz"), "foo bar baz");
    }
}
