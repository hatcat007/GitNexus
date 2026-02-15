use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use memvid_core::{Memvid, TimelineQuery};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::models::{ExportRequest, FrameDocument};

pub const MCP_SCHEMA_VERSION: &str = "gitnexus.mcp.v1";
pub const MCP_INDEX_SCHEMA_VERSION: &str = "gitnexus.mcp.index.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRecord {
    pub id: String,
    pub label: String,
    pub name: String,
    pub file_path: String,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
    pub language: Option<String>,
    pub uri: String,
    pub title: String,
    pub search_text: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeRecord {
    pub id: String,
    pub relation_type: String,
    pub source_id: String,
    pub target_id: String,
    pub confidence: f64,
    pub reason: String,
    pub step: Option<usize>,
    pub uri: String,
    pub search_text: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStepRecord {
    pub process_id: String,
    pub step: usize,
    pub function_id: String,
    pub relation_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolRecord {
    pub symbol_norm: String,
    pub symbol: String,
    pub node_id: String,
    pub file_path: String,
    pub node_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotspotRecord {
    pub file_path: String,
    pub calls_count: usize,
    pub node_count: usize,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityMembershipRecord {
    pub community_id: String,
    pub node_id: String,
    pub node_label: String,
    pub node_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FulltextEntry {
    pub ref_kind: String,
    pub ref_id: String,
    pub uri: String,
    pub track: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct CapsuleIndex {
    pub capsule_path: PathBuf,
    pub sidecar_path: PathBuf,
    pub schema_version: String,
    pub generated_at: DateTime<Utc>,
    pub manifest: Value,
    pub capabilities: Value,
    pub nodes: Vec<NodeRecord>,
    pub edges: Vec<EdgeRecord>,
    pub process_steps: Vec<ProcessStepRecord>,
    pub symbols: Vec<SymbolRecord>,
    pub hotspots: Vec<HotspotRecord>,
    pub community_membership: Vec<CommunityMembershipRecord>,
    pub fulltext: Vec<FulltextEntry>,
    pub node_by_id: HashMap<String, usize>,
    pub edge_by_id: HashMap<String, usize>,
    pub edges_out_by_node: HashMap<String, Vec<usize>>,
    pub edges_in_by_node: HashMap<String, Vec<usize>>,
    pub nodes_by_label: HashMap<String, Vec<usize>>,
    pub nodes_by_file: HashMap<String, Vec<usize>>,
    pub process_step_by_process: HashMap<String, Vec<usize>>,
    pub symbols_by_norm: HashMap<String, Vec<usize>>,
}

impl CapsuleIndex {
    fn build_runtime_maps(&mut self) {
        self.node_by_id.clear();
        self.edge_by_id.clear();
        self.edges_out_by_node.clear();
        self.edges_in_by_node.clear();
        self.nodes_by_label.clear();
        self.nodes_by_file.clear();
        self.process_step_by_process.clear();
        self.symbols_by_norm.clear();

        for (idx, node) in self.nodes.iter().enumerate() {
            self.node_by_id.insert(node.id.clone(), idx);
            self.nodes_by_label
                .entry(node.label.clone())
                .or_default()
                .push(idx);
            self.nodes_by_file
                .entry(node.file_path.clone())
                .or_default()
                .push(idx);
        }

        for (idx, edge) in self.edges.iter().enumerate() {
            self.edge_by_id.insert(edge.id.clone(), idx);
            self.edges_out_by_node
                .entry(edge.source_id.clone())
                .or_default()
                .push(idx);
            self.edges_in_by_node
                .entry(edge.target_id.clone())
                .or_default()
                .push(idx);
        }

        for (idx, step) in self.process_steps.iter().enumerate() {
            self.process_step_by_process
                .entry(step.process_id.clone())
                .or_default()
                .push(idx);
        }

        for (idx, symbol) in self.symbols.iter().enumerate() {
            self.symbols_by_norm
                .entry(symbol.symbol_norm.clone())
                .or_default()
                .push(idx);
        }

        for entries in self.process_step_by_process.values_mut() {
            entries.sort_by_key(|idx| {
                let step = &self.process_steps[*idx];
                (step.step, step.function_id.clone())
            });
        }
    }
}

pub fn sidecar_path_for_capsule(capsule_path: &Path) -> PathBuf {
    let mut file_name = capsule_path
        .file_name()
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_else(|| "capsule.mv2".to_string());
    file_name.push_str(".index.v1.sqlite");
    capsule_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(file_name)
}

pub fn build_and_persist_from_request(
    req: &ExportRequest,
    docs: &[FrameDocument],
    capsule_path: &Path,
) -> Result<CapsuleIndex> {
    let sidecar_path = sidecar_path_for_capsule(capsule_path);

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut fulltext = Vec::new();
    let mut manifest = Value::Null;
    let mut has_ai_bible = false;

    for doc in docs {
        if doc.uri.starts_with("mv2://meta/ai-bible/") {
            has_ai_bible = true;
        }

        let kind = if doc.label == "relation" {
            "relation"
        } else if doc.label == "manifest" {
            "manifest"
        } else if doc.label == "ai_bible" {
            "ai_bible"
        } else {
            "node"
        };

        fulltext.push(FulltextEntry {
            ref_kind: kind.to_string(),
            ref_id: doc.uri.clone(),
            uri: doc.uri.clone(),
            track: doc.track.clone(),
            text: doc.text.clone(),
        });

        if doc.label == "manifest" {
            manifest = doc.metadata.clone();
            continue;
        }

        if doc.label == "relation" {
            let edge_id = doc
                .metadata
                .get("id")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .unwrap_or_else(|| doc.uri.trim_start_matches("mv2://relations/").to_string());

            let relation_type = doc
                .metadata
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("UNKNOWN")
                .to_string();

            let source_id = doc
                .metadata
                .get("sourceId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();

            let target_id = doc
                .metadata
                .get("targetId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();

            let confidence = doc
                .metadata
                .get("confidence")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);

            let reason = doc
                .metadata
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();

            let step = doc
                .metadata
                .get("step")
                .and_then(Value::as_u64)
                .map(|v| v as usize);

            edges.push(EdgeRecord {
                id: edge_id,
                relation_type,
                source_id,
                target_id,
                confidence,
                reason,
                step,
                uri: doc.uri.clone(),
                search_text: doc.text.clone(),
                metadata: doc.metadata.clone(),
            });
            continue;
        }

        if doc.label == "ai_bible" {
            continue;
        }

        nodes.push(NodeRecord {
            id: doc
                .metadata
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_else(|| doc.uri.trim_start_matches("mv2://nodes/"))
                .to_string(),
            label: doc
                .metadata
                .get("label")
                .and_then(Value::as_str)
                .unwrap_or(&doc.label)
                .to_string(),
            name: doc
                .metadata
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(&doc.title)
                .to_string(),
            file_path: doc
                .metadata
                .get("filePath")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            start_line: doc
                .metadata
                .get("startLine")
                .and_then(Value::as_u64)
                .map(|v| v as usize),
            end_line: doc
                .metadata
                .get("endLine")
                .and_then(Value::as_u64)
                .map(|v| v as usize),
            language: doc
                .metadata
                .get("language")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            uri: doc.uri.clone(),
            title: doc.title.clone(),
            search_text: doc.text.clone(),
            metadata: doc.metadata.clone(),
        });
    }

    let process_steps = derive_process_steps(&edges);
    let symbols = derive_symbols(&nodes);
    let hotspots = derive_hotspots(&nodes, &edges);
    let community_membership = derive_community_membership(&nodes);

    let capabilities = json!({
        "schemaVersion": MCP_SCHEMA_VERSION,
        "indexSchemaVersion": MCP_INDEX_SCHEMA_VERSION,
        "supportsSemanticFallback": req.options.semantic_enabled,
        "hasAiBible": has_ai_bible,
        "hasManifest": !manifest.is_null(),
        "nodeCount": nodes.len(),
        "edgeCount": edges.len(),
        "fulltextCount": fulltext.len(),
    });

    let mut index = CapsuleIndex {
        capsule_path: capsule_path.to_path_buf(),
        sidecar_path,
        schema_version: MCP_INDEX_SCHEMA_VERSION.to_string(),
        generated_at: Utc::now(),
        manifest,
        capabilities,
        nodes,
        edges,
        process_steps,
        symbols,
        hotspots,
        community_membership,
        fulltext,
        node_by_id: HashMap::new(),
        edge_by_id: HashMap::new(),
        edges_out_by_node: HashMap::new(),
        edges_in_by_node: HashMap::new(),
        nodes_by_label: HashMap::new(),
        nodes_by_file: HashMap::new(),
        process_step_by_process: HashMap::new(),
        symbols_by_norm: HashMap::new(),
    };

    index.build_runtime_maps();
    persist_to_sidecar(&index)?;
    Ok(index)
}

pub fn load_from_sidecar(capsule_path: &Path) -> Result<CapsuleIndex> {
    let sidecar_path = sidecar_path_for_capsule(capsule_path);
    let conn = Connection::open(&sidecar_path)
        .with_context(|| format!("Failed opening sidecar {}", sidecar_path.display()))?;

    let schema_version: String = conn
        .query_row(
            "SELECT value FROM meta WHERE key='index_schema_version'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| MCP_INDEX_SCHEMA_VERSION.to_string());

    let generated_at: DateTime<Utc> = conn
        .query_row(
            "SELECT value FROM meta WHERE key='generated_at'",
            [],
            |row| {
                let s: String = row.get(0)?;
                Ok(DateTime::parse_from_rfc3339(&s)
                    .map(|v| v.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()))
            },
        )
        .unwrap_or_else(|_| Utc::now());

    let manifest: Value = conn
        .query_row(
            "SELECT value FROM meta WHERE key='manifest_json'",
            [],
            |row| {
                let s: String = row.get(0)?;
                Ok(serde_json::from_str(&s).unwrap_or(Value::Null))
            },
        )
        .unwrap_or(Value::Null);

    let capabilities: Value = conn
        .query_row(
            "SELECT value FROM meta WHERE key='capabilities_json'",
            [],
            |row| {
                let s: String = row.get(0)?;
                Ok(serde_json::from_str(&s).unwrap_or(Value::Null))
            },
        )
        .unwrap_or(Value::Null);

    let mut nodes = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT node_id,node_label,node_name,file_path,start_line,end_line,language,uri,title,search_text,metadata_json FROM nodes_by_id",
        )?;
        let rows = stmt.query_map([], |row| {
            let metadata_json: String = row.get(10)?;
            Ok(NodeRecord {
                id: row.get(0)?,
                label: row.get(1)?,
                name: row.get(2)?,
                file_path: row.get(3)?,
                start_line: row.get::<_, Option<i64>>(4)?.map(|v| v as usize),
                end_line: row.get::<_, Option<i64>>(5)?.map(|v| v as usize),
                language: row.get(6)?,
                uri: row.get(7)?,
                title: row.get(8)?,
                search_text: row.get(9)?,
                metadata: serde_json::from_str(&metadata_json).unwrap_or(Value::Null),
            })
        })?;
        for row in rows {
            nodes.push(row?);
        }
    }

    let mut edges = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT edge_id,relation_type,source_id,target_id,confidence,reason,step,uri,search_text,metadata_json FROM edges",
        )?;
        let rows = stmt.query_map([], |row| {
            let metadata_json: String = row.get(9)?;
            Ok(EdgeRecord {
                id: row.get(0)?,
                relation_type: row.get(1)?,
                source_id: row.get(2)?,
                target_id: row.get(3)?,
                confidence: row.get(4)?,
                reason: row.get(5)?,
                step: row.get::<_, Option<i64>>(6)?.map(|v| v as usize),
                uri: row.get(7)?,
                search_text: row.get(8)?,
                metadata: serde_json::from_str(&metadata_json).unwrap_or(Value::Null),
            })
        })?;
        for row in rows {
            edges.push(row?);
        }
    }

    let mut process_steps = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT process_id,step,function_id,relation_uri FROM process_steps_by_process_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ProcessStepRecord {
                process_id: row.get(0)?,
                step: row.get::<_, i64>(1)? as usize,
                function_id: row.get(2)?,
                relation_uri: row.get(3)?,
            })
        })?;
        for row in rows {
            process_steps.push(row?);
        }
    }

    let mut symbols = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT symbol_norm,symbol,node_id,file_path,node_label FROM symbols_by_name_normalized",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SymbolRecord {
                symbol_norm: row.get(0)?,
                symbol: row.get(1)?,
                node_id: row.get(2)?,
                file_path: row.get(3)?,
                node_label: row.get(4)?,
            })
        })?;
        for row in rows {
            symbols.push(row?);
        }
    }

    let mut hotspots = Vec::new();
    {
        let mut stmt =
            conn.prepare("SELECT file_path,calls_count,node_count,score FROM hotspots")?;
        let rows = stmt.query_map([], |row| {
            Ok(HotspotRecord {
                file_path: row.get(0)?,
                calls_count: row.get::<_, i64>(1)? as usize,
                node_count: row.get::<_, i64>(2)? as usize,
                score: row.get(3)?,
            })
        })?;
        for row in rows {
            hotspots.push(row?);
        }
    }

    let mut community_membership = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT community_id,node_id,node_label,node_name FROM community_membership",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(CommunityMembershipRecord {
                community_id: row.get(0)?,
                node_id: row.get(1)?,
                node_label: row.get(2)?,
                node_name: row.get(3)?,
            })
        })?;
        for row in rows {
            community_membership.push(row?);
        }
    }

    let mut fulltext = Vec::new();
    {
        let mut stmt =
            conn.prepare("SELECT ref_kind,ref_id,uri,track,text FROM fulltext_lexical_index")?;
        let rows = stmt.query_map([], |row| {
            Ok(FulltextEntry {
                ref_kind: row.get(0)?,
                ref_id: row.get(1)?,
                uri: row.get(2)?,
                track: row.get(3)?,
                text: row.get(4)?,
            })
        })?;
        for row in rows {
            fulltext.push(row?);
        }
    }

    let mut index = CapsuleIndex {
        capsule_path: capsule_path.to_path_buf(),
        sidecar_path,
        schema_version,
        generated_at,
        manifest,
        capabilities,
        nodes,
        edges,
        process_steps,
        symbols,
        hotspots,
        community_membership,
        fulltext,
        node_by_id: HashMap::new(),
        edge_by_id: HashMap::new(),
        edges_out_by_node: HashMap::new(),
        edges_in_by_node: HashMap::new(),
        nodes_by_label: HashMap::new(),
        nodes_by_file: HashMap::new(),
        process_step_by_process: HashMap::new(),
        symbols_by_norm: HashMap::new(),
    };
    index.build_runtime_maps();
    Ok(index)
}

