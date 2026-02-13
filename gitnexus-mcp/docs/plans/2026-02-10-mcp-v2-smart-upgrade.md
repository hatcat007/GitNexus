# GitNexus MCP v0.3.0 — Smart Upgrade Design

> SDK v2 migration + router tool + auto-reconnect + compact payloads

## Overview

Full upgrade of `gitnexus-mcp` across 4 pillars:

```
+-----------------------------------------------------+
|                  gitnexus-mcp v0.3.0                |
|                                                     |
|  +--------------+  +---------------+  +------------+|
|  |  McpServer   |  |  Router Tool  |  |  Reconnect ||
|  |  (SDK v2)    |  |  (Smart)      |  |  Engine    ||
|  |              |  |               |  |            ||
|  | registerTool |  | NL -> tool    |  | Daemon <-> ||
|  | zod/v4       |  | plan+execute  |  | Browser    ||
|  | method str   |  |               |  | backoff    ||
|  +--------------+  +---------------+  +------------+|
|                                                     |
|  +-------------------------------------------------+|
|  |  Compact Transport Layer                        ||
|  |  - Compressed tool descriptions (76% smaller)   ||
|  |  - Truncated results (configurable maxTokens)   ||
|  |  - Session result cache (5min TTL)              ||
|  +-------------------------------------------------+|
+-----------------------------------------------------+
```

**Goals:**
- Minimize total LLM token spend (fewer calls AND smaller payloads)
- Both-side auto-reconnect (daemon + browser heal independently)
- Modern SDK (v1 Server -> v2 McpServer)

---

## Pillar 1: SDK v2 Migration

### Dependency Changes

```diff
# package.json
- "@modelcontextprotocol/sdk": "^1.0.0",
+ "@modelcontextprotocol/server": "^2.0.0",
+ "@modelcontextprotocol/core": "^2.0.0",
- "zod-to-json-schema": "^3.23.0",
- "zod": "^3.23.0",
+ "zod": "^3.25.0",   # supports zod/v4
```

### Import Changes

```diff
# server.ts
- import { Server } from '@modelcontextprotocol/sdk/server/index.js';
- import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
- import { CallToolRequestSchema, ListToolsRequestSchema, ... } from '@modelcontextprotocol/sdk/types.js';
+ import { McpServer, StdioServerTransport } from '@modelcontextprotocol/server';

# errors.ts
- import { CallToolResult } from '@modelcontextprotocol/sdk/types.js';
+ import type { CallToolResult } from '@modelcontextprotocol/server';
```

### Server Initialization

```diff
- const server = new Server(
-   { name: 'gitnexus', version: '0.1.0' },
-   { capabilities: { tools: {}, resources: {} } }
- );
+ const server = new McpServer(
+   { name: 'gitnexus', version: '0.3.0' },
+   { capabilities: { tools: {}, resources: {} } }
+ );
```

### Tool Registration (replaces ListTools + CallTool handlers)

```typescript
// BEFORE: Manual handler approach
server.setRequestHandler(ListToolsRequestSchema, async () => ({
  tools: GITNEXUS_TOOLS.map(tool => ({ name, description, inputSchema })),
}));

server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;
  const prefixedName = `gitnexus_${name}`;
  const validation = validateToolInput(prefixedName, args);
  // ... giant switch/if chain
});

// AFTER: Each tool self-contained
server.registerTool('search', {
  description: 'Keyword + semantic code search. Returns {name, type, filePath, code, connections[]}. Prefer route tool.',
  inputSchema: z.object({
    query: z.string().min(1),
    limit: z.number().int().min(1).max(100).default(10),
    groupByProcess: z.boolean().default(true),
  }),
}, async ({ query, limit, groupByProcess }) => {
  const result = await withResilience('search', () =>
    breaker.fire('search', { query, limit, groupByProcess })
  );
  return { content: [{ type: 'text', text: JSON.stringify(result) }] };
});
```

### What Gets Removed

