---
name: gitnexus-mcp
description: "GitNexus MCP server for code intelligence — a knowledge graph of any codebase with 15 tools for search, analysis, and visualization. Use when working with a codebase loaded in GitNexus, when needing to understand code architecture, trace dependencies, analyze change impact, find similar code, or explore execution flows. Triggers on: code analysis, codebase questions, dependency tracing, impact analysis, architecture review, refactoring planning, PR risk assessment, or any task involving the GitNexus MCP tools (context, search, cypher, grep, read, explore, overview, impact, highlight, diff, deep_dive, review_file, trace_flow, find_similar, test_impact)."
---

# GitNexus MCP — Code Knowledge Graph

GitNexus builds a knowledge graph from any Git repository (in-browser via KuzuDB + WebGPU embeddings) and exposes 15 tools over MCP for AI agents. The graph contains code symbols (files, functions, classes, methods, interfaces), functional clusters (Leiden communities), and execution processes (traced flows).

## Prerequisite

GitNexus must be running in the user's browser with a repository loaded. Tools will return `BROWSER_DISCONNECTED` errors otherwise. The MCP server bridges stdio ↔ WebSocket to the browser.

## Graph Schema

```
Nodes: File, Folder, Function, Class, Interface, Method, Community, Process
Edge:  CodeRelation (single table) with 'type' property
```

**Edge types:**
- `CALLS` — method invocation or constructor injection
- `IMPORTS` — file-level import statement
- `EXTENDS` / `IMPLEMENTS` — class inheritance
- `CONTAINS` / `DEFINES` — structural (folder→file, file→function)
- `MEMBER_OF` — symbol → Community (functional cluster)
- `STEP_IN_PROCESS` — symbol → Process (with `step` order property)

**Edge properties:** `type` (string), `confidence` (float 0–1), `reason` (string, for fuzzy matches)

## Tool Selection — Pick the Right Tool

| Task | Tool | NOT this |
|------|------|----------|
| Understand codebase first | **context** | — (always call first) |
| Find code by concept/meaning | **search** | grep (text only) |
| Find exact string/pattern | **grep** | search (semantic) |
| Structural query (callers, imports, counts) | **cypher** | impact (overkill for simple queries) |
| Read source code | **read** | — (always read before citing) |
| Codebase architecture map | **overview** | cypher (overview is pre-built) |
| Deep dive on one entity | **explore** | cypher (explore is pre-built) |
| Change impact / dependency analysis | **impact** | manual multi-hop cypher |
| Full symbol analysis in one call | **deep_dive** | explore+impact+read separately |
| Full file review in one call | **review_file** | read+cypher separately |
| Trace execution path A→B | **trace_flow** | manual cypher path queries |
| Find duplicate/similar code | **find_similar** | — |
| PR risk assessment | **test_impact** | impact (test_impact is multi-file) |
| Compare index versions | **diff** | — |
| Highlight nodes in graph UI | **highlight** | — (visual only) |

## Core Workflow

1. **context** → Call FIRST. Returns project stats, hotspots, folder tree, tool guidance.
2. **search** or **grep** → Discover relevant code.
3. **read** → See actual source. NEVER guess from names alone.
4. **explore** / **cypher** / **impact** → Trace graph connections.
5. **highlight** → Visualize findings in the graph UI.

## Common Patterns

**"What does X do?"**
→ `search(X)` → `read(filePath)` → `explore(X, symbol)` → cite with `[[file:line]]`

**"What would break if I changed X?"**
→ `impact(X, upstream)` → `read` key d=1 callers → cite

**"Show me the architecture"**
→ `overview()` → `explore` top clusters → produce mermaid diagram

**"How does data flow from A to B?"**
→ `trace_flow(from=A, to=B)` → `read` each step → mermaid

**"Is this PR safe to merge?"**
→ `test_impact(changedFiles=[...])` → review riskScore, affected processes

**"Find all uses of pattern"**
→ `grep(pattern)` → `read` top matches → cite

**"Find duplicated logic"**
→ `find_similar(name)` → `read` candidates → compare

**"What changed since last index?"**
→ `diff()` → `review_file` on modified files

## Tool Quick Reference (15 tools)

### Discovery Tools
- **context** — No params. Call first. Returns project stats, hotspots, tree.
- **search** — `{query, limit?, groupByProcess?}` — Hybrid BM25+semantic search. Results grouped by process with cluster context.
- **grep** — `{pattern, caseSensitive?, maxResults?}` — Regex search across files. For exact strings only.
- **read** — `{filePath, startLine?, endLine?}` — Read file content. Supports partial paths.

### Architecture Tools
- **overview** — `{showClusters?, showProcesses?, limit?}` — Full codebase map: clusters, processes, cross-cluster deps.
- **explore** — `{name, type: symbol|cluster|process}` — Deep dive on one entity.

### Analysis Tools
- **impact** — `{target, direction: upstream|downstream, maxDepth?, relationTypes?, includeTests?, minConfidence?}` — Change impact with risk scoring.
- **deep_dive** — `{name}` — Composite: explore + impact + read in one call.
- **review_file** — `{filePath}` — Composite: read + symbols + cluster + processes + deps.
- **trace_flow** — `{from, to?, maxSteps?}` — Trace execution path between symbols.
- **find_similar** — `{name, limit?}` — Find structurally similar code via cluster + connection patterns.
- **test_impact** — `{changedFiles[], maxDepth?, suggestTests?}` — PR risk assessment: score 0–100, affected processes/clusters, suggested tests.
- **diff** — `{baseline?, filter?, includeContent?}` — Compare current vs previous index.

### Visualization
- **highlight** — `{nodeIds[], color?}` — Highlight nodes in graph UI.
- **cypher** — `{query}` — Raw Cypher against the graph. Read-only enforced. See [cypher-patterns.md](references/cypher-patterns.md) for examples and KuzuDB gotchas.

## Cypher Quick Rules

- All edges use `CodeRelation` table with `type` property: `[:CodeRelation {type: 'CALLS'}]`
- Use `label(n)` not `labels(n)` (KuzuDB syntax)
- Read-only only — CREATE/DELETE/SET/DROP are blocked
- Always add `LIMIT` to prevent OOM on large graphs
- For detailed patterns and KuzuDB-specific syntax: see [references/cypher-patterns.md](references/cypher-patterns.md)

## Error Handling

Errors return structured JSON with `code`, `message`, `suggestion`, and `retryable` fields.

| Error Code | Meaning | Action |
|------------|---------|--------|
| `VALIDATION_ERROR` | Bad params | Fix input per suggestion |
| `CYPHER_FORBIDDEN` | Write query blocked | Use read-only clauses only |
| `TIMEOUT` | Tool took too long | Simplify query, retry |
| `CIRCUIT_OPEN` | Repeated failures | Wait `retryAfter` seconds |
| `BROWSER_DISCONNECTED` | No GitNexus browser | Ask user to open GitNexus |

## Full Tool Reference

For complete parameter documentation, return formats, and examples for all 15 tools: see [references/tool-reference.md](references/tool-reference.md)
