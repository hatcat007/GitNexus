# Pitfalls Research

**Domain:** MCP (Model Context Protocol) Server Implementation
**Researched:** 2026-02-07
**Confidence:** HIGH (verified against official MCP documentation and security advisories)

---

## Critical Pitfalls

### Pitfall 1: Transport/Server Instance Reuse (CVE-2026-25536)

**What goes wrong:**
Reusing a single `StreamableHTTPServerTransport` or `McpServer` instance across multiple client requests causes cross-client data leakage. JSON-RPC message ID collisions route responses to the wrong HTTP stream.

**Why it happens:**
SDK client generates incrementing message IDs starting at 0. When two clients share a transport, their IDs collide. The second client's mapping overwrites the first, routing responses incorrectly.

**How to avoid:**
- **Stateless mode**: Create fresh `McpServer` + transport per request
- **Stateful mode**: Create fresh `McpServer` + transport per session

```typescript
// WRONG - shared instances leak data
const server = new McpServer({...});
const transport = new StreamableHTTPServerTransport({...});
app.post('/mcp', async (req, res) => {
  await server.connect(transport); // DANGEROUS
  await transport.handleRequest(req, res);
});

// CORRECT - fresh instances per request
app.post('/mcp', async (req, res) => {
  const server = new McpServer({...});
  const transport = new StreamableHTTPServerTransport({ sessionIdGenerator: undefined });
  await server.connect(transport);
  await transport.handleRequest(req, res);
});
```

**Warning signs:**
- Responses appearing in wrong client sessions
- Intermittent "unknown request ID" errors
- Progress notifications delivered to wrong client

**Phase to address:** Phase 1 (Core MCP Infrastructure)

---

### Pitfall 2: Token Passthrough Anti-Pattern

**What goes wrong:**
Accepting tokens from MCP clients without validating they were issued *to the MCP server*, then passing them through to downstream APIs. This circumvents security controls, breaks audit trails, and creates trust boundary issues.

**Why it happens:**
Developers treat MCP server as a "pure proxy" without understanding the security implications of token audience validation.

**Consequences:**
- Rate limiting, request validation bypass
- Downstream logs show wrong source identity
- Stolen tokens can be used for data exfiltration

**How to avoid:**
MCP servers MUST validate that tokens were explicitly issued for the MCP server itself. Never accept upstream tokens directly.

**Warning signs:**
- Code passes `Authorization` header from request directly to fetch
- No token audience validation
- Single token used for multiple services

**Phase to address:** Phase 2 (Authentication & Authorization)

---

### Pitfall 3: DNS Rebinding Vulnerability (CVE-2025-66414)

**What goes wrong:**
HTTP-based MCP servers on localhost without DNS rebinding protection allow malicious websites to bypass same-origin policy and invoke tools on behalf of the user.

**Why it happens:**
DNS rebinding protection was disabled by default in SDK versions < 1.24.0. Developers running unauthenticated servers on localhost are vulnerable.

**How to avoid:**
- Enable `enableDnsRebindingProtection` option
- Use `hostHeaderValidation()` middleware for custom Express configs
- Prefer stdio transport for local development
- Require authentication for HTTP-based servers

**Warning signs:**
- Server binds to localhost without authentication
- No Host header validation
- SDK version < 1.24.0

**Phase to address:** Phase 1 (Core MCP Infrastructure)

---

### Pitfall 4: Session Hijacking

**What goes wrong:**
Predictable session IDs or using sessions for authentication allows attackers to impersonate legitimate users or inject malicious payloads.

**Attack vectors:**
1. **Prompt injection**: Attacker sends malicious event with guessed session ID
2. **Impersonation**: Attacker uses stolen session ID to make unauthorized calls

**How to avoid:**
- Use cryptographically secure random session IDs (UUIDs with secure RNG)
- Bind session IDs to user-specific information (`<user_id>:<session_id>`)
- MCP servers MUST verify all inbound requests (never use sessions for auth)
- Rotate/expire session IDs

**Warning signs:**
- Sequential or predictable session IDs
- Session ID used as sole authentication mechanism
- No user identity binding

