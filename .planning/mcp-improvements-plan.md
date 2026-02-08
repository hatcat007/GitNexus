# MCP Improvements Plan

**Scope**: Implement 3 new MCP capabilities — `diff`, composite chains, and `test_impact`  
**Priority**: #3 → #5 → #8 (low-to-high effort, all high impact)

---

## Current Architecture (for reference)

```
AI Agent (Cursor/Claude/Windsurf)
  │ stdio (MCP protocol)
  ▼
gitnexus-mcp/src/mcp/server.ts     ← MCP server (validates, dispatches)
  │
  ▼
gitnexus-mcp/src/bridge/websocket-server.ts  ← WebSocket bridge (hub/peer)
  │ ws://localhost:54319
  ▼
gitnexus/src/core/mcp/mcp-client.ts          ← Browser client (receives tool calls)
  │
  ▼
gitnexus/src/components/MCPToggle.tsx         ← Registers handlers (search, cypher, etc.)
  │
  ▼
gitnexus/src/components/Header.tsx            ← Implements handlers (Cypher queries, grep, etc.)
```

**Tool dispatch flow**:
1. Agent calls tool via MCP → `server.ts` validates with Zod → calls `bridge.callTool(name, params)`
2. Bridge sends WS message to browser → `mcp-client.ts` receives → looks up handler
3. Handler in `MCPToggle.tsx` / `Header.tsx` executes → returns result
4. Result flows back: browser → bridge → server → agent

**Existing tools**: context, search, cypher, grep, read, explore, overview, impact, highlight

---

## Part 1: `diff` Tool

### What It Does
Returns a structured diff between the current index and the previous one (or between any two points). Leverages the file-hash comparison logic already built for `reindexFromGitHub`.

### Schema
```typescript
// gitnexus-mcp/src/mcp/schemas.ts
export const DiffSchema = z.object({
  /** Compare against: 'last_index' (default) or a specific session ID */
  baseline: z.string().default('last_index'),
  /** Include file content diffs (slower, more detail) */
  includeContent: z.boolean().default(false),
  /** Filter by change type */
  filter: z.enum(['all', 'added', 'modified', 'deleted']).default('all'),
});
```

### Tool Description
```
Compare current codebase index with previous version.
Shows added/modified/deleted files with optional content diffs.
Use after reindex to understand what changed.
Returns: { added[], modified[], deleted[], unchanged: number, summary: string }
```

### Implementation

#### A. Browser Side — Compute Diff On Demand

**File: `gitnexus/src/hooks/useAppState.tsx`**
- Add `getDiffFromLastIndex()` method to `AppState` interface
- Implementation:
  1. Load current session from IndexedDB (`dbGetSession(currentSessionId)`)
  2. Current file hashes are already in the saved session's `fileHashes`
  3. Load the *previous* session's `fileHashes` (need to track `previousSessionId` or store a snapshot)
  4. Diff the two hash maps → `{ added, modified, deleted, unchanged }`
  5. If `includeContent` is true, do a line-by-line diff for modified files using a simple unified-diff algorithm
- **Shortcut for MVP**: Store `lastIndexFileHashes` in the session whenever a reindex completes. The diff is always "current vs pre-reindex". This avoids needing multiple session IDs.

**Changes needed:**
1. `src/services/session-store.ts` — Add `previousFileHashes?: Record<string, string>` to `SavedSession`
2. `src/hooks/useAppState.tsx` — In `reindexFromGitHub`, before overwriting, save `oldFileHashes` as `previousFileHashes` in the session
3. `src/hooks/useAppState.tsx` — Add `getIndexDiff()` method that loads session and diffs `previousFileHashes` vs `fileHashes`
4. `src/components/MCPToggle.tsx` — Register `diff` handler
5. `src/components/Header.tsx` — Pass `onDiff` prop implementing `getIndexDiff()`

#### B. MCP Server Side

**File: `gitnexus-mcp/src/mcp/schemas.ts`**
- Add `DiffSchema` and `DiffInput` type
- Add to `toolSchemaMap` as `gitnexus_diff`

**File: `gitnexus-mcp/src/mcp/tools.ts`**
- Add `diff` tool definition to `GITNEXUS_TOOLS` array

**File: `gitnexus-mcp/src/mcp/server.ts`**
- No changes needed (generic dispatch handles new tools automatically)

---