- `tools.ts` — entire file (absorbed into server.ts registerTool calls)
- `zod-to-json-schema` dependency — SDK handles conversion automatically
- `toolSchemaMap` / `toolSchemas` / `validateToolInput()` from schemas.ts
- Manual `$schema` stripping logic
- The `gitnexus_` prefix mapping hack

### What schemas.ts Becomes

Keeps Zod schema definitions as named exports for reuse (router needs them), but drops all conversion/validation infrastructure:

```typescript
import * as z from 'zod/v4';

export const SearchSchema = z.object({
  query: z.string().min(1),
  limit: z.number().int().min(1).max(100).default(10),
  groupByProcess: z.boolean().default(true),
});

export const CypherSchema = z.object({
  query: z.string().min(1),
});

// ... remaining schemas, no conversion code
```

---

## Pillar 2: Router Tool (Plan + Execute)

### Purpose

Instead of the LLM reading 15 tool descriptions and guessing which to call, the `route` tool analyzes a natural-language question and either returns a plan or executes it directly.

### Tool Definition

```typescript
server.registerTool('route', {
  description: `Smart router for codebase queries. Analyzes your question and either plans or executes the optimal tool sequence.

Mode "execute" (default): Runs tools internally, returns results directly. ONE call = full answer.
Mode "plan": Returns tool call plan without executing. Use for manual control.

Call this FIRST — it replaces the need to read all tool descriptions.`,
  inputSchema: z.object({
    query: z.string().min(1).describe('Natural language question about the codebase'),
    mode: z.enum(['execute', 'plan']).default('execute'),
    maxTokens: z.number().int().min(100).max(10000).optional()
      .describe('Max tokens in response. Truncates results if exceeded.'),
  }),
}, async ({ query, mode, maxTokens }) => {
  const plan = routeQuery(query, client.context);

  if (mode === 'plan') {
    return { content: [{ type: 'text', text: JSON.stringify(plan) }] };
  }

  const results = await executePlan(plan, breaker);
  const output = maxTokens ? truncateResult(results, maxTokens) : results;
  return { content: [{ type: 'text', text: JSON.stringify(output) }] };
});
```

### Routing Logic (src/mcp/router.ts)

Deterministic pattern matching — no AI involved:

