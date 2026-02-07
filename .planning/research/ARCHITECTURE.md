# GitNexus MCP Server Architecture

**Domain:** MCP Server Implementation (TypeScript/Node.js)
**Researched:** 2026-02-07
**Confidence:** HIGH

---

## 1. Current Architecture

### Overview

GitNexus uses a **Hub-and-Spoke** architecture with a WebSocket bridge connecting browser-based graph data to stdio-based MCP clients.

```
┌─────────────────┐     stdio      ┌──────────────────┐
│  AI Agent       │◄──────────────►│  MCP Server      │
│  (Cursor/Claude)│                │  (server.ts)     │
└─────────────────┘                └────────┬─────────┘
                                            │
                                            │ ToolCaller interface
                                            ▼
                                   ┌──────────────────┐
                                   │  WebSocketBridge │
                                   │  (Hub or Peer)   │
                                   └────────┬─────────┘
                                            │ WebSocket
                                            ▼
                                   ┌──────────────────┐
                                   │  GitNexus        │
                                   │  Browser App     │
                                   │  (Graph Engine)  │
                                   └──────────────────┘
```

### Component Analysis

#### `server.ts` (227 lines)
**Role:** MCP protocol handler

**Strengths:**
- Clean separation: MCP protocol ↔ ToolCaller interface
- Proper resource handling (codebase context as markdown)
- Graceful shutdown with signal handlers

**Gaps:**
- No input validation ( trusts upstream)
- No error classification (all errors treated equally)
- No request timeout enforcement at this layer
- No observability hooks

#### `tools.ts` (226 lines)
**Role:** Tool schema definitions

**Strengths:**
- Rich descriptions with usage guidance
- Clear input schemas

**Gaps:**
- No Zod schemas (just JSON Schema descriptions)
- No output validation
- Tool descriptions are static (no runtime context)

#### `websocket-server.ts` (398 lines)
**Role:** Hub/Peer WebSocket bridge

**Strengths:**
- Auto-detects Hub vs Peer mode
- Context broadcasting to all peers
- 30-second request timeout
- Basic handshake protocol

**Gaps:**
- No retry logic on disconnect
- No circuit breaker for failing requests
- No rate limiting
- No message size limits
- No origin validation (security comment exists but not implemented)
- Hardcoded timeout values

---

## 2. Recommended Layer Structure

### Production-Grade MCP Server Stack

```
┌────────────────────────────────────────────────────────────────────┐
│  LAYER 0: TRANSPORT (stdio / StreamableHTTP)                       │
│  - Raw protocol I/O                                                 │
│  - No business logic                                                │
└────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌────────────────────────────────────────────────────────────────────┐
│  LAYER 1: SECURITY & VALIDATION (FIRST LINE)                       │
│  - Input schema validation (Zod)                                    │
│  - Rate limiting (per-client/per-tool)                              │
│  - Request size limits                                              │
│  - Origin validation (for HTTP)                                     │
│  - Sanitize error messages (no stack traces to client)              │
└────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌────────────────────────────────────────────────────────────────────┐
│  LAYER 2: OBSERVABILITY (CROSS-CUTTING)                            │
│  - Structured logging (pino) with request context                   │
│  - Metrics collection (Prometheus)                                  │
│  - Distributed tracing (OpenTelemetry spans)                        │
│  - Audit trail for tool calls                                       │
└────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌────────────────────────────────────────────────────────────────────┐
│  LAYER 3: RESILIENCE (AROUND BACKEND CALLS)                        │
│  - Circuit breaker (opossum) — wrap backend calls                   │
│  - Retry with exponential backoff — for transient failures          │
│  - Timeout enforcement — kill slow requests                         │
│  - Bulkhead isolation — prevent resource exhaustion                 │
│                                                                     │
│  WHERE: Between validation and caching, wrapping each backend call  │
└────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌────────────────────────────────────────────────────────────────────┐
│  LAYER 4: CACHING (BEFORE BACKEND)                                  │
│  - LRU cache for read-heavy queries (search, cypher)                │
│  - Cache key: hash(method + sorted params)                          │
│  - TTL: 5 min for graph queries, 1 min for search                   │
│  - Cache invalidation: on graph mutation (future)                   │
│                                                                     │
│  WHERE: After resilience, before WebSocket bridge call              │
└────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌────────────────────────────────────────────────────────────────────┐
│  LAYER 5: BACKEND ADAPTER (WebSocketBridge)                         │
│  - Connection management (Hub/Peer)                                 │
│  - Message serialization                                            │
│  - Context synchronization                                          │
│  - Reconnection logic                                               │
└────────────────────────────────────────────────────────────────────┘
```

