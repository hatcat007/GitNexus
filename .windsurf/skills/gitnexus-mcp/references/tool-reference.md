# GitNexus MCP — Full Tool Reference

All 15 tools with complete parameter specs, return formats, and usage examples.

---

## 1. context

Get codebase context. **Call this FIRST** before any other tool.

**Params:** none

**Returns:** Markdown with:
- Project name and stats (files, functions, classes, interfaces, methods)
- Hotspots (top connected nodes with name, type, filePath, connection count)
- Directory structure (TOON format)
- Available tools list and graph schema

**Example:** `context()`

---

## 2. search

Hybrid search combining BM25 keyword matching + semantic vector similarity (RRF fusion).

**Params:**
| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `query` | string (required) | — | Natural language or keyword query |
| `limit` | int 1–100 | 10 | Max results |
| `groupByProcess` | bool | true | Group results by execution process |

**Returns:** Ranked list per result: name, type, filePath, line range, cluster membership, process grouping, 1-hop graph connections, score.

**Use for:** conceptual queries — "authentication middleware", "database connection", "error handling"
**Not for:** exact strings (use grep), structural queries (use cypher/impact)

**Example:** `search({query: "authentication middleware", limit: 15})`

---

## 3. cypher

Execute read-only Cypher queries against the code knowledge graph.

**Params:**
| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `query` | string (required) | — | Cypher query. Must start with MATCH/RETURN/WITH. |

**Returns:** Tabular results with column headers from RETURN clause. Max 50 rows displayed.

**Key rules:**
- All edges: `[:CodeRelation {type: 'CALLS'}]` — NOT `[:CALLS]`
- Use `label(n)` not `labels(n)` (KuzuDB)
- Always add `LIMIT`
- Write queries are blocked (CREATE, DELETE, SET, DROP, etc.)

**See:** [cypher-patterns.md](cypher-patterns.md) for full examples and KuzuDB gotchas.

**Example:** `cypher({query: "MATCH (f:Function) RETURN f.name, f.filePath LIMIT 10"})`

---

## 4. grep

Regex search for exact patterns across all source files.

**Params:**
| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `pattern` | string (required) | — | Regex pattern. Escape special chars. |
| `caseSensitive` | bool | false | Case-sensitive matching |
| `maxResults` | int 1–500 | 50 | Max results |

**Returns:** Array of `{filePath, line, lineNumber, match}`.

**Use for:** exact strings, TODOs, error codes, specific identifiers, import statements
**Not for:** conceptual queries (use search), graph structure (use cypher/impact)

**Example:** `grep({pattern: "TODO|FIXME", maxResults: 20})`

---

## 5. read

Read file content from the codebase.

**Params:**
| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `filePath` | string (required) | — | Full or partial file path |
| `startLine` | int | — | Start line (1-indexed) |
| `endLine` | int | — | End line (≥ startLine) |

**Returns:** `{filePath, content, language, lines}`

**Rule:** ALWAYS read before concluding. Never guess from names alone.

**Example:** `read({filePath: "src/utils/helpers.ts", startLine: 1, endLine: 50})`

---

## 6. explore

Deep dive on a specific symbol, cluster, or process.

**Params:**
| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string (required) | — | Name or ID of entity |
| `type` | enum: symbol, cluster, process (required) | — | Entity type |

**Returns by type:**
- **symbol:** cluster membership, process participation, incoming/outgoing connections with edge types and confidence
- **cluster:** all members (up to 50), cohesion score, description, processes touching it
- **process:** step-by-step execution trace with file paths, clusters traversed

**Example:** `explore({name: "AuthService", type: "symbol"})`

---

## 7. overview

Get the full codebase architecture map.

**Params:**
| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `showClusters` | bool | true | Include clusters |
| `showProcesses` | bool | true | Include processes |
| `limit` | int 1–100 | 20 | Max items per section |

**Returns:** All clusters with member counts and cohesion scores, all processes with step counts, cross-cluster call dependencies, critical paths.

**Example:** `overview({limit: 30})`

---

## 8. impact

Analyze the impact of changing a code element. Handles multi-hop traversal, deduplication, and risk scoring automatically.