```typescript
interface RoutePlan {
  steps: RouteStep[];
  context?: { projectName: string; files: number; functions: number };
}

interface RouteStep {
  tool: string;
  params: Record<string, any>;
  reason: string;
  dependsOn?: number;  // index of step whose result feeds into this
}

export function routeQuery(query: string, context?: CodebaseContext): RoutePlan {
  const q = query.toLowerCase().trim();
  const plan: RoutePlan = { steps: [] };

  // Attach context summary if available (saves a separate context call)
  if (context) {
    plan.context = {
      projectName: context.projectName,
      files: context.stats.fileCount,
      functions: context.stats.functionCount,
    };
  }

  // Pattern: "what calls X" / "callers of X" / "who uses X"
  if (q.match(/\b(calls?|callers?\s+of|who\s+uses?|depends?\s+on)\b/)) {
    const symbol = extractSymbol(q);
    plan.steps.push({
      tool: 'cypher',
      params: {
        query: `MATCH (a)-[:CodeRelation {type: 'CALLS'}]->(b {name: "${symbol}"}) RETURN a.name, a.type, a.filePath`,
      },
      reason: 'Direct graph query for callers',
    });
    return plan;
  }

  // Pattern: "impact of changing X" / "what breaks if I change X"
  if (q.match(/\b(impact|breaks?|affect|blast\s*radius|ripple)\b/)) {
    const symbol = extractSymbol(q);
    plan.steps.push({
      tool: 'impact',
      params: { target: symbol, direction: 'upstream', maxDepth: 3 },
      reason: 'Impact analysis for change ripple effects',
    });
    return plan;
  }

  // Pattern: "explain X" / "deep dive X" / "tell me about X"
  if (q.match(/\b(explain|deep\s*dive|tell\s+me\s+about|analyze|understand)\b/)) {
    const symbol = extractSymbol(q);
    plan.steps.push({
      tool: 'deep_dive',
      params: { name: symbol },
      reason: 'Complete analysis: explore + impact + source code',
    });
    return plan;
  }

  // Pattern: "read file X" / "show me X" / "contents of X"
  if (q.match(/\b(read|show|contents?\s+of|open|view)\b.*\.(ts|js|py|go|rs|tsx|jsx|json|md)/)) {
    const filePath = extractFilePath(q);
    plan.steps.push({
      tool: 'read',
      params: { filePath },
      reason: 'Direct file read',
    });
    return plan;
  }

  // Pattern: regex-like or exact strings (TODO, console.log, error codes)
  if (q.match(/\b(todo|fixme|hack|console\.log|import\s|require\()\b/) || q.match(/[.*+?^${}()|[\]\\]/)) {
    plan.steps.push({
      tool: 'grep',
      params: { pattern: extractPattern(q), caseSensitive: false },
      reason: 'Regex pattern search in file contents',
    });
    return plan;
  }

  // Pattern: "architecture" / "overview" / "structure"
  if (q.match(/\b(architecture|overview|structure|map|clusters?|communities)\b/)) {
    plan.steps.push({
      tool: 'overview',
      params: {},
      reason: 'High-level codebase map',
    });
    return plan;
  }

  // Pattern: "similar to X" / "duplicates" / "like X"
  if (q.match(/\b(similar|duplicate|like|resembles?|same\s+as)\b/)) {
    const symbol = extractSymbol(q);
    plan.steps.push({
      tool: 'find_similar',
      params: { name: symbol },
      reason: 'Find structurally similar code',
    });
    return plan;
  }

  // Pattern: "trace from X to Y" / "how does X reach Y" / "flow"
  if (q.match(/\b(trace|flow|path|reaches?|connects?)\b/)) {
    const symbols = extractSymbols(q, 2);
    plan.steps.push({
      tool: 'trace_flow',
      params: { from: symbols[0], to: symbols[1] },
      reason: 'Trace execution path between symbols',
    });
    return plan;
  }

  // Pattern: multi-step — "explain X and what breaks if I change it"
  if (q.match(/\band\b/) && q.match(/\b(change|break|impact|modify)\b/)) {
    const symbol = extractSymbol(q);
    plan.steps.push({
      tool: 'search',
      params: { query: symbol, limit: 3 },
      reason: 'Find the symbol first',
    });
    plan.steps.push({
      tool: 'deep_dive',
      params: { name: '$result[0].name' },
      reason: 'Full analysis using search result',
      dependsOn: 0,
    });
    return plan;
  }

  // Default: semantic search (catch-all)
  plan.steps.push({
    tool: 'search',
    params: { query: q, limit: 10, groupByProcess: true },
    reason: 'Semantic + keyword search (default)',
  });

  return plan;
}
```

### Multi-Step Execution Engine

```typescript
export async function executePlan(
  plan: RoutePlan,
  breaker: CircuitBreaker
): Promise<ExecutionResult> {
  const stepResults: any[] = [];
  const startTime = Date.now();

  for (let i = 0; i < plan.steps.length; i++) {
    const step = plan.steps[i];
    let params = { ...step.params };

    // Resolve dependencies from previous step results
    if (step.dependsOn !== undefined && stepResults[step.dependsOn]) {
      params = resolveDependencies(params, stepResults[step.dependsOn]);
    }

    // Check cache first
    const cacheKey = `${step.tool}:${JSON.stringify(params)}`;
    const cached = getCached(cacheKey);
    if (cached) {
      stepResults.push(cached);
      continue;
    }

    // Execute with resilience
    try {
      const result = await withToolTimeout(step.tool, async () => {
        return breaker.fire(step.tool, params);
      });
      stepResults.push(result);
      setCache(cacheKey, result);
    } catch (error) {
      stepResults.push({
        error: error instanceof Error ? error.message : 'Unknown error',
        tool: step.tool,
      });
      // Don't break — continue with remaining steps that don't depend on this
    }
  }

  return {
    plan: plan.steps.map(s => ({ tool: s.tool, reason: s.reason })),
    results: stepResults,
    context: plan.context,
    totalDuration: Date.now() - startTime,
  };
}

function resolveDependencies(params: Record<string, any>, prevResult: any): Record<string, any> {
  const resolved = { ...params };
  for (const [key, value] of Object.entries(resolved)) {
    if (typeof value === 'string' && value.startsWith('$result')) {
      // Parse $result[0].name -> prevResult[0]?.name
      const path = value.replace('$result', '');
      resolved[key] = resolvePath(prevResult, path) || value;
    }
  }
  return resolved;
}
```