pub fn build_from_capsule(capsule_path: &Path) -> Result<CapsuleIndex> {
    let sidecar_path = sidecar_path_for_capsule(capsule_path);
    let mut mem = Memvid::open_read_only(capsule_path)
        .with_context(|| format!("Failed opening capsule {}", capsule_path.display()))?;

    let stats = mem.stats().ok();
    let timeline = mem.timeline(TimelineQuery::builder().no_limit().build())?;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut fulltext = Vec::new();
    let mut manifest = Value::Null;
    let mut has_ai_bible = false;

    for entry in timeline {
        let frame = match mem.frame_by_id(entry.frame_id) {
            Ok(frame) => frame,
            Err(_) => continue,
        };

        let uri = frame
            .uri
            .clone()
            .or(entry.uri.clone())
            .unwrap_or_else(|| format!("mv2://frame/{}", entry.frame_id));

        let text = frame
            .search_text
            .clone()
            .unwrap_or_else(|| entry.preview.clone());

        let track = frame.track.clone().unwrap_or_default();

        let ref_kind = if uri.starts_with("mv2://relations/") {
            "relation"
        } else if uri.starts_with("mv2://meta/manifest") {
            "manifest"
        } else if uri.starts_with("mv2://meta/ai-bible/") {
            has_ai_bible = true;
            "ai_bible"
        } else {
            "node"
        }
        .to_string();

        fulltext.push(FulltextEntry {
            ref_kind,
            ref_id: uri.clone(),
            uri: uri.clone(),
            track,
            text: text.clone(),
        });

        if uri.starts_with("mv2://meta/manifest") {
            manifest = parse_metadata_json(&text).unwrap_or(Value::Null);
            continue;
        }

        if uri.starts_with("mv2://meta/ai-bible/") {
            continue;
        }

        if uri.starts_with("mv2://relations/") {
            let mut metadata = parse_metadata_json(&text).unwrap_or(Value::Null);
            if metadata.is_null() {
                metadata = json!({});
            }
            let edge_id = uri.trim_start_matches("mv2://relations/").to_string();
            let relation_type = metadata
                .get("type")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .or_else(|| parse_relation_type(&text))
                .unwrap_or_else(|| "UNKNOWN".to_string());

            let source_id = metadata
                .get("sourceId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();

            let target_id = metadata
                .get("targetId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();

            let confidence = metadata
                .get("confidence")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let reason = metadata
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let step = metadata
                .get("step")
                .and_then(Value::as_u64)
                .map(|v| v as usize)
                .or_else(|| parse_step_from_text(&text));

            edges.push(EdgeRecord {
                id: edge_id,
                relation_type,
                source_id,
                target_id,
                confidence,
                reason,
                step,
                uri,
                search_text: text,
                metadata,
            });

            continue;
        }

        let metadata = parse_metadata_json(&text).unwrap_or(Value::Null);

        let label = if uri.starts_with("mv2://communities/") {
            "Community".to_string()
        } else if uri.starts_with("mv2://processes/") {
            "Process".to_string()
        } else {
            parse_node_label(&text).unwrap_or_else(|| "Node".to_string())
        };

        let id = parse_id_line(&text)
            .unwrap_or_else(|| uri.rsplit('/').next().unwrap_or_default().to_string());

        let name = parse_name_line(&text).unwrap_or_else(|| id.clone());
        let file_path = parse_file_path_line(&text).unwrap_or_default();
        let start_line = metadata
            .get("startLine")
            .and_then(Value::as_u64)
            .map(|v| v as usize);
        let end_line = metadata
            .get("endLine")
            .and_then(Value::as_u64)
            .map(|v| v as usize);
        let language = metadata
            .get("language")
            .and_then(Value::as_str)
            .map(ToString::to_string);

        nodes.push(NodeRecord {
            id,
            label,
            name: name.clone(),
            file_path,
            start_line,
            end_line,
            language,
            uri,
            title: name,
            search_text: text,
            metadata,
        });
    }

    let process_steps = derive_process_steps(&edges);
    let symbols = derive_symbols(&nodes);
    let hotspots = derive_hotspots(&nodes, &edges);
    let community_membership = derive_community_membership(&nodes);
    let supports_semantic = stats.as_ref().map(|s| s.has_vec_index).unwrap_or(false);
    let stats_summary = stats
        .as_ref()
        .map(|s| json!({ "hasVecIndex": s.has_vec_index }));

    let capabilities = json!({
        "schemaVersion": MCP_SCHEMA_VERSION,
        "indexSchemaVersion": MCP_INDEX_SCHEMA_VERSION,
        "supportsSemanticFallback": supports_semantic,
        "hasAiBible": has_ai_bible,
        "hasManifest": !manifest.is_null(),
        "stats": stats_summary,
        "nodeCount": nodes.len(),
        "edgeCount": edges.len(),
        "fulltextCount": fulltext.len(),
    });

    let mut index = CapsuleIndex {
        capsule_path: capsule_path.to_path_buf(),
        sidecar_path,
        schema_version: MCP_INDEX_SCHEMA_VERSION.to_string(),
        generated_at: Utc::now(),
        manifest,
        capabilities,
        nodes,
        edges,
        process_steps,
        symbols,
        hotspots,
        community_membership,
        fulltext,
        node_by_id: HashMap::new(),
        edge_by_id: HashMap::new(),
        edges_out_by_node: HashMap::new(),
        edges_in_by_node: HashMap::new(),
        nodes_by_label: HashMap::new(),
        nodes_by_file: HashMap::new(),
        process_step_by_process: HashMap::new(),
        symbols_by_norm: HashMap::new(),
    };
    index.build_runtime_maps();
    Ok(index)
}

pub fn persist_to_sidecar(index: &CapsuleIndex) -> Result<()> {
    if let Some(parent) = index.sidecar_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed creating sidecar directory {}", parent.display()))?;
    }

    let conn = Connection::open(&index.sidecar_path)
        .with_context(|| format!("Failed opening {}", index.sidecar_path.display()))?;

    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS nodes_by_id (
            node_id TEXT PRIMARY KEY,
            node_label TEXT NOT NULL,
            node_name TEXT NOT NULL,
            file_path TEXT NOT NULL,
            start_line INTEGER,
            end_line INTEGER,
            language TEXT,
            uri TEXT NOT NULL,
            title TEXT NOT NULL,
            search_text TEXT NOT NULL,
            metadata_json TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS nodes_by_label (
            node_label TEXT NOT NULL,
            node_id TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS nodes_by_file (
            file_path TEXT NOT NULL,
            node_id TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS symbols_by_name_normalized (
            symbol_norm TEXT NOT NULL,
            symbol TEXT NOT NULL,
            node_id TEXT NOT NULL,
            file_path TEXT NOT NULL,
            node_label TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS edges (
            edge_id TEXT PRIMARY KEY,
            relation_type TEXT NOT NULL,
            source_id TEXT NOT NULL,
            target_id TEXT NOT NULL,
            confidence REAL NOT NULL,
            reason TEXT NOT NULL,
            step INTEGER,
            uri TEXT NOT NULL,
            search_text TEXT NOT NULL,
            metadata_json TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS edges_by_source_type (
            source_id TEXT NOT NULL,
            relation_type TEXT NOT NULL,
            edge_id TEXT NOT NULL,
            target_id TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS edges_by_target_type (
            target_id TEXT NOT NULL,
            relation_type TEXT NOT NULL,
            edge_id TEXT NOT NULL,
            source_id TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS process_steps_by_process_id (
            process_id TEXT NOT NULL,
            step INTEGER NOT NULL,
            function_id TEXT NOT NULL,
            relation_uri TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS fulltext_lexical_index (
            ref_kind TEXT NOT NULL,
            ref_id TEXT NOT NULL,
            uri TEXT NOT NULL,
            track TEXT NOT NULL,
            text TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS hotspots (
            file_path TEXT PRIMARY KEY,
            calls_count INTEGER NOT NULL,
            node_count INTEGER NOT NULL,
            score REAL NOT NULL
        );
        CREATE TABLE IF NOT EXISTS community_membership (
            community_id TEXT NOT NULL,
            node_id TEXT NOT NULL,
            node_label TEXT NOT NULL,
            node_name TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_nodes_label ON nodes_by_label(node_label);
        CREATE INDEX IF NOT EXISTS idx_nodes_file ON nodes_by_file(file_path);
        CREATE INDEX IF NOT EXISTS idx_symbols_norm ON symbols_by_name_normalized(symbol_norm);
        CREATE INDEX IF NOT EXISTS idx_edges_source_type ON edges_by_source_type(source_id, relation_type);
        CREATE INDEX IF NOT EXISTS idx_edges_target_type ON edges_by_target_type(target_id, relation_type);
        CREATE INDEX IF NOT EXISTS idx_process_steps ON process_steps_by_process_id(process_id, step);
        CREATE INDEX IF NOT EXISTS idx_fulltext_kind ON fulltext_lexical_index(ref_kind);
        CREATE INDEX IF NOT EXISTS idx_fulltext_uri ON fulltext_lexical_index(uri);
        CREATE INDEX IF NOT EXISTS idx_community_id ON community_membership(community_id);
        DELETE FROM meta;
        DELETE FROM nodes_by_id;
        DELETE FROM nodes_by_label;
        DELETE FROM nodes_by_file;
        DELETE FROM symbols_by_name_normalized;
        DELETE FROM edges;
        DELETE FROM edges_by_source_type;
        DELETE FROM edges_by_target_type;
        DELETE FROM process_steps_by_process_id;
        DELETE FROM fulltext_lexical_index;
        DELETE FROM hotspots;
        DELETE FROM community_membership;
        ",
    )?;

    let tx = conn.unchecked_transaction()?;

    tx.execute(
        "INSERT INTO meta(key,value) VALUES(?1,?2)",
        params!["index_schema_version", index.schema_version],
    )?;
    tx.execute(
        "INSERT INTO meta(key,value) VALUES(?1,?2)",
        params!["generated_at", index.generated_at.to_rfc3339()],
    )?;
    tx.execute(
        "INSERT INTO meta(key,value) VALUES(?1,?2)",
        params!["manifest_json", index.manifest.to_string()],
    )?;
    tx.execute(
        "INSERT INTO meta(key,value) VALUES(?1,?2)",
        params!["capabilities_json", index.capabilities.to_string()],
    )?;

    for node in &index.nodes {
        tx.execute(
            "INSERT INTO nodes_by_id(node_id,node_label,node_name,file_path,start_line,end_line,language,uri,title,search_text,metadata_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                node.id,
                node.label,
                node.name,
                node.file_path,
                node.start_line.map(|v| v as i64),
                node.end_line.map(|v| v as i64),
                node.language,
                node.uri,
                node.title,
                node.search_text,
                node.metadata.to_string()
            ],
        )?;
        tx.execute(
            "INSERT INTO nodes_by_label(node_label,node_id) VALUES(?1,?2)",
            params![node.label, node.id],
        )?;
        tx.execute(
            "INSERT INTO nodes_by_file(file_path,node_id) VALUES(?1,?2)",
            params![node.file_path, node.id],
        )?;
    }

    for symbol in &index.symbols {
        tx.execute(
            "INSERT INTO symbols_by_name_normalized(symbol_norm,symbol,node_id,file_path,node_label) VALUES(?1,?2,?3,?4,?5)",
            params![
                symbol.symbol_norm,
                symbol.symbol,
                symbol.node_id,
                symbol.file_path,
                symbol.node_label
            ],
        )?;
    }

    for edge in &index.edges {
        tx.execute(
            "INSERT INTO edges(edge_id,relation_type,source_id,target_id,confidence,reason,step,uri,search_text,metadata_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                edge.id,
                edge.relation_type,
                edge.source_id,
                edge.target_id,
                edge.confidence,
                edge.reason,
                edge.step.map(|v| v as i64),
                edge.uri,
                edge.search_text,
                edge.metadata.to_string()
            ],
        )?;
        tx.execute(
            "INSERT INTO edges_by_source_type(source_id,relation_type,edge_id,target_id) VALUES(?1,?2,?3,?4)",
            params![edge.source_id, edge.relation_type, edge.id, edge.target_id],
        )?;
        tx.execute(
            "INSERT INTO edges_by_target_type(target_id,relation_type,edge_id,source_id) VALUES(?1,?2,?3,?4)",
            params![edge.target_id, edge.relation_type, edge.id, edge.source_id],
        )?;
    }

    for step in &index.process_steps {
        tx.execute(
            "INSERT INTO process_steps_by_process_id(process_id,step,function_id,relation_uri) VALUES(?1,?2,?3,?4)",
            params![step.process_id, step.step as i64, step.function_id, step.relation_uri],
        )?;
    }

    for entry in &index.fulltext {
        tx.execute(
            "INSERT INTO fulltext_lexical_index(ref_kind,ref_id,uri,track,text) VALUES(?1,?2,?3,?4,?5)",
            params![entry.ref_kind, entry.ref_id, entry.uri, entry.track, entry.text],
        )?;
    }

    for hotspot in &index.hotspots {
        tx.execute(
            "INSERT INTO hotspots(file_path,calls_count,node_count,score) VALUES(?1,?2,?3,?4)",
            params![
                hotspot.file_path,
                hotspot.calls_count as i64,
                hotspot.node_count as i64,
                hotspot.score
            ],
        )?;
    }

    for membership in &index.community_membership {
        tx.execute(
            "INSERT INTO community_membership(community_id,node_id,node_label,node_name) VALUES(?1,?2,?3,?4)",
            params![
                membership.community_id,
                membership.node_id,
                membership.node_label,
                membership.node_name
            ],
        )?;
    }

    tx.commit()?;
    Ok(())
}

fn derive_process_steps(edges: &[EdgeRecord]) -> Vec<ProcessStepRecord> {
    let mut steps = Vec::new();
    for edge in edges {
        if edge.relation_type != "STEP_IN_PROCESS" {
            continue;
        }

        let process_id = if edge.target_id.contains("proc_") {
            edge.target_id.clone()
        } else if edge.source_id.contains("proc_") {
            edge.source_id.clone()
        } else if let Some(pos) = edge.uri.find("_proc_") {
            edge.uri[(pos + 1)..].to_string()
        } else {
            continue;
        };

        let function_id = if edge.source_id.contains("proc_") {
            edge.target_id.clone()
        } else {
            edge.source_id.clone()
        };

        steps.push(ProcessStepRecord {
            process_id,
            step: edge.step.unwrap_or(0),
            function_id,
            relation_uri: edge.uri.clone(),
        });
    }

    steps.sort_by_key(|step| (step.process_id.clone(), step.step, step.function_id.clone()));
    steps
}

fn derive_symbols(nodes: &[NodeRecord]) -> Vec<SymbolRecord> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for node in nodes {
        if node.name.trim().is_empty() {
            continue;
        }

        let norm = normalize_symbol(&node.name);
        if norm.is_empty() {
            continue;
        }

        let key = format!("{norm}|{}", node.id);
        if !seen.insert(key) {
            continue;
        }

        out.push(SymbolRecord {
            symbol_norm: norm,
            symbol: node.name.clone(),
            node_id: node.id.clone(),
            file_path: node.file_path.clone(),
            node_label: node.label.clone(),
        });
    }

    out
}

fn derive_hotspots(nodes: &[NodeRecord], edges: &[EdgeRecord]) -> Vec<HotspotRecord> {
    let node_file: HashMap<_, _> = nodes
        .iter()
        .map(|n| (n.id.clone(), n.file_path.clone()))
        .collect();

    let mut calls_by_file: HashMap<String, usize> = HashMap::new();
    for edge in edges {
        if edge.relation_type != "CALLS" {
            continue;
        }
        if let Some(file_path) = node_file.get(&edge.source_id) {
            *calls_by_file.entry(file_path.clone()).or_insert(0) += 1;
        }
    }

    let mut node_count_by_file: HashMap<String, usize> = HashMap::new();
    for node in nodes {
        if node.file_path.is_empty() {
            continue;
        }
        *node_count_by_file
            .entry(node.file_path.clone())
            .or_insert(0) += 1;
    }

    let mut files: HashSet<String> = calls_by_file.keys().cloned().collect();
    files.extend(node_count_by_file.keys().cloned());

    let mut hotspots = Vec::new();
    for file_path in files {
        let calls_count = calls_by_file.get(&file_path).copied().unwrap_or(0);
        let node_count = node_count_by_file.get(&file_path).copied().unwrap_or(0);
        let score = (calls_count as f64 * 10.0) + node_count as f64;
        hotspots.push(HotspotRecord {
            file_path,
            calls_count,
            node_count,
            score,
        });
    }

    hotspots.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.file_path.cmp(&b.file_path))
    });
    hotspots
}

fn derive_community_membership(nodes: &[NodeRecord]) -> Vec<CommunityMembershipRecord> {
    let mut out = Vec::new();

    for node in nodes {
        let Some(communities) = node
            .metadata
            .get("communities")
            .and_then(Value::as_array)
            .cloned()
        else {
            continue;
        };

        for community in communities {
            let Some(community_id) = community.as_str() else {
                continue;
            };
            out.push(CommunityMembershipRecord {
                community_id: community_id.to_string(),
                node_id: node.id.clone(),
                node_label: node.label.clone(),
                node_name: node.name.clone(),
            });
        }
    }

    out
}

fn normalize_symbol(input: &str) -> String {
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

fn parse_metadata_json(text: &str) -> Option<Value> {
    let marker = "metadata=";
    let start = text.find(marker)? + marker.len();
    let slice = text.get(start..)?.trim_start();
    let brace_pos = slice.find('{')?;
    let json_slice = &slice[brace_pos..];
    let mut depth = 0usize;
    let mut end = None;

    for (idx, ch) in json_slice.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
                if depth == 0 {
                    end = Some(idx + 1);
                    break;
                }
            }
            _ => {}
        }
    }

    let end = end?;
    serde_json::from_str(&json_slice[..end]).ok()
}

fn parse_line_value(text: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    text.lines()
        .find_map(|line| line.strip_prefix(&prefix).map(ToString::to_string))
}

fn parse_relation_type(text: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.strip_prefix("Relationship ").map(ToString::to_string))
}

fn parse_step_from_text(text: &str) -> Option<usize> {
    for line in text.lines() {
        if let Some(raw) = line.strip_prefix("step=") {
            if let Ok(v) = raw.trim().parse::<usize>() {
                return Some(v);
            }
        }
    }
    None
}

fn parse_node_label(text: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.strip_prefix("Node ").map(ToString::to_string))
}

fn parse_id_line(text: &str) -> Option<String> {
    parse_line_value(text, "id")
}

fn parse_name_line(text: &str) -> Option<String> {
    parse_line_value(text, "name")
}

fn parse_file_path_line(text: &str) -> Option<String> {
    parse_line_value(text, "filePath")
}