### Where Each Concern Lives

| Concern | Layer | Implementation |
|---------|-------|----------------|
| **Input Validation** | L1 Security | Zod schemas in tool definitions; validate before processing |
| **Rate Limiting** | L1 Security | `rate-limiter-flexible` before tool execution |
| **Caching** | L4 Caching | `lru-cache` wrapping `bridge.callTool()` for GET-like ops |
| **Circuit Breaker** | L3 Resilience | `opossum` wrapping `bridge.callTool()` |
| **Retry Logic** | L3 Resilience | `retry` package for transient WebSocket errors |
| **Logging** | L2 Observability | `pino` child loggers with requestId, toolName |
| **Metrics** | L2 Observability | `prom-client` counters/histograms |
| **Tracing** | L2 Observability | OpenTelemetry spans per tool call |
| **Error Sanitization** | L1 Security | Never expose internal errors to client |

---

## 3. Component Diagram (Data Flow)

### Request Flow

```
AI Agent                     MCP Server                              GitNexus Browser
   │                             │                                          │
   │  1. tool/call request       │                                          │
   │────────────────────────────►│                                          │
   │                             │                                          │
   │                      ┌──────┴──────┐                                   │
   │                      │ VALIDATE    │                                   │
   │                      │ - Zod check │                                   │
   │                      │ - Rate limit│                                   │
   │                      └──────┬──────┘                                   │
   │                             │                                          │
   │                      ┌──────┴──────┐                                   │
   │                      │ LOG & TRACE │                                   │
   │                      │ - Start span│                                   │
   │                      └──────┬──────┘                                   │
   │                             │                                          │
   │                      ┌──────┴──────┐                                   │
   │                      │ CACHE CHECK │                                   │
   │                      │ - LRU lookup│                                   │
   │                      └──────┬──────┘                                   │
   │                             │                                          │
   │               ┌─────────────┼─────────────┐                            │
   │               ▼             │             ▼                            │
   │        ┌──────────┐         │      ┌──────────┐                        │
   │        │ HIT:     │         │      │ MISS:    │                        │
   │        │ Return   │         │      │ Continue │                        │
   │        └────┬─────┘         │      └────┬─────┘                        │
   │             │               │           │                              │
   │             │               │    ┌──────┴──────┐                       │
   │             │               │    │ CIRCUIT     │                       │
   │             │               │    │ BREAKER     │                       │
   │             │               │    └──────┬──────┘                       │
   │             │               │           │                              │
   │             │               │    ┌──────┴──────┐      WebSocket        │
   │             │               │    │ RETRY LOGIC │─────────────────────►│
   │             │               │    └──────┬──────┘                      │
   │             │               │           │                              │
   │             │               │           │    2. Execute on graph       │
   │             │               │           │◄─────────────────────────────│
   │             │               │           │                              │
   │             │               │    ┌──────┴──────┐                       │
   │             │               │    │ CACHE SET   │                       │
   │             │               │    └──────┬──────┘                       │
   │             │               │           │                              │
   │             │               │    ┌──────┴──────┐                       │
   │             │               │    │ LOG & METRIC│                       │
   │             │               │    └──────┬──────┘                       │
   │             │               │           │                              │
   │  3. response               │           │                              │
   │◄───────────────────────────┼───────────┘                              │
   │                             │                                          │
```

### Error Flow

```
Error Type          Where Caught        What Happens
─────────────────────────────────────────────────────────
Validation error    L1 Security         Return 400-style MCP error
Rate limit          L1 Security         Return 429-style MCP error
Cache error         L4 Caching          Log, continue without cache
Circuit open        L3 Resilience       Return 503-style MCP error
Timeout             L3 Resilience       Return 408-style MCP error
Backend error       L5 Adapter          Retry if transient, else fail
Unknown error       L1 Security         Sanitize, return generic error
```

---

## 4. Build Order

### Phase 1: Foundation (Do First)
**Why:** Everything depends on these

```
1.1 Structured Logging (pino)
    - Replace console.error with pino
    - Add request context (requestId, toolName)
    - Log to stderr (MCP-safe)

1.2 Input Validation (Zod)
    - Convert tool schemas to Zod
    - Validate all inputs before processing
    - Return validation errors in MCP format
```

**Files to modify:**
- `src/mcp/tools.ts` → Add Zod schemas
- `src/mcp/server.ts` → Add validation layer
- New: `src/mcp/validation.ts`

### Phase 2: Resilience (Do Second)
**Why:** Prevent cascading failures

