# Requirements: GitNexus MCP Server Enhancement

**Defined:** 2026-02-07
**Core Value:** AI agents can reliably query GitNexus's graph-based code intelligence without failures, timeouts, or incomplete results—every tool call returns decision-ready context.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Reliability

- [ ] **RELI-01**: Tool calls return structured error responses with `isError: true` and actionable context
- [ ] **RELI-02**: Each tool has configurable timeout (via AbortController), not hardcoded 30s
- [ ] **RELI-03**: Server handles SIGINT/SIGTERM gracefully, cleaning up connections before exit
- [ ] **RELI-04**: Circuit breaker prevents cascade failures when browser/graph unavailable
- [ ] **RELI-05**: WebSocket reconnects use exponential backoff with jitter

### Security

- [ ] **SECU-01**: All tool inputs validated against Zod schemas before processing
- [ ] **SECU-02**: Cypher tool sanitizes queries (whitelist allowed patterns, reject DROP/DELETE/CREATE)
- [ ] **SECU-03**: Transport/server instances are isolated per session (CVE-2026-25536 fix)
- [ ] **SECU-04**: Rate limiting per agent prevents resource monopolization

### Performance

- [ ] **PERF-01**: Context and overview responses cached with LRU cache and TTL

### Observability

- [ ] **OBSV-01**: All operations logged with pino structured logging (JSON, child loggers for context)
- [ ] **OBSV-02**: Health check MCP resource returns connection status and graph availability

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Observability (Extended)

- **OBSV-03**: Prometheus metrics endpoint for request duration, error rates, cache hit/miss
- **OBSV-04**: OpenTelemetry tracing for distributed debugging

### Protocol Enhancements

- **PROTO-01**: Streaming responses for large results via SSE notifications
- **PROTO-02**: Request batching to reduce round trips

### Multi-Agent

- **MULTI-01**: Per-agent context isolation
- **MULTI-02**: Activity streams for agent coordination

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| DNS rebinding protection | Using stdio transport, not HTTP localhost server |
| Custom auth system | Delegating to host app / OAuth providers |
| Agent permissions | MCP spec doesn't define permission model yet |
| Cache query results | Invalidation complexity too high for v1 |
| Graph mutation support | Current design assumes read-only access |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| RELI-01 | 1 | Pending |
| RELI-02 | 2 | Pending |
| RELI-03 | 1 | Pending |
| RELI-04 | 2 | Pending |
| RELI-05 | 2 | Pending |
| SECU-01 | 1 | Pending |
| SECU-02 | 1 | Pending |
| SECU-03 | 1 | Pending |
| SECU-04 | 4 | Pending |
| PERF-01 | 3 | Pending |
| OBSV-01 | 1 | Pending |
| OBSV-02 | 1 | Pending |

**Coverage:**
- v1 requirements: 12 total
- Mapped to phases: 12
- Unmapped: 0 ✓

---
*Requirements defined: 2026-02-07*
*Last updated: 2026-02-07 after initial definition*
