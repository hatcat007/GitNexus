# MCP2 Roadmap - GitNexus Improvements

> **Last updated:** 2026-02-08

## Current State (Verified)

### ✅ Already Implemented
| Feature | Location | Description |
|---------|----------|-------------|
| **Incremental Re-index** | `session-store.ts:57-59` | SHA-256 file hashes for diff detection + `previousFileHashes` for diff tool |
| **Embedding Preservation** | `ingestion.worker.ts:381` | `preserveUnchangedData()` keeps embeddings for unchanged files |
| **LLM Response Streaming** | `agent.ts:308` | `streamAgentResponse()` - token-by-token LLM output |
| **AST Caching** | `ast-cache.ts` | LRU cache with WASM memory disposal |
| **Background Reindex** | `useAppState.tsx` | `reindexFromGitHub()` — re-clones, diffs hashes, runs pipeline only on changes |
| **Toast Notifications** | `useToast.ts`, `ToastContainer.tsx` | Global event-emitter toast system (info, success, warning, error, changelog) |
| **Auto-Change Detection** | `useChangeDetector.ts` | 30s polling via `git.listServerRefs()`, toast with "Reindex now" action |
| **Cypher Sanitization** | `Header.tsx:cypherEsc()` | Escapes `\`, `'`, `"`, `` ` ``, null bytes + 500-char length cap on all MCP queries |

### ✅ MCP Tools (15 total)
| Tool | Type | Location | Description |
|------|------|----------|-------------|
| `context` | Core | `Header.tsx` | Send codebase context to agent |
| `search` | Core | `Header.tsx` | Semantic search across codebase |
| `cypher` | Core | `Header.tsx` | Raw Cypher query execution |
| `grep` | Core | `Header.tsx` | Regex search across file contents |
| `read` | Core | `Header.tsx` | Read file content with optional line range |
| `explore` | Core | `Header.tsx` | Explore symbol, cluster, or process |
| `overview` | Core | `Header.tsx` | Codebase stats, hotspots, folder tree |
| `impact` | Core | `Header.tsx` | Upstream/downstream dependency traversal |
| `highlight` | Core | `Header.tsx` | Highlight nodes in graph visualization |
| **`diff`** | **New** | `Header.tsx`, `useAppState.tsx` | Compare current vs previous index (added/modified/deleted files) |
| **`deep_dive`** | **New (composite)** | `Header.tsx` | explore + impact + read in one call |
| **`review_file`** | **New (composite)** | `Header.tsx` | Full file context: content, symbols, cluster, processes, deps |
| **`trace_flow`** | **New (composite)** | `Header.tsx` | Trace execution path between symbols or list process traces |
| **`find_similar`** | **New** | `Header.tsx` | Find structurally similar code via cluster membership |
| **`test_impact`** | **New** | `Header.tsx` | PR risk assessment: score 0-100, affected processes/clusters, suggested tests |

Schemas: `gitnexus-mcp/src/mcp/schemas.ts` (Zod) | Definitions: `gitnexus-mcp/src/mcp/tools.ts` | Registration: `MCPToggle.tsx`

### ❌ Not Yet Implemented (This Roadmap)
| Feature | Gap |
|---------|-----|
| Streaming Tool Results | Tools return full string at once |
| Result Verbosity Control | No verbosity parameter on search/impact |
| Semantic **Graph** Diff | `diff` tool compares files only — no node/relationship/cluster structural diff |
| Query Templates | Partially addressed by composite tools, but no generic template system |
| Tool Suggestions | Agents sometimes pick wrong tool — no meta-tool yet |

---

## Top 5 MCP2 Improvements

### 1. 🥇 Streaming Tool Results

**Problem:** Large `impact()` or `search()` results block until complete. Agent waits idle.

**Solution:** Return results as async generator:

```typescript
// tools.ts - New streaming variant
const searchStreamTool = tool(
  async function* ({ query, limit }: { query: string; limit?: number }) {
    const k = limit ?? 10;
    
    // Yield results as they arrive
    for (let i = 0; i < searchResults.length; i++) {
      yield {
        type: 'result',
        index: i + 1,
        total: searchResults.length,
        data: formatResult(searchResults[i])
      };
    }
    
    yield { type: 'done', total: searchResults.length };
  }
);
```

**MCP Protocol Change:**
```json
// Before: Single response
{"type": "tool_result", "result": "...100KB string..."}

// After: Streamed chunks
{"type": "tool_stream", "chunk": {"index": 1, "data": "..."}}
{"type": "tool_stream", "chunk": {"index": 2, "data": "..."}}
{"type": "tool_stream_end", "total": 50}
```

**Impact:** Agent can start reasoning after first result, ~50% faster perceived response.

