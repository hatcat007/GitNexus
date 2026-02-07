# Stack Research

**Domain:** MCP Server Implementation (TypeScript/Node.js)
**Researched:** 2026-02-07
**Confidence:** HIGH

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `@modelcontextprotocol/sdk` | ^1.26.0 | MCP server SDK | Official TypeScript SDK; v1.x is production-ready (v2 is pre-alpha until Q1 2026); supports Streamable HTTP, stdio transports |
| `zod` | ^4.0.0 | Schema validation | Required peer dependency for MCP SDK; used for tool input/output schemas; v4 has improved performance and TypeScript inference |
| TypeScript | ^5.7.0 | Type system | ES2020+ target required for AJV imports; native ESM support; satisfies MCP SDK requirements |
| Node.js | >=20.0.0 | Runtime | Required for Web Crypto API (`globalThis.crypto`) used by SDK auth extensions; LTS support |

### Transport Layer

| Technology | Version | Purpose | When to Use |
|------------|---------|---------|-------------|
| Streamable HTTP | (built-in) | Remote server transport | **Recommended** for all remote MCP servers; supports POST requests, SSE notifications, session management, resumability |
| stdio | (built-in) | Local transport | For local CLI tools spawned by MCP hosts (Claude Desktop, etc.) |
| SSE (deprecated) | (built-in) | Legacy transport | Only for backwards compatibility with old clients |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `pino` | ^9.0.0 | Structured logging | HIGH confidence. Industry standard for Node.js; JSON output; child loggers for request context; MCP SDK recommends logging to stderr |
| `pino-pretty` | ^13.0.0 | Dev logging | For local development readability |
| `retry` | ^0.13.0 | Retry logic | Simple exponential backoff for transient failures |
| `opossum` | ^8.0.0 | Circuit breaker | Prevents cascade failures; opens circuit after threshold failures; half-open state for recovery |
| `lru-cache` | ^11.0.0 | In-memory caching | For frequently-accessed data (code search results, graph queries); TTL support; size-based eviction |
| `rate-limiter-flexible` | ^5.0.0 | Rate limiting | In-memory rate limiting with multiple algorithms; supports token bucket, sliding window |
| `zod` | ^4.0.0 | Input validation | Already a dependency; use for validating all tool inputs before processing |

### Monitoring & Observability

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `prom-client` | ^15.0.0 | Prometheus metrics | Expose metrics endpoint for scraping; standard for Kubernetes environments |
| `@opentelemetry/api` | ^1.9.0 | Tracing API | For distributed tracing; integrates with Jaeger, Zipkin, etc. |
| `@opentelemetry/sdk-node` | ^0.57.0 | OTel SDK | Full OpenTelemetry support; auto-instrumentation for HTTP, etc. |

### Middleware Packages (Optional)

| Package | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `@modelcontextprotocol/express` | ^1.0.0 | Express integration | If using Express.js as HTTP server; includes DNS rebinding protection |
| `@modelcontextprotocol/hono` | ^1.0.0 | Hono integration | If using Hono framework (lighter weight than Express) |
| `@modelcontextprotocol/node` | ^1.0.0 | Node.js HTTP | For raw Node.js `http` module integration |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| `vitest` | Unit/integration testing | MCP SDK uses Vitest; fast, ESM-native, TypeScript support |
| `@vitest/coverage-v8` | Code coverage | Native coverage with V8 |
| `tsx` | TypeScript execution | For running examples and development |
| `@anthropic-ai/mcp-inspector` | MCP debugging | Interactive debugging of MCP servers |

## Installation

```bash
# Core
npm install @modelcontextprotocol/sdk zod

# Logging
npm install pino pino-pretty

# Resilience
npm install retry opossum lru-cache rate-limiter-flexible

# Monitoring (optional)
npm install prom-client @opentelemetry/api @opentelemetry/sdk-node

# Dev dependencies
npm install -D vitest @vitest/coverage-v8 tsx typescript @types/node
```

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| `@modelcontextprotocol/sdk` v1.x | `@modelcontextprotocol/server` v2 (pre-alpha) | Only for experimental features; not production-ready until Q1 2026 |
| `pino` | `winston` | If you need multi-transport logging (files, external services); winston is heavier but more flexible |
| `pino` | `bunyan` | If you need ring buffer output for debugging |
| `lru-cache` | `keyv` | If you need multi-backend caching (Redis, etc.) |
| `lru-cache` | `cache-manager` | If you need tiered caching with multiple stores |
| `rate-limiter-flexible` | `express-rate-limit` | If only Express middleware needed; less flexible for non-Express |
| `vitest` | `jest` | If existing codebase uses Jest; Vitest is faster and ESM-native |
| `retry` | `async-retry` | If you prefer promise-based API with more options |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `console.log` for production | Not structured, no log levels, can't filter | `pino` with structured logging |
| `@modelcontextprotocol/sdk` v2 (main branch) | Pre-alpha, unstable, not production-ready | v1.x branch for production |
| HTTP+SSE transport | Deprecated, only for backwards compatibility | Streamable HTTP |
| stdout for logging | Breaks MCP protocol (stdio transport uses stdout) | stderr or `server.sendLoggingMessage()` |
| `axios` for HTTP | Heavy, fetch is native in Node 18+ | Native `fetch` or `undici` |
| Global state for sessions | Won't work in multi-node deployments | Use session stores with Streamable HTTP |
| JWT in client-side | MCP SDK handles auth; use `server.sendLoggingMessage()` for security events | Built-in auth hooks |