**Phase to address:** Phase 2 (Authentication & Authorization)

---

### Pitfall 5: No Input Validation / Cypher Injection

**What goes wrong:**
Unvalidated tool inputs allow injection attacks. Specifically, the `cypher` tool with user-provided queries is equivalent to SQL injection risk.

**Why it happens:**
MCP specification requires servers to validate inputs, but developers often skip this step.

**How to avoid:**
- Validate ALL tool inputs against inputSchema
- For Cypher queries: whitelist allowed patterns, reject dangerous operations (CREATE, DELETE, DROP)
- Sanitize file paths in `read` tool (prevent directory traversal)
- Use parameterized queries where possible

```typescript
// WRONG - direct user input
const result = await client.callTool(name, args);

// CORRECT - validated input
const schema = GITNEXUS_TOOLS.find(t => t.name === name)?.inputSchema;
if (schema) {
  validateInput(args, schema); // Throws on invalid
}
const result = await client.callTool(name, args);
```

**Warning signs:**
- Tool handlers accept raw `arguments` without validation
- Cypher queries constructed via string concatenation
- File paths from user input used directly

**Phase to address:** Phase 1 (Core MCP Infrastructure)

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Skip input validation | Faster tool development | Injection vulnerabilities, crashes | Never |
| Reuse transport instances | Simpler code, less memory | Cross-client data leakage (CVE-2026-25536) | Never |
| No rate limiting | Easier implementation | DoS vulnerability, resource exhaustion | Single-user local only |
| Basic 30s timeout | Quick to implement | No resilience, cascading failures | Prototype only |
| No circuit breaker | Simpler error handling | Repeated failures cascade, system instability | Never for production |
| Pass-through errors | Less code | Leaks internal details to clients | Never |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Claude Desktop | Relative paths in `claude_desktop_config.json` | Always use absolute paths |
| Claude Desktop | Logging to stdout (breaks protocol) | Log to stderr only |
| WebSocket bridge | No reconnection logic | Implement exponential backoff with max retries |
| Neo4j/Cypher | Allowing arbitrary Cypher | Whitelist safe patterns, reject mutations |
| File system | No path sanitization | Validate paths stay within allowed directories |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| No caching | Repeated identical queries, slow responses | Cache context, hotspots, frequent queries | 10+ concurrent agents |
| Single-threaded handlers | Blocked requests during slow operations | Use worker threads or async queues | Any slow tool call |
| Large context payloads | Memory pressure, slow serialization | Lazy loading, pagination, streaming | 50K+ file codebases |
| No request timeouts | Hung connections | Implement per-tool timeouts with abort signals | Network issues |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| No authentication on localhost HTTP server | DNS rebinding attack (CVE-2025-66414) | Enable DNS rebinding protection or use stdio |
| Session ID used for auth | Session hijacking, impersonation | Always verify requests independently |
| Token passthrough | Security bypass, audit trail loss | Validate token audience for MCP server |
| Unvalidated tool annotations | Clients trust untrusted metadata | Treat tool annotations as untrusted |
| Secrets in tool responses | Credential exposure | Sanitize outputs, mask sensitive data |
| No rate limiting per agent | DoS, resource exhaustion | Implement per-client rate limits |

---

## Testing Blind Spots

| Blind Spot | What Gets Missed | How to Test |
|------------|------------------|-------------|
| Concurrent client handling | Cross-client data leakage | Simulate multiple clients with overlapping message IDs |
| Transport reconnection | State loss, infinite loops | Kill transport mid-session, verify recovery |
| Large result sets | Memory issues, timeouts | Test with 100K+ row results |
| Malformed inputs | Unhandled exceptions | Fuzz test all tool inputs |
| Session expiration | Stale sessions persist | Test session timeout paths |
| Authentication failures | Improper error handling | Test all auth failure modes |

---

## "Looks Done But Isn't" Checklist

