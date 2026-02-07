# Roadmap: GitNexus MCP Server Enhancement

## Overview

Transform GitNexus's MCP server from functional to production-grade by adding layered reliability, security, and performance capabilities. The journey builds foundation first (logging, validation, security hardening), adds resilience patterns to prevent cascading failures, introduces caching for read-heavy operations, and finishes with rate limiting to prevent abuse.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Foundation & Security** - Logging, validation, error handling, security hardening
- [ ] **Phase 2: Resilience** - Timeouts, circuit breaker, retry logic
- [ ] **Phase 3: Performance** - LRU caching for read-heavy operations
- [ ] **Phase 4: Protection** - Rate limiting per agent

## Phase Details

### Phase 1: Foundation & Security
**Goal**: MCP server has production-grade observability, input validation, and security hardening
**Depends on**: Nothing (first phase)
**Requirements**: RELI-01, RELI-03, SECU-01, SECU-02, SECU-03, OBSV-01, OBSV-02
**Success Criteria** (what must be TRUE):
  1. AI agent receives structured JSON errors with actionable context when tool calls fail
  2. All tool inputs are validated against Zod schemas before processing
  3. Cypher queries are sanitized (whitelist patterns, reject destructive operations)
  4. Transport/server instances are isolated per session (no cross-client leakage)
  5. Server logs all operations with structured JSON output (pino)
  6. Health check MCP resource returns connection status and graph availability
**Plans**: 2 plans

Plans:
- [ ] 01-01-PLAN.md — Create foundation modules (logger, errors, schemas, cypher-sanitizer)
- [ ] 01-02-PLAN.md — Integrate into server, add health check, enhance shutdown

### Phase 2: Resilience
**Goal**: Server handles failures gracefully without cascading or hanging
**Depends on**: Phase 1
**Requirements**: RELI-02, RELI-04, RELI-05
**Success Criteria** (what must be TRUE):
  1. Each tool call has configurable timeout (via AbortController), no hardcoded 30s
  2. Circuit breaker prevents cascade failures when browser/graph unavailable
  3. WebSocket reconnects use exponential backoff with jitter
**Plans**: TBD (during planning)

Plans:
- [ ] 02-01: (TBD during planning)

### Phase 3: Performance
**Goal**: Frequently-accessed data is cached to reduce latency and load
**Depends on**: Phase 2
**Requirements**: PERF-01
**Success Criteria** (what must be TRUE):
  1. Context and overview responses are cached with LRU cache and TTL
**Plans**: TBD (during planning)

Plans:
- [ ] 03-01: (TBD during planning)

### Phase 4: Protection
**Goal**: Server is protected against resource monopolization
**Depends on**: Phase 3
**Requirements**: SECU-04
**Success Criteria** (what must be TRUE):
  1. Rate limiting per agent prevents one client from monopolizing resources
**Plans**: TBD (during planning)

Plans:
- [ ] 04-01: (TBD during planning)

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation & Security | 0/2 | Ready | - |
| 2. Resilience | 0/TBD | Not started | - |
| 3. Performance | 0/TBD | Not started | - |
| 4. Protection | 0/TBD | Not started | - |