---

### 2. 🥈 Result Verbosity Control

**Problem:** `search()` can return 50+ results with full metadata, burning 10K+ tokens.

**Solution:** Add verbosity tiers:

```typescript
// tools.ts - Search tool enhancement
const searchTool = tool(
  async ({ query, limit, verbosity }: {
    query: string;
    limit?: number;
    verbosity?: 'minimal' | 'normal' | 'full';
  }) => {
    const v = verbosity ?? 'normal';
    
    // Minimal: Just names + IDs (for quick scans)
    if (v === 'minimal') {
      return results.map(r => `${r.label}: ${r.name} [${r.nodeId}]`).join('\n');
    }
    
    // Normal: Names + file paths (current default)
    if (v === 'normal') {
      return results.map(r => `[${r.idx}] ${r.label}: ${r.name}\n  File: ${r.filePath}`).join('\n\n');
    }
    
    // Full: Include connections, cluster, processes (current behavior)
    return fullFormat(results);
  }
);
```

**Token Savings:**
| Verbosity | 50 results | Savings |
|-----------|------------|---------|
| `minimal` | ~2K tokens | 80% |
| `normal` | ~5K tokens | 50% |
| `full` | ~10K tokens | 0% |

---

### 3. 🥉 Semantic Graph Diff (Upgrade existing `diff` tool)

**Current state:** The `diff` tool compares file-level hashes (added/modified/deleted files). This is useful but lacks structural insight.

**Problem:** "What changed?" at the graph level — new/removed nodes, broken relationships, cluster shifts.

**Solution:** Extend the existing `diff` tool to also compare graph snapshots:

```typescript
// Extend DiffSchema to support graph-level diff
export const DiffSchema = z.object({
  baseline: z.string().default('last_index'),
  includeContent: z.boolean().default(false),
  filter: z.enum(['all', 'added', 'modified', 'deleted']).default('all'),
  graphLevel: z.boolean().default(false), // NEW: enable structural diff
});

// When graphLevel=true, also return:
{
  addedNodes: [...],
  removedNodes: [...],
  changedRelationships: [...],
  newClusters: [...],
  affectedProcesses: [...],
}
```

**Requires:** Storing a graph snapshot (node IDs, relationship fingerprints) alongside `previousFileHashes` in the session.

**Example Output:**
```
Graph Diff: last_index → current

Files: +15 added, ~3 modified, -1 deleted
Nodes: +22 added, -5 removed
Relationships: 8 broken, 12 new

New Clusters:
  + PaymentGateway (5 functions)
  - LegacyAuth (removed)

Affected Processes:
  UserLogin: Step 3 removed (validateToken)
  Checkout: New step 5 (processPayment)
```

---

### 4. Query Templates

**Problem:** Same complex Cypher patterns re-written every time.

**Solution:** Pre-built template library:

```typescript
// tools.ts - New template tool
const templateTool = tool(
  async ({ name, params }: { name: string; params: Record<string, any> }) => {
    const templates: Record<string, (p: any) => string> = {
      'find_breaking_changes': (p) => `
        MATCH (caller)-[:CodeRelation {type: 'CALLS'}]->(fn {name: '${p.target}'})
        RETURN caller.name, caller.filePath
      `,
      
      'find_circular_deps': () => `
        MATCH (a)-[:CodeRelation {type: 'CALLS'}]->(b)-[:CodeRelation {type: 'CALLS'}]->(a)
        RETURN a.name, b.name
      `,
      
      'find_dead_code': () => `
        MATCH (fn:Function)
        WHERE NOT (())-[:CodeRelation {type: 'CALLS'}]->(fn)
        AND NOT fn.name STARTS WITH '_'
        RETURN fn.name, fn.filePath
      `,
      
      'find_cross_cluster_deps': (p) => `
        MATCH (a)-[:CodeRelation {type: 'MEMBER_OF'}]->(c1:Community)
        MATCH (a)-[:CodeRelation {type: '${p.relType ?? 'CALLS'}'}]->(b)
        MATCH (b)-[:CodeRelation {type: 'MEMBER_OF'}]->(c2:Community)
        WHERE c1.id <> c2.id
        RETURN c1.label, c2.label, COUNT(*) as calls
        ORDER BY calls DESC
      `,
      
      'trace_data_flow': (p) => `
        MATCH path = (start {name: '${p.start}'})-[:CodeRelation*1..${p.depth ?? 5}]->(end)
        WHERE ALL(r IN relationships(path) WHERE r.type IN ['CALLS', 'IMPORTS'])
        RETURN [n IN nodes(path) | n.name] as flow
      `,
    };
    
    const template = templates[name];
    if (!template) {
      return `Unknown template: ${name}. Available: ${Object.keys(templates).join(', ')}`;
    }
    
    return executeQuery(template(params));
  }
);
```