- [ ] **Transport lifecycle**: Often missing proper cleanup — verify `close()` called on disconnect
- [ ] **Error handling**: Often catches but doesn't report — verify `isError: true` in tool results
- [ ] **Input validation**: Often skipped for "internal" tools — validate ALL tools
- [ ] **Timeout handling**: Often just set globally — verify per-tool timeout with abort signal
- [ ] **Session cleanup**: Often leaves orphaned sessions — verify cleanup on client disconnect
- [ ] **Graceful shutdown**: Often just `process.exit()` — verify pending requests complete

---

## Developer Experience Mistakes

| Mistake | User Impact | Better Approach |
|---------|-------------|-----------------|
| Vague tool descriptions | AI agents misuse tools | Include examples, when-to-use, return format |
| Missing error context | "Unknown error" leaves agents stuck | Include actionable error messages |
| Undocumented schema | AI agents send wrong types | Provide complete JSON Schema with descriptions |
| No usage examples | Trial-and-error integration | Include example requests/responses in tool descriptions |
| Silent failures | Agents assume success | Always return explicit success/error status |

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Transport reuse | MEDIUM | Refactor to create fresh instances per request/session |
| Token passthrough | HIGH | Implement proper auth flow, migrate all clients |
| DNS rebinding exposure | HIGH | Upgrade SDK, add Host validation middleware |
| Session hijacking | HIGH | Regenerate all sessions, add user binding |
| Cypher injection | HIGH | Audit all queries, add validation layer |

---

## Verification of AI-Generated Claims

The following claims from the project context were **VERIFIED**:

| Claim | Status | Evidence |
|-------|--------|----------|
| No retry logic for failed tool calls | **CONFIRMED** | Current `mcp-client.ts` has no retry mechanism |
| Basic timeout (30s) without exponential backoff | **CONFIRMED** | No timeout handling visible in current implementation |
| No circuit breaker for repeated failures | **CONFIRMED** | No circuit breaker pattern implemented |
| No caching of frequently accessed data | **CONFIRMED** | Context sent fresh each time, no caching |
| No input sanitization for Cypher queries | **CONFIRMED** | `cypher` tool passes query directly without validation |
| No rate limiting per agent | **CONFIRMED** | No rate limiting visible |
| No authentication between Hub/Peer | **CONFIRMED** | WebSocket bridge has no auth mechanism |
| Unvalidated file paths in read tool | **CONFIRMED** | `read` tool takes arbitrary `filePath` parameter |
| No unit tests for MCP server | **CONFIRMED** | No test files found in gitnexus-mcp |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Transport/Server reuse | Phase 1 | Test with concurrent clients |
| Token passthrough | Phase 2 | Security audit of auth flow |
| DNS rebinding | Phase 1 | Test from malicious origin |
| Session hijacking | Phase 2 | Penetration testing |
| Input validation | Phase 1 | Fuzz testing all tools |
| Rate limiting | Phase 2 | Load testing |
| Error handling | Phase 1 | Chaos testing |
| Caching | Phase 3 | Performance benchmarks |

---

## Sources

**HIGH Confidence (Official):**
- [MCP Specification - Tools](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)
- [MCP Security Best Practices](https://modelcontextprotocol.io/specification/2025-06-18/basic/security_best_practices)
- [MCP Debugging Guide](https://modelcontextprotocol.io/docs/tools/debugging)
- [TypeScript SDK Security Advisory GHSA-345p-7cg4-v4c7](https://github.com/modelcontextprotocol/typescript-sdk/security/advisories/GHSA-345p-7cg4-v4c7)
- [TypeScript SDK Security Advisory GHSA-w48q-cv73-mx4w](https://github.com/modelcontextprotocol/typescript-sdk/security/advisories/GHSA-w48q-cv73-mx4w)
- [TypeScript SDK Security Advisory GHSA-cqwc-fm46-7fff](https://github.com/modelcontextprotocol/typescript-sdk/security/advisories/GHSA-cqwc-fm46-7fff)

**MEDIUM Confidence (Official Examples):**
- [TypeScript SDK Repository](https://github.com/modelcontextprotocol/typescript-sdk)
- [MCP Reference Servers](https://github.com/modelcontextprotocol/servers)

---
*Pitfalls research for: MCP Server Implementation*
*Researched: 2026-02-07*