## Part 2: Composite Tool Chains

### What They Do
Single tools that internally execute multiple operations and return a combined result. Reduces agent round-trips from ~4 calls to 1.

### Chains to Implement

#### `deep_dive(symbol)`
**Purpose**: Complete analysis of a code symbol in one call.  
**Internal steps**:
1. `explore(symbol, 'symbol')` → get cluster, processes, connections
2. `impact(symbol, 'downstream', 3)` → get affected nodes
3. `read(filePath)` → get source code (from explore result)
4. Merge into structured response

**Schema**:
```typescript
export const DeepDiveSchema = z.object({
  name: z.string().min(1),
});
```

**Returns**:
```json
{
  "symbol": { "name", "type", "filePath", "cluster", "processes" },
  "source": "...code...",
  "impact": { "downstream": [...], "riskLevel": "medium" },
  "connections": { "callers": [...], "callees": [...], "imports": [...] }
}
```

#### `review_file(filePath)`
**Purpose**: Full review context for a file.  
**Internal steps**:
1. `read(filePath)` → get content
2. `cypher` → find all symbols defined in file + their connections
3. `cypher` → find which processes this file participates in
4. `cypher` → find which community/cluster this file belongs to
5. Merge into review report

**Schema**:
```typescript
export const ReviewFileSchema = z.object({
  filePath: z.string().min(1),
});
```

**Returns**:
```json
{
  "file": { "path", "language", "lines" },
  "content": "...",
  "symbols": [{ "name", "type", "line" }],
  "cluster": { "name", "cohesion" },
  "processes": [{ "name", "role" }],
  "dependencies": { "imports": [...], "importedBy": [...] },
  "complexity": { "symbolCount", "connectionCount", "processCount" }
}
```

#### `trace_flow(from, to?)`
**Purpose**: Trace execution path between two symbols (or from entry point).  
**Internal steps**:
1. If `to` provided: find shortest path via Cypher `shortestPath`
2. If only `from`: find all processes containing `from`, return traces
3. For each step in the path: `read` the relevant code snippet
4. Return ordered trace with code context

**Schema**:
```typescript
export const TraceFlowSchema = z.object({
  from: z.string().min(1),
  to: z.string().optional(),
  maxSteps: z.number().int().min(1).max(20).default(10),
});
```

#### `find_similar(name)`
**Purpose**: Find structurally similar code (same cluster, similar connections).  
**Internal steps**:
1. Find the target node's cluster
2. Find all members of that cluster
3. Rank by connection similarity (shared callers/callees)
4. Return top matches with context

**Schema**:
```typescript
export const FindSimilarSchema = z.object({
  name: z.string().min(1),
  limit: z.number().int().min(1).max(20).default(5),
});
```

### Implementation Strategy

**Key decision**: Composites run **entirely in the browser**. The MCP server just dispatches the single tool name; the browser handler orchestrates the sub-calls internally using the existing `runQuery`, `semanticSearch`, `fileContents` etc. This avoids multiple WS round-trips.

**Changes needed:**

1. **`gitnexus-mcp/src/mcp/schemas.ts`** — Add 4 new schemas + types
2. **`gitnexus-mcp/src/mcp/tools.ts`** — Add 4 new tool definitions
3. **`gitnexus/src/components/MCPToggle.tsx`** — Register 4 new handlers + add props
4. **`gitnexus/src/components/Header.tsx`** — Implement 4 composite handlers using existing `runQuery`, `fileContents`, `semanticSearch`

---

## Part 3: `test_impact` Tool

### What It Does
Given a list of changed files (like from a PR), traverse the graph to find all affected code, assign risk scores based on centrality and coupling, and return a structured risk report.

### Schema
```typescript
export const TestImpactSchema = z.object({
  /** List of changed file paths */
  changedFiles: z.array(z.string().min(1)).min(1),
  /** How deep to trace dependencies */
  maxDepth: z.number().int().min(1).max(5).default(2),
  /** Include suggested test files */
  suggestTests: z.boolean().default(true),
});
```

### Tool Description
```
Risk assessment for a set of file changes (like a PR).
Traces graph dependencies to find all affected code.

Returns:
- riskScore: 0-100 (based on centrality, coupling, process disruption)
- riskLevel: critical | high | medium | low
- affectedProcesses: processes that touch changed files
- affectedClusters: communities impacted
- impactedFiles: files that depend on changed files (grouped by depth)
- suggestedTests: test files related to impacted code
- summary: human-readable risk assessment
```