### Result Cache

```typescript
const resultCache = new Map<string, { data: any; timestamp: number }>();
const CACHE_TTL = 5 * 60 * 1000; // 5 minutes

function getCached(key: string): any | null {
  const entry = resultCache.get(key);
  if (!entry) return null;
  if (Date.now() - entry.timestamp > CACHE_TTL) {
    resultCache.delete(key);
    return null;
  }
  return entry.data;
}

function setCache(key: string, data: any): void {
  // Cap cache size to prevent memory leaks
  if (resultCache.size > 200) {
    const oldest = resultCache.keys().next().value;
    if (oldest) resultCache.delete(oldest);
  }
  resultCache.set(key, { data, timestamp: Date.now() });
}
```

### Result Truncation

```typescript
export function truncateResult(result: any, maxTokens: number): any {
  const json = JSON.stringify(result, null, 2);
  const estimatedTokens = Math.ceil(json.length / 4); // ~4 chars per token

  if (estimatedTokens <= maxTokens) return result;

  // For ExecutionResult: truncate each step's results
  if (result.results && Array.isArray(result.results)) {
    const perStep = Math.floor(maxTokens / result.results.length);
    return {
      ...result,
      results: result.results.map((r: any) => truncateSingle(r, perStep)),
      truncated: true,
    };
  }

  return truncateSingle(result, maxTokens);
}

function truncateSingle(result: any, maxTokens: number): any {
  if (Array.isArray(result)) {
    const json = JSON.stringify(result);
    const itemSize = Math.ceil(json.length / Math.max(result.length, 1));
    const maxItems = Math.max(1, Math.floor((maxTokens * 4) / itemSize));
    return {
      items: result.slice(0, maxItems),
      truncated: result.length > maxItems,
      total: result.length,
      showing: Math.min(maxItems, result.length),
    };
  }
  return result;
}
```

---

## Pillar 3: Both-Side Auto-Reconnect

### Daemon Side — Peer Mode Reconnect

In `src/bridge/websocket-server.ts`, the Peer's `onclose` handler currently gives up. Add active reconnection:

```typescript
// In startAsPeer() — update the onclose handler
ws.on('close', () => {
  if (!this.started) {
    resolve(false);
  } else {
    this.client = null;
    this._context = null;
    this.notifyContextListeners();

    // NEW: Auto-reconnect to Hub
    if (this.shouldReconnect) {
      this.scheduleReconnect(this.reconnectAttempt, async () => {
        try {
          const success = await this.startAsPeer();
          if (success) {
            this.resetReconnectState();
          }
          // If failed, onclose fires again -> schedules next attempt
        } catch {
          // scheduleReconnect handles the next attempt
        }
      });
    }
  }
});
```

**Hub mode note:** The Hub is a WebSocket server — it can't "reconnect" to the browser. It passively waits. But the Hub's `handleHubDisconnect()` now logs expected reconnect timing based on the browser's backoff schedule, giving operators visibility.

### Browser Side — New Reconnect Engine

In `gitnexus/src/core/mcp/mcp-client.ts`:

```typescript
export class MCPBrowserClient {
  // NEW: Reconnection state
  private reconnectAttempt = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private maxReconnectAttempts = 20;
  private shouldReconnect = true;

  async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        this.ws = new WebSocket(`ws://localhost:${this.port}`);

        this.ws.onopen = () => {
          // Reset backoff on successful connection
          this.reconnectAttempt = 0;
          this.notifyConnectionListeners(true);
          if (this.pendingContext) this.sendContext(this.pendingContext);
          resolve();
        };

        this.ws.onerror = () => {
          // onclose fires after onerror — reconnect handled there
          this.notifyConnectionListeners(false);
          reject(new Error('Failed to connect to MCP bridge'));
        };

        this.ws.onmessage = (event) => { /* unchanged */ };

        // NEW: Auto-reconnect on close
        this.ws.onclose = () => {
          this.ws = null;
          this.notifyConnectionListeners(false);

          if (this.shouldReconnect && this.reconnectAttempt < this.maxReconnectAttempts) {
            this.scheduleReconnect();
          } else if (this.reconnectAttempt >= this.maxReconnectAttempts) {
            console.warn('[MCP] Max reconnect attempts reached. Call connect() manually.');
          }
        };
      } catch (error) {
        reject(error);
      }
    });
  }

  // NEW: Exponential backoff with jitter
  private scheduleReconnect() {
    const baseDelay = 500;
    const maxDelay = 30000;
    const exponential = Math.min(maxDelay, baseDelay * Math.pow(2, this.reconnectAttempt));
    const delay = Math.floor(exponential * (0.5 + Math.random() * 0.5)); // Jitter

    this.reconnectAttempt++;
    console.log(`[MCP] Reconnecting in ${delay}ms (attempt ${this.reconnectAttempt}/${this.maxReconnectAttempts})`);

    this.reconnectTimer = setTimeout(() => {
      this.connect().catch(() => {
        // onclose will fire -> schedules next attempt automatically
      });
    }, delay);
  }

  // UPDATED: User-initiated disconnect stops auto-reconnect
  disconnect() {
    this.shouldReconnect = false;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.ws?.close();
    this.ws = null;
  }

  // NEW: Re-enable auto-reconnect (for UI toggle)
  enableAutoReconnect() {
    this.shouldReconnect = true;
    this.reconnectAttempt = 0;
  }
}
```

### Reconnect Behavior Matrix

| Scenario | Daemon (Hub) | Daemon (Peer) | Browser |
|----------|-------------|---------------|---------|
| Browser tab closes | Waits (passive) | N/A | N/A |
| Browser tab refreshes | Waits (passive) | N/A | Auto-reconnects on page load |
| Daemon restarts | N/A | N/A | Detects onclose, retries with backoff |
| Hub crashes | N/A | Retries startAsPeer() with backoff | Retries with backoff |
| Network blip | Waits (passive) | Retries with backoff | Retries with backoff |
| User clicks Disconnect | N/A | N/A | shouldReconnect=false, no retry |
| Max attempts reached | N/A | Stops, logs warning | Stops, logs warning |

---

## Pillar 4: Compact Descriptions

### Before vs After — All 15 Tools

Tool descriptions shrink from ~2,000 total tokens to ~480 tokens. The detailed guidance moves into the router's internal pattern matching.

| Tool | Before (tokens) | After (tokens) | After Description |
|------|----------------|----------------|-------------------|
| context | ~60 | ~25 | "Get codebase stats, hotspots, and structure. Called automatically by route tool." |
| search | ~73 | ~19 | "Keyword + semantic code search. Returns {name, type, filePath, code, connections[]}." |
| cypher | ~120 | ~30 | "Execute read-only Cypher against the code knowledge graph. See route tool for query help." |
| grep | ~55 | ~18 | "Regex search in file contents. Returns {filePath, line, lineNumber, match}." |
| read | ~45 | ~15 | "Read file content. Returns {filePath, content, language, lines}." |
| explore | ~55 | ~20 | "Deep dive on a symbol, cluster, or process. Shows membership and connections." |
| overview | ~45 | ~18 | "Codebase map: all clusters and processes with member counts." |
| impact | ~100 | ~22 | "Analyze change impact. Returns affected nodes by depth with risk assessment." |
| highlight | ~35 | ~12 | "Highlight nodes in the graph visualization." |
| diff | ~55 | ~18 | "Compare current index with previous. Shows added/modified/deleted files." |
| deep_dive | ~55 | ~18 | "Complete symbol analysis in one call: explore + impact + source code." |
| review_file | ~55 | ~18 | "Full file context: content, symbols, dependencies, cluster, processes." |
| trace_flow | ~45 | ~18 | "Trace execution path between symbols or from an entry point." |
| find_similar | ~45 | ~18 | "Find structurally similar code using cluster and connection patterns." |
| test_impact | ~70 | ~20 | "Risk assessment for file changes. Returns riskScore, affected processes, suggested tests." |
| **route** | — | ~80 | Full description (primary entry point for LLMs) |
| **TOTAL** | **~2,000** | **~480** | **76% reduction** |

---

## File Changes Summary

### Files to Modify

| File | Changes |
|------|---------|
| `package.json` | Replace SDK deps, remove zod-to-json-schema, bump to v0.3.0 |
| `src/mcp/server.ts` | Major rewrite: McpServer + registerTool() for 16 tools, absorbs tools.ts |
| `src/mcp/schemas.ts` | Keep Zod schemas, remove conversion/validation infrastructure |
| `src/mcp/errors.ts` | Update import path |
| `src/bridge/websocket-server.ts` | Add Peer auto-reconnect, fix "disonnected" typo, add shouldReconnect flag |
| `gitnexus/src/core/mcp/mcp-client.ts` | Add auto-reconnect engine with backoff + jitter |

### Files to Create

| File | Purpose |
|------|---------|
| `src/mcp/router.ts` | routeQuery() + executePlan() + result cache + truncation |

### Files to Delete

| File | Reason |
|------|--------|
| `src/mcp/tools.ts` | Absorbed into server.ts registerTool() calls |

---

## Implementation Order

```
Phase 1: SDK v2 Migration
  1. Update package.json dependencies
  2. Rewrite server.ts with McpServer + registerTool()
  3. Simplify schemas.ts (remove conversion code)
  4. Update errors.ts imports
  5. Delete tools.ts
  -> Test: verify all 15 tools via MCP inspector

