use std::collections::HashMap;

use chrono::Utc;
use serde_json::json;

use crate::models::{ExportRequest, FrameDocument, GraphNode};

const MV2_SCHEMA_VERSION: &str = "gitnexus.mv2.schema.v1";
const EXPORT_SCHEMA_VERSION: &str = "gitnexus.export.schema.v1";
const AI_BIBLE_VERSION: &str = "gitnexus.ai-bible.v1";

pub fn build_frame_documents(req: &ExportRequest) -> Vec<FrameDocument> {
    let mut documents = Vec::new();
    let node_limit = req.options.max_node_frames.min(req.nodes.len());
    let relation_limit = req.options.max_relation_frames.min(req.relationships.len());

    let node_lookup: HashMap<&str, &GraphNode> =
        req.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    for node in req.nodes.iter().take(node_limit) {
        documents.push(build_node_document(req, node));
    }

    for rel in req.relationships.iter().take(relation_limit) {
        let source = node_lookup.get(rel.source_id.as_str());
        let target = node_lookup.get(rel.target_id.as_str());
        let source_name = source
            .map(|n| n.properties.name.as_str())
            .unwrap_or("<unknown>");
        let target_name = target
            .map(|n| n.properties.name.as_str())
            .unwrap_or("<unknown>");
        let source_label = source.map(|n| n.label.as_str()).unwrap_or("Unknown");
        let target_label = target.map(|n| n.label.as_str()).unwrap_or("Unknown");

        let title = format!("{source_name} {} {target_name}", rel.r#type);
        let uri = format!("mv2://relations/{}", rel.id);
        let track = format!("relations/{}", rel.r#type);
        let step_suffix = rel
            .step
            .map(|s| format!(" step={s}"))
            .unwrap_or_else(String::new);
        let summary = format!(
            "Relationship {rel_type}\nsource={source_name} ({source_label})\ntarget={target_name} ({target_label})\nconfidence={confidence:.3}\nreason={reason}{step_suffix}",
            rel_type = rel.r#type,
            confidence = rel.confidence,
            reason = rel.reason
        );
        let metadata = json!({
            "id": rel.id,
            "type": rel.r#type,
            "sourceId": rel.source_id,
            "targetId": rel.target_id,
            "sourceName": source_name,
            "targetName": target_name,
            "sourceLabel": source_label,
            "targetLabel": target_label,
            "confidence": rel.confidence,
            "reason": rel.reason,
            "step": rel.step,
        });

        documents.push(FrameDocument {
            title,
            label: "relation".to_string(),
            text: format!("{summary}\n\nmetadata={metadata}"),
            uri,
            track,
            tags: vec![
                "source=gitnexus".to_string(),
                format!("relationType={}", rel.r#type),
                format!("sessionId={}", req.session_id),
            ],
            metadata,
        });
    }

    documents.push(build_manifest_document(req, node_limit, relation_limit));
    documents.extend(build_ai_bible_documents(req));
    documents
}

fn build_node_document(req: &ExportRequest, node: &GraphNode) -> FrameDocument {
    let snippet = build_snippet(
        req.file_contents.get(&node.properties.file_path),
        node.properties.start_line,
        node.properties.end_line,
        req.options.max_snippet_chars,
        node.label.as_str(),
    );

    let metadata = json!({
        "id": node.id,
        "label": node.label,
        "name": node.properties.name,
        "filePath": node.properties.file_path,
        "startLine": node.properties.start_line,
        "endLine": node.properties.end_line,
        "language": node.properties.language,
        "isExported": node.properties.is_exported,
        "heuristicLabel": node.properties.heuristic_label,
        "cohesion": node.properties.cohesion,
        "symbolCount": node.properties.symbol_count,
        "keywords": node.properties.keywords,
        "description": node.properties.description,
        "enrichedBy": node.properties.enriched_by,
        "processType": node.properties.process_type,
        "stepCount": node.properties.step_count,
        "communities": node.properties.communities,
        "entryPointId": node.properties.entry_point_id,
        "terminalId": node.properties.terminal_id,
        "entryPointScore": node.properties.entry_point_score,
        "entryPointReason": node.properties.entry_point_reason,
    });

    let uri = match node.label.as_str() {
        "Community" => format!("mv2://communities/{}", node.id),
        "Process" => format!("mv2://processes/{}", node.id),
        _ => format!("mv2://nodes/{}", node.id),
    };

    let track = node_track(&node.label);
    let text = format!(
        "Node {label}\nid={id}\nname={name}\nfilePath={file}\n\nsnippet:\n{snippet}\n\nmetadata={metadata}",
        label = node.label,
        id = node.id,
        name = node.properties.name,
        file = node.properties.file_path
    );

    FrameDocument {
        title: format!("{}: {}", node.label, node.properties.name),
        label: node.label.clone(),
        text,
        uri,
        track,
        tags: vec![
            "source=gitnexus".to_string(),
            format!("nodeLabel={}", node.label),
            format!("sessionId={}", req.session_id),
        ],
        metadata,
    }
}

fn build_manifest_document(
    req: &ExportRequest,
    node_frames: usize,
    relation_frames: usize,
) -> FrameDocument {
    let mut label_counts: HashMap<String, usize> = HashMap::new();
    let mut relation_counts: HashMap<String, usize> = HashMap::new();

    for node in &req.nodes {
        *label_counts.entry(node.label.clone()).or_insert(0) += 1;
    }
    for rel in &req.relationships {
        *relation_counts.entry(rel.r#type.clone()).or_insert(0) += 1;
    }

    let metadata = json!({
        "generatedAt": Utc::now(),
        "mv2SchemaVersion": MV2_SCHEMA_VERSION,
        "exportSchemaVersion": EXPORT_SCHEMA_VERSION,
        "aiBibleVersion": AI_BIBLE_VERSION,
        "sessionId": req.session_id,
        "projectName": req.project_name,
        "source": req.source,
        "options": req.options,
        "capsuleCapabilities": {
            "strictJsonToolResponses": true,
            "cursorPagination": true,
            "semanticFallbackOnly": true,
            "defaultResponseBudgetBytes": 65536,
            "supportsLegacyCapsules": true,
            "toolCount": 16,
            "toolSetVersion": "gitnexus.tools.v1",
        },
        "totals": {
            "nodes": req.nodes.len(),
            "relationships": req.relationships.len(),
            "exportedNodeFrames": node_frames,
            "exportedRelationFrames": relation_frames,
            "fileCount": req.file_contents.len(),
        },
        "nodeLabels": label_counts,
        "relationshipTypes": relation_counts,
    });

    FrameDocument {
        title: format!("GitNexus manifest: {}", req.project_name),
        label: "manifest".to_string(),
        text: format!(
            "GitNexus export manifest\nproject={}\nsource={}\nnodes={}\nrelationships={}\n\nmetadata={metadata}",
            req.project_name,
            req.source.base_name,
            req.nodes.len(),
            req.relationships.len(),
        ),
        uri: "mv2://meta/manifest".to_string(),
        track: "meta".to_string(),
        tags: vec![
            "source=gitnexus".to_string(),
            "kind=manifest".to_string(),
            format!("sessionId={}", req.session_id),
        ],
        metadata,
    }
}

fn build_ai_bible_documents(req: &ExportRequest) -> Vec<FrameDocument> {
    let manifest_metadata = json!({
        "version": AI_BIBLE_VERSION,
        "schemaVersion": "gitnexus.mcp.v1",
        "mcpTransport": "streamable_http_jsonrpc",
        "primaryGoal": "deterministic_accuracy",
        "responseBudgetBytes": 65536,
        "semanticPolicy": "fallback_only",
        "toolCount": 16,
    });

    let tool_matrix_metadata = json!({
        "toolSetVersion": "gitnexus.tools.v1",
        "tools": [
            "symbol_lookup",
            "node_get",
            "neighbors_get",
            "edge_get",
            "text_search",
            "call_trace",
            "callers_of",
            "callees_of",
            "process_list",
            "process_get",
            "impact_analysis",
            "file_outline",
            "file_snippet",
            "community_list",
            "manifest_get",
            "query_explain"
        ]
    });

    let retrieval_metadata = json!({
        "ladder": [
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
        ]
    });

    let playbook_metadata = json!({
        "playbooks": [
            "root_cause_from_symptom",
            "change_impact_before_edit",
            "subsystem_architecture_extraction",
            "process_comprehension_step_in_process"
        ],
        "sessionId": req.session_id,
    });

    vec![
        FrameDocument {
            title: "AI Bible Manifest".to_string(),
            label: "ai_bible".to_string(),
            text: format!(
                "AI Bible manifest\nversion={AI_BIBLE_VERSION}\nproject={}\n\nmetadata={manifest_metadata}",
                req.project_name
            ),
            uri: "mv2://meta/ai-bible/manifest".to_string(),
            track: "meta".to_string(),
            tags: vec![
                "source=gitnexus".to_string(),
                "kind=ai-bible".to_string(),
                format!("sessionId={}", req.session_id),
            ],
            metadata: manifest_metadata,
        },
        FrameDocument {
            title: "AI Bible Tool Matrix".to_string(),
            label: "ai_bible".to_string(),
            text: format!(
                "AI Bible tool matrix\nmode=strict_json\ntransport=streamable_http_jsonrpc\n\nmetadata={tool_matrix_metadata}"
            ),
            uri: "mv2://meta/ai-bible/tool-matrix".to_string(),
            track: "meta".to_string(),
            tags: vec![
                "source=gitnexus".to_string(),
                "kind=ai-bible".to_string(),
                format!("sessionId={}", req.session_id),
            ],
            metadata: tool_matrix_metadata,
        },
        FrameDocument {
            title: "AI Bible Retrieval Ladder".to_string(),
            label: "ai_bible".to_string(),
            text: format!(
                "AI Bible retrieval ladder\ndefault=deterministic\nsemantic=fallback_only\n\nmetadata={retrieval_metadata}"
            ),
            uri: "mv2://meta/ai-bible/retrieval-ladder".to_string(),
            track: "meta".to_string(),
            tags: vec![
                "source=gitnexus".to_string(),
                "kind=ai-bible".to_string(),
                format!("sessionId={}", req.session_id),
            ],
            metadata: retrieval_metadata,
        },
        FrameDocument {
            title: "AI Bible Playbooks".to_string(),
            label: "ai_bible".to_string(),
            text: format!(
                "AI Bible playbooks\n1=root_cause_from_symptom\n2=change_impact_before_edit\n3=subsystem_architecture_extraction\n4=process_comprehension_step_in_process\n\nmetadata={playbook_metadata}"
            ),
            uri: "mv2://meta/ai-bible/playbooks/core".to_string(),
            track: "meta".to_string(),
            tags: vec![
                "source=gitnexus".to_string(),
                "kind=ai-bible".to_string(),
                format!("sessionId={}", req.session_id),
            ],
            metadata: playbook_metadata,
        },
    ]
}

fn build_snippet(
    file_content: Option<&String>,
    start_line: Option<usize>,
    end_line: Option<usize>,
    max_chars: usize,
    label: &str,
) -> String {
    let Some(content) = file_content else {
        return "<no source content available>".to_string();
    };

    let lines: Vec<&str> = content.lines().collect();
    let default_end = if label == "File" {
        lines.len().min(80)
    } else {
        lines.len().min(40)
    };

    let start = start_line.unwrap_or(1).max(1).min(lines.len().max(1));
    let end = end_line
        .unwrap_or(default_end)
        .max(start)
        .min(lines.len().max(1));
    let snippet = if lines.is_empty() {
        String::new()
    } else {
        lines[(start - 1)..end].join("\n")
    };

    truncate_chars(&snippet, max_chars.max(80))
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    input.chars().take(max_chars).collect::<String>() + "\n...[truncated]"
}

fn node_track(node_label: &str) -> String {
    match node_label {
        "Community" => "communities".to_string(),
        "Process" => "processes".to_string(),
        "File" => "files".to_string(),
        _ => format!("nodes/{node_label}"),
    }
}
