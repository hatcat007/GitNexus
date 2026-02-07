# Research: Phase 1 - Foundation & Security

**Phase:** 01 - Foundation & Security
**Researched:** 2026-02-07
**Confidence:** HIGH (based on codebase analysis + MCP SDK docs + project-level research)

---

## Current State Analysis

### Files to Modify

| File | Purpose | Current State |
|------|---------|---------------|
| `gitnexus-mcp/src/mcp/server.ts` | MCP server implementation | Basic error handling, graceful shutdown exists but minimal |
| `gitnexus-mcp/src/mcp/tools.ts` | Tool definitions | Plain object schemas, no Zod validation |
| `gitnexus-mcp/src/bridge/websocket-server.ts` | WebSocket bridge | Hardcoded 30s timeout, no validation, no logging |

### Existing Patterns

```typescript
// server.ts:196-207 - Current error handling (basic)
catch (error) {
  const message = error instanceof Error ? error.message : 'Unknown error';
  return {
    content: [{ type: 'text', text: `Error: ${message}` }],
    isError: true,
  };
}

// server.ts:215-225 - Graceful shutdown (exists)
process.on('SIGINT', async () => {
  client.disconnect?.();
  await server.close();
  process.exit(0);
});

// websocket-server.ts:380-386 - Hardcoded timeout
setTimeout(() => {
  if (this.pendingRequests.has(id)) {
    this.pendingRequests.delete(id);
    reject(new Error('Request timeout'));
  }
}, 30000);
```

---

## 1. Structured Error Responses (RELI-01)

### MCP Error Format

MCP expects `isError: true` + content array. Can include additional metadata.

### Recommended Pattern

```typescript
// src/mcp/errors.ts
export interface GitNexusError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
  suggestion?: string; // Actionable guidance for AI
}

export const ErrorCodes = {
  VALIDATION_ERROR: 'VALIDATION_ERROR',
  TOOL_NOT_FOUND: 'TOOL_NOT_FOUND',
  BROWSER_DISCONNECTED: 'BROWSER_DISCONNECTED',
  QUERY_TIMEOUT: 'QUERY_TIMEOUT',
  CYPHER_FORBIDDEN: 'CYPHER_FORBIDDEN',
  INTERNAL_ERROR: 'INTERNAL_ERROR',
} as const;

export function formatError(error: GitNexusError): CallToolResult {
  return {
    content: [
      {
        type: 'text',
        text: JSON.stringify({
          error: true,
          code: error.code,
          message: error.message,
          details: error.details,
          suggestion: error.suggestion,
        }, null, 2),
      },
    ],
    isError: true,
  };
}

// Usage in handler:
try {
  const result = await client.callTool(name, args);
  return { content: [{ type: 'text', text: JSON.stringify(result, null, 2) }] };
} catch (error) {
  if (error instanceof ValidationError) {
    return formatError({
      code: ErrorCodes.VALIDATION_ERROR,
      message: error.message,
      suggestion: 'Check the tool schema and provide valid parameters.',
    });
  }
  // ... other error types
}
```

### Integration Point

Modify `server.ts:181-208` CallToolRequestSchema handler.

---

## 2. Zod Input Validation (SECU-01)

### Current State

Tools use plain object schemas:
```typescript
// tools.ts - current
inputSchema: {
  type: 'object',
  properties: {
    query: { type: 'string', description: '...' },
    limit: { type: 'number', description: '...', default: 10 },
  },
  required: ['query'],
}
```

### Recommended Pattern

```typescript
// src/mcp/schemas.ts
import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';

// Define Zod schemas
export const SearchSchema = z.object({
  query: z.string().min(1).describe('Natural language or keyword search query'),
  limit: z.number().int().min(1).max(100).default(10).describe('Max results'),
  groupByProcess: z.boolean().default(true).describe('Group by process'),
});

export const CypherSchema = z.object({
  query: z.string().min(1).describe('Cypher query to execute'),
});

export const ReadSchema = z.object({
  filePath: z.string().min(1).describe('Path to file'),
  startLine: z.number().int().min(1).optional(),
  endLine: z.number().int().min(1).optional(),
});

// Convert to JSON Schema for MCP
export const toolSchemas = {
  search: zodToJsonSchema(SearchSchema),
  cypher: zodToJsonSchema(CypherSchema),
  read: zodToJsonSchema(ReadSchema),
  // ... other tools
};

// Validation helper
export function validateToolInput(toolName: string, input: unknown): z.SafeParseReturnType<unknown, any> {
  const schemaMap: Record<string, z.ZodSchema> = {
    search: SearchSchema,
    cypher: CypherSchema,
    read: ReadSchema,
    // ...
  };
  
  const schema = schemaMap[toolName];
  if (!schema) {
    return { success: false, error: new z.ZodError([]) }; // Should not happen
  }
  
  return schema.safeParse(input);
}
```