**Params:**
| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `target` | string (required) | — | Function, class, or file name |
| `direction` | enum: upstream, downstream (required) | — | upstream = what depends on this; downstream = what this depends on |
| `maxDepth` | int 1–10 | 3 | Traversal depth |
| `relationTypes` | string[] | ["CALLS","IMPORTS","EXTENDS","IMPLEMENTS"] | Edge types to follow |
| `includeTests` | bool | false | Include test files |
| `minConfidence` | float 0–1 | 0.7 | Min edge confidence |

**Returns:** Per depth level: `Type|Name|File:Line|EdgeType|Confidence%`. Plus: affected processes (with step positions), affected clusters (direct/indirect), risk level (LOW/MEDIUM/HIGH/CRITICAL), call-site code snippets for d=1 results.

**Key:** Results are trusted graph analysis. Do NOT re-validate with cypher.

**Example:** `impact({target: "executeQuery", direction: "upstream", maxDepth: 2})`

---

## 9. highlight

Highlight nodes in the GitNexus graph visualization.

**Params:**
| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `nodeIds` | string[] (required, min 1) | — | Node IDs to highlight |
| `color` | string | — | Optional highlight color |

**Returns:** Confirmation. User sees nodes glow in graph view.

**Example:** `highlight({nodeIds: ["Function:validateUser", "Function:hashPassword"]})`

---

## 10. diff

Compare current codebase index with previous version.

**Params:**
| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `baseline` | string | "last_index" | Baseline to compare against |
| `filter` | enum: all, added, modified, deleted | "all" | Filter by change type |
| `includeContent` | bool | false | Include line-level diffs (slower) |

**Returns:** `{added[], modified[], deleted[], unchanged: number, summary}`

**Example:** `diff({filter: "modified", includeContent: true})`

---

## 11. deep_dive

Complete analysis of a code symbol in ONE call. Combines explore + impact + read.

**Params:**
| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string (required) | — | Symbol name |

**Returns:** `{symbol, source, impact, connections}` — cluster membership, process participation, downstream impact, risk level, actual source code.

**Use instead of:** calling explore, impact, and read separately.

**Example:** `deep_dive({name: "handleRequest"})`

---

## 12. review_file

Full review context for a file in ONE call.

**Params:**
| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `filePath` | string (required) | — | File path |

**Returns:** `{file, content, symbols[], cluster, processes[], dependencies, complexity}` — file content, all defined symbols, cluster membership, processes, imports and reverse-imports.

**Example:** `review_file({filePath: "src/core/auth.ts"})`

---

## 13. trace_flow

Trace execution path between two symbols, or from an entry point.

**Params:**
| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `from` | string (required) | — | Source symbol name |
| `to` | string | — | Target symbol (omit to find all processes containing `from`) |
| `maxSteps` | int 1–20 | 10 | Max steps in trace |

**Returns:** `{paths: [{steps: [{name, type, filePath, code}]}]}` — each step includes code snippet.

**Example:** `trace_flow({from: "handleLogin", to: "sendEmail", maxSteps: 8})`

---

## 14. find_similar

Find structurally similar code to a given symbol using cluster membership and connection patterns.

**Params:**
| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string (required) | — | Symbol name |
| `limit` | int 1–20 | 5 | Max similar results |

**Returns:** `{target, similar: [{name, type, filePath, sharedCluster, sharedConnections, similarity}]}`

**Use for:** finding duplicates, understanding patterns, refactoring candidates.

**Example:** `find_similar({name: "validateInput", limit: 10})`

---

## 15. test_impact

Risk assessment for a set of file changes (like a PR).

**Params:**
| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `changedFiles` | string[] (required, min 1) | — | File paths that changed |
| `maxDepth` | int 1–5 | 2 | Dependency traversal depth |
| `suggestTests` | bool | true | Suggest related test files |

**Returns:**
- `riskScore`: 0–100
- `riskLevel`: critical | high | medium | low
- `affectedProcesses`: processes touching changed files
- `affectedClusters`: communities impacted
- `impactedFiles`: files depending on changed files (by depth)
- `suggestedTests`: test files related to impacted code
- `summary`: human-readable risk assessment

Also highlights all affected nodes in the graph visualization.

**Example:** `test_impact({changedFiles: ["src/auth/login.ts", "src/auth/session.ts"]})`
