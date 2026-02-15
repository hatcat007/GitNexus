# GitNexus MV2 AI Bible (Agent-First MCP v1)

```json
{
  "contractVersion": "gitnexus.ai-bible.v1",
  "schemaVersion": "gitnexus.mcp.v1",
  "toolSetVersion": "gitnexus.tools.v1",
  "jsonSchemaRefs": {
    "envelope": "./schemas/mcp-envelope.v1.schema.json",
    "toolResults": "./schemas/mcp-tool-results.v1.schema.json",
    "contract": "./AI_BIBLE_MV2_MCP.contract.v1.json"
  },
  "responsePolicy": {
    "defaultBudgetBytes": 65536,
    "pagination": "cursor",
    "truncationCode": "RESULT_TRUNCATED"
  },
  "retrievalPolicy": {
    "primary": "graph_exact_then_lexical",
    "semantic": "fallback_only"
  }
}
```

## 1) Capsule Mental Model
- A capsule is a deterministic memory graph serialized as MV2 frames.
- Frame classes:
  - Node frames: `mv2://nodes/*`, `mv2://communities/*`, `mv2://processes/*`
  - Relation frames: `mv2://relations/*`
  - Meta frames: `mv2://meta/manifest`, `mv2://meta/ai-bible/*`
- Canonical reasoning rule:
  - Facts about entities come from node frames.
  - Facts about dependencies/flows come from relation frames.
  - Export/runtime invariants come from manifest + AI Bible meta frames.

## 2) URI + Track Semantics
- URI contract:
  - `mv2://nodes/<node-id>`
  - `mv2://relations/<relation-id>`
  - `mv2://communities/<community-id>`
  - `mv2://processes/<process-id>`
  - `mv2://meta/manifest`
  - `mv2://meta/ai-bible/<section>`
- Track contract:
  - `nodes/<NodeLabel>`
  - `relations/<RelationType>`
  - `communities`
  - `processes`
  - `files`
  - `meta`

## 3) Tool Routing Matrix

| Task | Primary Sequence | Fallback |
|---|---|---|
| Symbol resolution | `symbol_lookup -> node_get` | `text_search` |
| Root-cause from symptom | `text_search -> symbol_lookup -> call_trace -> impact_analysis -> file_snippet` | `query_explain` |
| Change impact before edit | `symbol_lookup -> node_get -> neighbors_get -> callers_of -> callees_of -> impact_analysis` | `query_explain` |
| Subsystem architecture extraction | `community_list -> process_list -> process_get -> neighbors_get -> manifest_get` | `text_search` |
| Process walkthrough | `process_list -> process_get -> file_snippet` | `call_trace` |
| File-level understanding | `file_outline -> file_snippet -> neighbors_get` | `text_search` |

## 4) Retrieval Ladder + Confidence Thresholds
- Retrieval ladder:
  1. Graph exact (`nodeId`, `edgeId`, `processId`, exact symbol normalization)
  2. Lexical (`text_search`, tokenized deterministic scoring)
  3. Graph expansion + rerank (`neighbors_get`, `impact_analysis`, call traversals)
  4. Semantic fallback (only when lexical confidence is below threshold)
- Confidence tiers:
  - `high`: `score >= 0.85`
  - `medium`: `0.60 <= score < 0.85`
  - `low`: `score < 0.60`
- Mandatory confidence payload in every tool response:
  - `score`, `tier`, `factors[]`, `warnings[]`

## 5) Deep-Understanding Playbooks

### Root Cause From Symptom
1. Run `text_search` for stack/symptom tokens.
2. Resolve candidate symbols with `symbol_lookup`.
3. Confirm node facts with `node_get`.
4. Expand execution path via `call_trace`.
5. Quantify blast radius via `impact_analysis`.
6. Pull bounded evidence with `file_snippet`.

### Change Impact Before Edit
1. Resolve edit target via `symbol_lookup`.
2. Load inbound/outbound callers (`callers_of`, `callees_of`).
3. Expand neighborhood using `neighbors_get`.
4. Rank high-risk files from `impact_analysis.hotspots`.

### Subsystem Architecture Extraction
1. `community_list` for modular boundaries.
2. `process_list` and `process_get` for functional flows.
3. `neighbors_get` around top community/process nodes.
4. `manifest_get` for global graph distribution.

### Process Comprehension (`STEP_IN_PROCESS`)
1. Identify process node with `process_list`.
2. Resolve ordered steps via `process_get.steps[*].step`.
3. Use `file_snippet` on each function node for grounded narrative.

## 6) Anti-Patterns + Hallucination Guards
- Do not invent IDs, file paths, labels, or relations not returned by tools.
- Do not infer call direction without `CALLS` edge evidence.
- Do not return full file contents by default.
- Always include explicit unknowns when confidence is medium/low.
- If response is truncated or paginated, continue with cursor before concluding.

## 7) Deterministic Prompt Templates

### Template: Impact Check
```text
Objective: Determine deterministic impact for <symbol-or-node>.
Steps:
1) symbol_lookup(query="<symbol>")
2) node_get(nodeId="<id>")
3) callers_of(nodeId="<id>")
4) callees_of(nodeId="<id>")
5) impact_analysis(nodeId="<id>", maxDepth=3)
Output:
- Confirmed node facts
- Direct callers/callees
- Impacted node count + top hotspots
- Confidence block verbatim
```

### Template: Process Walkthrough
```text
Objective: Explain process <process>.
Steps:
1) process_list(limit=50)
2) process_get(processId="<id>")
3) file_snippet(nodeId="<step-function-id>", maxChars=1200) for key steps
Output:
- Ordered process steps
- Step-to-function mapping
- Bounded snippets as evidence
- Confidence block verbatim
```

## 8) Troubleshooting + Retry + Pagination
- Retryable errors:
  - `RATE_LIMITED`, `TIMEOUT`, `INTERNAL_ERROR`, `RESULT_TRUNCATED`
- Non-retryable errors:
  - `INVALID_ARGUMENT`, `NOT_FOUND`, `CAPSULE_INCOMPATIBLE`
- Cursor continuation:
  - If `pagination.nextCursor` exists, continue same tool with `cursor=<nextCursor>`.
- Stale/absent sidecar index:
  - First read may trigger index rebuild.
  - If read fails, return `CAPSULE_INCOMPATIBLE` with `traceId`.

## 9) Versioning + Compatibility
- MV2 metadata keys:
  - `mv2SchemaVersion`
  - `exportSchemaVersion`
  - `aiBibleVersion`
- Runtime schema:
  - `schemaVersion = gitnexus.mcp.v1`
- Compatibility approach:
  - Parse current and legacy URI/text conventions.
  - Compute `capsuleCapabilities` at load time.

## 10) SLO/SLA + Ops Runbook
- Target p95 for common reads (`symbol_lookup`, `node_get`, `neighbors_get`, `text_search`, `file_snippet`): `<300ms` on warm index.
- Response budget:
  - Default `64KB`, cursor-based continuation.
- Security defaults:
  - Bearer auth required.
  - Per-key rate limiting enabled.
  - Metadata-only logs in production.
- Runtime artifacts:
  - Sidecar index: `<capsule>.index.v1.sqlite`
  - Export retention: 24h by default.