## Stack Patterns by Variant

**If deploying as CLI tool (stdio transport):**
- Use `StdioServerTransport`
- Log to stderr only (not stdout)
- No HTTP server needed
- Simpler error handling (process exit on fatal)

**If deploying as remote server (HTTP):**
- Use `StreamableHTTPServerTransport`
- Configure session management for stateful servers
- Add rate limiting middleware
- Expose `/metrics` for Prometheus
- Use DNS rebinding protection with `allowedHosts`

**If deploying multi-node (load balanced):**
- Use external session store (Redis) for session resumability
- Share event store across nodes
- Use sticky sessions or session-affinity routing

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| `@modelcontextprotocol/sdk@1.26.0` | `zod@^3.25 \|\| ^4.0` | SDK internally imports from `zod/v4`, backwards compatible with v3.25+ |
| `@modelcontextprotocol/sdk@1.26.0` | Node.js `>=20` | Web Crypto API required for JWT auth |
| `typescript@5.7` | `ES2020` target | Required for AJV imports in SDK |
| `vitest@^2.0` | `typescript@5.x` | Native TypeScript support |
| `pino@9` | Node.js `>=18.19` | Uses native performance hooks |

## Error Handling Patterns

### Retry with Exponential Backoff
```typescript
import retry from 'retry';

async function withRetry<T>(fn: () => Promise<T>, maxRetries = 3): Promise<T> {
  return new Promise((resolve, reject) => {
    const operation = retry.operation({ retries: maxRetries, factor: 2 });
    operation.attempt(async () => {
      try {
        resolve(await fn());
      } catch (err) {
        if (!operation.retry(err as Error)) {
          reject(operation.mainError());
        }
      }
    });
  });
}
```

### Circuit Breaker Pattern
```typescript
import CircuitBreaker from 'opossum';

const breaker = new CircuitBreaker(riskyOperation, {
  timeout: 3000,
  errorThresholdPercentage: 50,
  resetTimeout: 30000
});

breaker.fire(args).catch(err => {
  // Handle circuit open or operation failure
});
```

## Caching Strategy

### LRU Cache for Query Results
```typescript
import { LRUCache } from 'lru-cache';

const queryCache = new LRUCache<string, GraphResult>({
  max: 500,           // 500 items
  maxSize: 50 * 1024 * 1024, // 50MB
  sizeCalculation: (value) => JSON.stringify(value).length,
  ttl: 1000 * 60 * 5, // 5 minutes
});

// In tool handler
const cacheKey = `search:${query}:${limit}`;
const cached = queryCache.get(cacheKey);
if (cached) return { content: [{ type: 'text', text: JSON.stringify(cached) }] };
```

## Rate Limiting Pattern

```typescript
import { RateLimiterMemory } from 'rate-limiter-flexible';

const limiter = new RateLimiterMemory({
  points: 100,        // 100 requests
  duration: 60,       // per 60 seconds
  blockDuration: 60,  // block for 60 seconds on limit exceeded
});

// In transport middleware
try {
  await limiter.consume(clientId);
} catch {
  // Return 429 Too Many Requests
}
```

## Monitoring Integration

### Prometheus Metrics
```typescript
import client from 'prom-client';

const register = new client.Registry();
const toolCallsCounter = new client.Counter({
  name: 'mcp_tool_calls_total',
  help: 'Total MCP tool calls',
  labelNames: ['tool_name', 'status'],
  registers: [register]
});

// Expose at /metrics endpoint
```

### Structured Logging with Pino
```typescript
import pino from 'pino';

const logger = pino({
  level: process.env.LOG_LEVEL || 'info',
  ...(process.env.NODE_ENV !== 'production' && { transport: { target: 'pino-pretty' } })
});

// Use child loggers for request context
const requestLogger = logger.child({ requestId, sessionId });
```

## Sources

- `@modelcontextprotocol/sdk` GitHub — verified v1.26.0 current, v1.x production branch
- Official MCP TypeScript SDK documentation — server.md, client.md
- MCP Specification 2025-03-26 — transport requirements, protocol version
- npm registry — version verification for supporting libraries
- Node.js LTS schedule — runtime version requirements

---
*Stack research for: MCP Server Implementation*
*Researched: 2026-02-07*