### Integration Points

1. `tools.ts` - Update `inputSchema` to use `zodToJsonSchema()` output
2. `server.ts` - Add validation in CallToolRequestSchema handler before calling tool

---

## 3. Cypher Sanitization (SECU-02)

### Risk

Raw Cypher queries can execute destructive operations. Equivalent to SQL injection.

### Recommended Pattern: Whitelist + Blocklist

```typescript
// src/mcp/cypher-sanitizer.ts

const ALLOWED_CLAUSES = ['MATCH', 'RETURN', 'WHERE', 'WITH', 'ORDER', 'BY', 'LIMIT', 'SKIP', 'AS', 'OPTIONAL'];
const FORBIDDEN_KEYWORDS = ['CREATE', 'MERGE', 'DELETE', 'DETACH', 'DROP', 'SET', 'REMOVE', 'CALL'];

export interface SanitizationResult {
  valid: boolean;
  error?: string;
  query?: string;
}

export function sanitizeCypher(query: string): SanitizationResult {
  const upperQuery = query.toUpperCase().trim();
  
  // Check for forbidden keywords
  for (const keyword of FORBIDDEN_KEYWORDS) {
    // Match word boundary to avoid false positives (e.g., "created_at")
    const regex = new RegExp(`\\b${keyword}\\b`);
    if (regex.test(upperQuery)) {
      return {
        valid: false,
        error: `Cypher query contains forbidden keyword: ${keyword}. Only read operations (MATCH, RETURN, etc.) are allowed.`,
      };
    }
  }
  
  // Check that query starts with allowed clause
  const firstWord = upperQuery.split(/\s+/)[0];
  if (!ALLOWED_CLAUSES.includes(firstWord)) {
    return {
      valid: false,
      error: `Cypher query must start with a read clause. Found: ${firstWord}`,
    };
  }
  
  // Additional checks for suspicious patterns
  if (upperQuery.includes('LOAD CSV') || upperQuery.includes('CALL {')) {
    return {
      valid: false,
      error: 'Cypher query contains disallowed operations (LOAD CSV, subqueries).',
    };
  }
  
  return { valid: true, query };
}
```

### Integration Point

In `server.ts` CallToolRequestSchema handler, check if `name === 'cypher'` and validate before forwarding.

---

## 4. Transport Isolation (SECU-03 / CVE-2026-25536)

### The Vulnerability

From MCP security advisory GHSA-345p-7cg4-v4c7:
- Reusing `Server` or `Transport` instances across clients causes cross-client data leakage
- Each client MUST get fresh instances

### Current State

```typescript
// server.ts:104-116 - Server created once per process
export async function startMCPServer(client: ToolCaller): Promise<void> {
  const server = new Server(...);
  // ...
  const transport = new StdioServerTransport();
  await server.connect(transport);
}
```

### Analysis

**Current implementation is SAFE** for stdio transport because:
- stdio is single-client (one AI agent spawns one MCP process)
- Server/transport created fresh per process invocation
- No instance reuse across clients

**However**, if future migration to HTTP transport:
```typescript
// DANGEROUS - reusing server instance
const server = new Server(...);
app.post('/mcp', (req, res) => {
  // BUG: server instance shared across requests
  server.handleRequest(req.body);
});

// SAFE - fresh instance per request
app.post('/mcp', async (req, res) => {
  const server = new Server(...);
  const transport = new StreamableServerTransport();
  await server.connect(transport);
  // ... handle request
});
```

### Recommendation

1. Add documentation comment warning about this
2. If HTTP transport added later, ensure fresh instances per session
3. No code change needed now - stdio is inherently isolated

---

## 5. pino Structured Logging (OBSV-01)

### Recommended Pattern

```typescript
// src/mcp/logger.ts
import pino from 'pino';

export const logger = pino({
  level: process.env.LOG_LEVEL || 'info',
  transport: {
    target: 'pino-pretty',
    options: { colorize: true },
  },
});

// Child logger factory for request context
export function createRequestLogger(requestId: string, toolName?: string) {
  return logger.child({
    requestId,
    tool: toolName,
    agent: process.env.GITNEXUS_AGENT || 'unknown',
  });
}
```

### Integration Points

1. `server.ts` - Log tool calls, errors, request timing
2. `websocket-server.ts` - Log connections, disconnections, context changes