### Implementation

**Browser-side handler** (in `Header.tsx`):
1. For each changed file, find all symbols defined in it via Cypher
2. For each symbol, run downstream traversal (callers, importers) up to `maxDepth`
3. Collect all affected processes (via `STEP_IN_PROCESS` edges)
4. Collect all affected communities (via `MEMBER_OF` edges)
5. Compute risk score:
   - Base: number of affected downstream nodes / total nodes
   - Multipliers: cross-community impact (×1.5), process disruption (×1.3), hotspot files (×1.5)
   - Cap at 100
6. Map score to level: 0-25 low, 25-50 medium, 50-75 high, 75-100 critical
7. If `suggestTests`: find files with "test", "spec", or "__tests__" in path that import any affected file
8. Highlight all affected nodes in graph visualization
9. Return structured report

**Changes needed:**
1. **`gitnexus-mcp/src/mcp/schemas.ts`** — Add `TestImpactSchema`
2. **`gitnexus-mcp/src/mcp/tools.ts`** — Add `test_impact` tool definition
3. **`gitnexus/src/components/MCPToggle.tsx`** — Register `test_impact` handler + prop
4. **`gitnexus/src/components/Header.tsx`** — Implement risk assessment logic

---

## File Change Summary

### `gitnexus-mcp/src/mcp/schemas.ts`
- Add: `DiffSchema`, `DeepDiveSchema`, `ReviewFileSchema`, `TraceFlowSchema`, `FindSimilarSchema`, `TestImpactSchema`
- Add all to `toolSchemaMap`
- Export all input types

### `gitnexus-mcp/src/mcp/tools.ts`
- Add 6 new entries to `GITNEXUS_TOOLS`: `diff`, `deep_dive`, `review_file`, `trace_flow`, `find_similar`, `test_impact`

### `gitnexus/src/services/session-store.ts`
- Add `previousFileHashes?: Record<string, string>` to `SavedSession`

### `gitnexus/src/hooks/useAppState.tsx`
- Add `getIndexDiff()` method
- Save `previousFileHashes` during reindex
- Expose `getIndexDiff` in AppState interface

### `gitnexus/src/components/MCPToggle.tsx`
- Add props: `onDiff`, `onDeepDive`, `onReviewFile`, `onTraceFlow`, `onFindSimilar`, `onTestImpact`
- Register 6 new handlers

### `gitnexus/src/components/Header.tsx`
- Implement all 6 handlers using `runQuery`, `fileContents`, `semanticSearch`, `setHighlightedNodeIds`, `triggerNodeAnimation`

---

## Implementation Order

```
Step 1: MCP schemas + tool definitions (gitnexus-mcp)
        → schemas.ts: 6 new Zod schemas
        → tools.ts: 6 new tool entries
        → Build MCP server to verify

Step 2: diff tool (browser side)
        → session-store.ts: add previousFileHashes
        → useAppState.tsx: save previousFileHashes + getIndexDiff()
        → MCPToggle.tsx: register diff handler
        → Header.tsx: implement onDiff

Step 3: Composite chains (browser side)
        → MCPToggle.tsx: register deep_dive, review_file, trace_flow, find_similar
        → Header.tsx: implement all 4 composite handlers

Step 4: test_impact tool (browser side)
        → MCPToggle.tsx: register test_impact handler
        → Header.tsx: implement risk assessment logic

Step 5: Build both packages + verify
        → tsc --noEmit (gitnexus)
        → npm run build (gitnexus-mcp)
        → Manual test with Cascade MCP
```

---

## Risk / Open Questions

1. **`diff` baseline**: For MVP, diff is always "current vs pre-reindex". Supporting arbitrary session comparison would require a session picker UI — defer to later.
2. **Composite performance**: `deep_dive` runs ~4 Cypher queries sequentially. If any single query is slow (>2s), the composite could hit the 60s MCP timeout. Mitigate with `Promise.all` where queries are independent.
3. **`test_impact` accuracy**: Risk scores are heuristic-based. Graph centrality is a proxy for importance, not a guarantee. Document this in the tool description.
4. **`trace_flow` shortest path**: KuzuDB may not support `shortestPath()` natively. Fallback: BFS via multiple `MATCH` queries with increasing depth.