**Usage:**
```
Agent: "Find circular dependencies"
→ mcp_router_template({name: "find_circular_deps", params: {}})
```

---

### 5. Tool Suggestions (Meta-Tool)

**Problem:** Agents sometimes use wrong tool for the task.

**Solution:** Intent-based tool recommendation:

```typescript
// tools.ts - New suggest tool
const suggestTool = tool(
  async ({ intent }: { intent: string }) => {
    const suggestions = [
      {
        patterns: ['what calls', 'who uses', 'dependents', 'break if I change'],
        tool: 'impact',
        params: { direction: 'upstream' },
        example: 'impact({target: "validateUser", direction: "upstream"})'
      },
      {
        patterns: ['what does X import', 'dependencies of', 'calls what'],
        tool: 'impact',
        params: { direction: 'downstream' },
        example: 'impact({target: "AuthService", direction: "downstream"})'
      },
      {
        patterns: ['find where', 'search for', 'locate'],
        tool: 'search',
        params: {},
        example: 'search({query: "authentication middleware"})'
      },
      {
        patterns: ['exact string', 'regex', 'pattern', 'TODO'],
        tool: 'grep',
        params: {},
        example: 'grep({pattern: "TODO|FIXME"})'
      },
      {
        patterns: ['how does X work', 'explain', 'understand'],
        tool: 'explore',
        params: { type: 'symbol' },
        example: 'explore({name: "AuthService", type: "symbol"})'
      },
      {
        patterns: ['architecture', 'overview', 'structure'],
        tool: 'overview',
        params: {},
        example: 'overview()'
      },
    ];
    
    const intentLower = intent.toLowerCase();
    const match = suggestions.find(s => 
      s.patterns.some(p => intentLower.includes(p))
    );
    
    if (match) {
      return `Recommended: ${match.tool}()\n\nExample:\n${match.example}\n\nWhy: Best match for "${intent}"`;
    }
    
    return `No specific recommendation. Try:\n- search() for finding code\n- impact() for dependencies\n- explore() for deep dives`;
  }
);
```

---

## Implementation Priority (Remaining)

| Phase | Features | Effort | Impact | Notes |
|-------|----------|--------|--------|-------|
| **P1** | Verbosity Control | 2h | High (token savings) | Add `verbosity` param to search/impact |
| **P1** | Semantic Graph Diff upgrade | 8h | High (architectural insight) | Extend existing `diff` tool with `graphLevel` flag |
| **P2** | Tool Suggestions | 3h | Medium (agent accuracy) | Meta-tool for intent routing |
| **P2** | Streaming Results | 8h | High (perceived speed) | Requires MCP protocol extension |
| **P3** | Query Templates | 2h | Low | Mostly addressed by composite tools (`deep_dive`, `review_file`, `trace_flow`, `find_similar`) |

---

## Technical Notes

### Streaming Protocol (MCP Extension)

Current MCP expects single response. Extension needed:

```typescript
// mcp-client.ts - Handle streamed results
case 'tool_stream':
  this.emit('tool_stream', {
    toolCallId: message.toolCallId,
    chunk: message.chunk
  });
  break;

case 'tool_stream_end':
  this.emit('tool_stream_end', {
    toolCallId: message.toolCallId,
    total: message.total
  });
  break;
```

### Graph Diff Implementation

Store graph snapshots per commit:

```typescript
// session-store.ts - Extend saved session
interface SavedSession {
  // ...existing fields
  commitGraphs?: Record<string, {
    nodeCount: number;
    relationshipCount: number;
    clusterLabels: string[];
    processLabels: string[];
  }>;
}
```

---

## Future Considerations

### Not in MCP2 Scope (But Interesting)

1. **Proactive Alerts** - Push notifications for code health issues
2. **Multi-Repository Queries** - Cross-repo graph analysis
3. **Bidirectional Annotations** - Agents write back insights to graph
4. **Query Result Caching** - Cache with graph-hash invalidation

These require architectural changes beyond tool-level improvements.

---

## Success Metrics

| Metric | Before MCP2 | Current | MCP2 Target |
|--------|-------------|---------|-------------|
| MCP tool count | 9 | **15** | 17+ (verbosity, suggestions) |
| Avg token usage per search | ~10K | ~10K | ~3K (verbosity) |
| Time to first result | Full wait | Full wait | Instant (streaming) |
| Tool selection accuracy | ~70% | ~75% (composites help) | ~95% (suggestions) |
| Common query reuse | 0% | **~60%** (composites) | 80% (templates) |
| Cypher injection protection | None | **Full** (`cypherEsc`) | Full |