Phase 2: Router Tool
  6. Create src/mcp/router.ts with routeQuery() patterns
  7. Implement executePlan() with multi-step chaining
  8. Add result cache (Map with 5min TTL, 200 entry cap)
  9. Add truncateResult() with maxTokens support
  10. Register route tool in server.ts
  -> Test: verify plan mode returns correct tool suggestions
  -> Test: verify execute mode returns results directly
  -> Test: verify multi-step chaining with dependsOn

Phase 3: Auto-Reconnect
  11. Add Peer reconnect loop in websocket-server.ts
  12. Add browser auto-reconnect in mcp-client.ts
  13. Add shouldReconnect flag + enableAutoReconnect()
  -> Test: kill daemon, verify browser reconnects
  -> Test: kill Hub, verify Peer reconnects
  -> Test: user Disconnect stops auto-reconnect

Phase 4: Compact Descriptions
  14. Shorten all tool descriptions in server.ts registerTool() calls
  -> Test: verify LLM can still discover and use tools correctly
```

---

## Token Cost Analysis

| Metric | Before (v0.2.0) | After (v0.3.0) | Improvement |
|--------|-----------------|----------------|-------------|
| Tool listing tokens | ~2,000 | ~480 | 76% smaller |
| Avg calls per question | 3-5 | 1-2 | 60-80% fewer |
| Avg tokens per interaction | 4,000-6,000 | 500-1,000 | ~85% reduction |
| Reconnect behavior | Manual | Automatic | Hands-free |
| SDK version | v1 (low-level) | v2 (registerTool) | Modern API |