```typescript
// server.ts - tool call logging
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;
  const requestId = `req_${Date.now()}`;
  const log = createRequestLogger(requestId, name);
  
  const startTime = Date.now();
  log.info({ args }, 'Tool call started');
  
  try {
    const result = await client.callTool(name, args);
    log.info({ duration: Date.now() - startTime }, 'Tool call completed');
    return result;
  } catch (error) {
    log.error({ error, duration: Date.now() - startTime }, 'Tool call failed');
    throw error;
  }
});
```

---

## 6. Health Check Resource (OBSV-02)

### MCP Resource Pattern

Add a new resource alongside the existing context resource.

```typescript
// server.ts - add to ListResourcesRequestSchema
resources: [
  // ... existing context resource
  {
    uri: 'gitnexus://codebase/health',
    name: 'GitNexus Health',
    description: 'Connection status and graph availability',
    mimeType: 'application/json',
  },
]

// server.ts - add to ReadResourceRequestSchema
if (uri === 'gitnexus://codebase/health') {
  const health = {
    status: client.context ? 'healthy' : 'no_context',
    timestamp: new Date().toISOString(),
    connection: {
      browser: client.isConnected,
      mode: /* hub or peer */,
    },
    context: client.context ? {
      project: client.context.projectName,
      files: client.context.stats.fileCount,
    } : null,
  };
  
  return {
    contents: [{ uri, mimeType: 'application/json', text: JSON.stringify(health, null, 2) }],
  };
}
```

---

## 7. Graceful Shutdown Enhancement (RELI-03)

### Current State

```typescript
// server.ts:215-225 - Basic shutdown
process.on('SIGINT', async () => {
  client.disconnect?.();
  await server.close();
  process.exit(0);
});
```

### Enhanced Pattern

```typescript
// src/mcp/shutdown.ts
let isShuttingDown = false;

export async function gracefulShutdown(signal: string, client: ToolCaller, server: Server) {
  if (isShuttingDown) return;
  isShuttingDown = true;
  
  console.error(`\nReceived ${signal}. Starting graceful shutdown...`);
  
  // 1. Stop accepting new requests (server.close())
  try {
    await server.close();
    console.error('MCP server closed');
  } catch (e) {
    console.error('Error closing MCP server:', e);
  }
  
  // 2. Wait for pending requests to complete (with timeout)
  // WebSocketBridge tracks pendingRequests internally
  // Give them 5 seconds to complete
  await new Promise(resolve => setTimeout(resolve, 5000));
  
  // 3. Close WebSocket connections
  client.disconnect?.();
  console.error('WebSocket disconnected');
  
  // 4. Flush logs if using pino with async transport
  // await logger.flush(); // if needed
  
  console.error('Graceful shutdown complete');
  process.exit(0);
}

// server.ts - register handlers
process.on('SIGINT', () => gracefulShutdown('SIGINT', client, server));
process.on('SIGTERM', () => gracefulShutdown('SIGTERM', client, server));

// Handle uncaught exceptions
process.on('uncaughtException', (error) => {
  console.error('Uncaught exception:', error);
  gracefulShutdown('uncaughtException', client, server);
});
```

---

## Implementation Order

Based on dependencies and impact:

1. **pino logging** (OBSV-01) - Everything else can log through this
2. **Zod validation** (SECU-01) - Foundational for all tools
3. **Structured errors** (RELI-01) - Depends on validation
4. **Cypher sanitization** (SECU-02) - Depends on validation pattern
5. **Health check resource** (OBSV-02) - Independent, quick win
6. **Graceful shutdown** (RELI-03) - Enhancement of existing code
7. **Transport isolation docs** (SECU-03) - Documentation only

---

## Dependencies to Add

```json
{
  "dependencies": {
    "zod": "^3.23.0",
    "zod-to-json-schema": "^3.23.0",
    "pino": "^9.0.0",
    "pino-pretty": "^11.0.0"
  }
}
```

---

## Files to Create

| File | Purpose |
|------|---------|
| `src/mcp/errors.ts` | Error types and formatting |
| `src/mcp/schemas.ts` | Zod schemas for all tools |
| `src/mcp/cypher-sanitizer.ts` | Cypher query validation |
| `src/mcp/logger.ts` | pino logger setup |
| `src/mcp/shutdown.ts` | Graceful shutdown logic |

---

## Confidence Assessment

| Area | Level | Notes |
|------|-------|-------|
| Error handling | HIGH | MCP SDK docs + existing code pattern |
| Zod validation | HIGH | Standard pattern, zod-to-json-schema well documented |
| Cypher sanitization | MEDIUM | Custom implementation, needs testing |
| Transport isolation | HIGH | CVE analysis confirms current approach is safe |
| pino logging | HIGH | Industry standard, well documented |
| Health check | HIGH | Follows existing resource pattern |
| Graceful shutdown | HIGH | Enhancement of existing working code |

**Overall:** HIGH

---

*Research complete. Ready for planning.*