```
2.1 Request Timeout
    - Enforce timeout at tool handler level
    - Configurable per-tool timeouts
    - AbortController for cancellation

2.2 Circuit Breaker
    - Wrap WebSocketBridge.callTool()
    - Track failure rate
    - Open circuit on threshold

2.3 Retry Logic
    - Retry transient WebSocket errors
    - Exponential backoff
    - Max 3 retries
```

**Files to modify:**
- `src/bridge/websocket-server.ts` → Add circuit breaker
- New: `src/mcp/resilience.ts`

### Phase 3: Caching (Do Third)
**Why:** Reduce load, improve latency

```
3.1 LRU Cache
    - Cache search, cypher, grep results
    - TTL: 5 min default
    - Size limit: 50MB

3.2 Cache Invalidation
    - Clear cache on context change
    - Manual invalidation API (future)
```

**Files to modify:**
- `src/mcp/server.ts` → Add cache layer
- New: `src/mcp/cache.ts`

### Phase 4: Security (Do Fourth)
**Why:** Production hardening

```
4.1 Rate Limiting
    - Per-tool rate limits
    - Per-client tracking
    - Sliding window algorithm

4.2 Error Sanitization
    - Never expose stack traces
    - Generic error messages to client
    - Detailed errors to logs only

4.3 Request Size Limits
    - Max params size
    - Max string lengths
```

**Files to modify:**
- `src/mcp/server.ts` → Add rate limiter
- New: `src/mcp/security.ts`

### Phase 5: Observability (Do Last)
**Why:** Production visibility

```
5.1 Metrics
    - Tool call counter
    - Latency histogram
    - Error rate gauge
    - Cache hit/miss ratio

5.2 Tracing (Optional)
    - OpenTelemetry spans
    - Distributed trace context
```

**Files to modify:**
- `src/mcp/server.ts` → Add metrics
- New: `src/mcp/observability.ts`

---

## 5. Implementation Patterns

### Tool Handler with All Layers

```typescript
async function handleToolCall(request: CallToolRequest): Promise<CallToolResult> {
  const { name, arguments: args } = request.params;
  const requestId = uuid();
  const logger = rootLogger.child({ requestId, tool: name });
  
  // L1: Validate
  const schema = TOOL_SCHEMAS[name];
  if (!schema) throw new ToolNotFoundError(name);
  
  const parsed = schema.safeParse(args);
  if (!parsed.success) {
    return { content: [{ type: 'text', text: `Invalid input: ${parsed.error.message}` }], isError: true };
  }
  
  // L1: Rate limit
  try {
    await rateLimiter.consume(clientId);
  } catch {
    return { content: [{ type: 'text', text: 'Rate limit exceeded' }], isError: true };
  }
  
  // L4: Cache check (for read-only tools)
  if (isReadOnly(name)) {
    const cached = cache.get(cacheKey(name, parsed.data));
    if (cached) {
      logger.info({ cached: true }, 'Tool call completed');
      metrics.cacheHits.inc({ tool: name });
      return cached;
    }
  }
  
  // L3: Execute with resilience
  try {
    const result = await breaker.fire(async () => {
      return withTimeout(
        bridge.callTool(name, parsed.data),
        TOOL_TIMEOUTS[name] ?? 30000
      );
    });
    
    // L4: Cache set
    if (isReadOnly(name)) {
      cache.set(cacheKey(name, parsed.data), result);
    }
    
    logger.info({ duration: Date.now() - start }, 'Tool call completed');
    return result;
    
  } catch (err) {
    // L1: Sanitize error
    const safeMessage = isOperationalError(err) 
      ? err.message 
      : 'Internal server error';
    
    logger.error({ err }, 'Tool call failed');
    return { content: [{ type: 'text', text: safeMessage }], isError: true };
  }
}
```

---

## 6. Key Decisions

| Decision | Rationale |
|----------|-----------|
| **Zod over JSON Schema** | Runtime validation + type inference; MCP SDK uses Zod internally |
| **Circuit breaker around bridge** | Browser disconnect shouldn't freeze all tool calls |
| **Cache at MCP layer, not bridge** | Per-tool cache policies; bridge stays simple |
| **Logging to stderr** | MCP protocol uses stdout; must not pollute |
| **No Redis (yet)** | Single-node deployment; LRU cache sufficient for now |
| **30s default timeout** | Graph queries can be slow; balance UX vs resource protection |

---

## Sources

- Current codebase: `gitnexus-mcp/src/mcp/`, `gitnexus-mcp/src/bridge/`
- STACK.md: Recommended libraries and patterns
- MCP SDK documentation: Server patterns, error handling
- Production MCP server examples: resilience patterns

---
*Architecture research for: GitNexus MCP Enhancement*
*Researched: 2026-02-07*
