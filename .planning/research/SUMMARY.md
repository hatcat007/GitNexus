# Project Research Summary

**Project:** GitNexus MCP Server Enhancement
**Domain:** MCP (Model Context Protocol) Server Implementation (TypeScript/Node.js)
**Researched:** 2026-02-07
**Confidence:** HIGH

## Executive Summary

GitNexus MCP Server is a production-grade MCP server that connects AI agents (Claude, Cursor) to browser-based code graph data via a WebSocket bridge. The server implements the Model Context Protocol, allowing AI assistants to query code knowledge graphs through standardized tools.

The recommended approach is a **layered architecture** with 5 distinct layers: Transport → Security & Validation → Observability → Resilience → Caching → Backend Adapter. This separation of concerns ensures each layer has a single responsibility and enables independent testing, debugging, and scaling. The build order follows a clear dependency chain: start with logging and validation (everything depends on these), add resilience patterns (prevent cascading failures), implement caching (performance), and finally add observability (production visibility).

Key risks center on **security vulnerabilities** (CVE-2026-25536 for transport reuse, CVE-2025-66414 for DNS rebinding) and **reliability gaps** (no circuit breaker, no retry logic, hardcoded timeouts). The MCP ecosystem is new and evolving, but the official TypeScript SDK is production-ready at v1.x and well-documented.

## Key Findings

### Recommended Stack

The MCP Server Implementation uses the official `@modelcontextprotocol/sdk` v1.x for protocol compliance, with supporting libraries chosen for production reliability and Node.js ecosystem compatibility.

**Core technologies:**
- **@modelcontextprotocol/sdk ^1.26.0** — MCP protocol implementation; v1.x is production-ready (v2 is pre-alpha until Q1 2026)
- **zod ^4.0.0** — Schema validation; required peer dependency, used for tool input/output validation
- **TypeScript ^5.7.0** — Type system; ES2020+ target required for SDK AJV imports
- **Node.js >=20.0.0** — Runtime; required for Web Crypto API used by SDK auth extensions

**Supporting libraries:**
- **pino ^9.0.0** — Structured logging; industry standard, JSON output, child loggers for request context
- **opossum ^8.0.0** — Circuit breaker; prevents cascade failures when browser/graph unavailable
- **lru-cache ^11.0.0** — In-memory caching; for frequently-accessed query results with TTL support
- **rate-limiter-flexible ^5.0.0** — Rate limiting; per-agent limits with token bucket algorithm
- **vitest ^2.0.0** — Testing; MCP SDK uses Vitest, fast and ESM-native

### Expected Features

**Must have (table stakes):**
- Structured error responses — AI agents need parseable errors to recover gracefully
- Input validation — JSON Schema validation via Zod; SDK enforces inputSchema
- Configurable timeout — Per-tool timeouts with AbortController cancellation
- Graceful shutdown — SIGINT/SIGTERM handlers to prevent data loss
- Health check resource — Simple MCP resource returning connection status
- Cypher query sanitization — Critical security requirement, whitelist allowed patterns

**Should have (competitive):**
- Exponential backoff retries — For WebSocket reconnects (not tool calls, agents handle those)
- Circuit breaker — Fail fast when browser/graph unavailable
- Structured observability — Request timing, error rates, OpenTelemetry-compatible
- Basic caching — Context and schema caching, not query results (invalidation complexity)
- Rate limiting per agent — Prevent one agent from monopolizing resources

**Defer (v2+):**
- Streaming responses — Requires protocol changes, complex
- Request batching — Requires client cooperation
- Session management — For stateful multi-step operations
- Multi-agent coordination — Full isolation, activity streams

### Architecture Approach

GitNexus uses a **Hub-and-Spoke** architecture where the MCP server acts as a protocol handler, delegating to a WebSocket bridge that connects to the browser-based graph engine. The recommended enhancement adds 5 processing layers between the transport and the backend adapter.

**Major components:**
1. **Transport Layer (L0)** — stdio for AI clients / Streamable HTTP for remote access
2. **Security & Validation Layer (L1)** — Zod validation, rate limiting, error sanitization
3. **Observability Layer (L2)** — pino logging, Prometheus metrics, OpenTelemetry tracing
4. **Resilience Layer (L3)** — Circuit breaker, retry logic, timeout enforcement
5. **Caching Layer (L4)** — LRU cache for read-heavy queries with TTL
6. **Backend Adapter (L5)** — WebSocket bridge (Hub/Peer mode), context synchronization

### Critical Pitfalls

1. **Transport/Server Instance Reuse (CVE-2026-25536)** — Reusing transport instances causes cross-client data leakage. Always create fresh McpServer + transport per request/session.

2. **Token Passthrough Anti-Pattern** — Never accept upstream tokens without validating audience. MCP servers MUST validate tokens were issued for the server itself.

3. **DNS Rebinding Vulnerability (CVE-2025-66414)** — HTTP servers on localhost without protection allow malicious websites to invoke tools. Enable `enableDnsRebindingProtection` or use stdio.

4. **Session Hijacking** — Predictable session IDs allow impersonation. Use cryptographically secure UUIDs, bind to user identity, never use sessions for auth.

5. **No Input Validation / Cypher Injection** — The `cypher` tool with user queries is equivalent to SQL injection. Whitelist safe patterns, reject CREATE/DELETE/DROP.

## Implications for Roadmap

Based on the dependency chain identified in architecture research and the phase mapping from pitfalls research:

### Phase 1: Foundation & Security Hardening
**Rationale:** Everything depends on logging and validation. Critical security pitfalls (CVEs, injection) must be addressed before any other work. This phase establishes the base layer that all subsequent phases build upon.
**Delivers:** Structured logging, input validation, error handling, Cypher sanitization
**Addresses:** Structured error responses, Input validation, Graceful shutdown, Health check, Cypher sanitization (from FEATURES.md P1)
**Avoids:** Transport reuse (CVE-2026-25536), DNS rebinding (CVE-2025-66414), Input validation/Cypher injection

### Phase 2: Resilience Patterns
**Rationale:** With validation in place, add reliability patterns to prevent cascading failures. Circuit breaker and retry logic are essential before adding caching (which could mask failures).
**Delivers:** Configurable timeouts, circuit breaker, exponential backoff retry logic
**Uses:** opossum (circuit breaker), retry (backoff logic)
**Implements:** Resilience Layer (L3) from architecture

### Phase 3: Caching Layer
**Rationale:** With reliable error handling and failure protection, caching can safely improve performance without masking underlying issues. Cache invalidation on context change is straightforward.
**Delivers:** LRU cache for read-heavy queries, TTL management, cache invalidation
**Uses:** lru-cache ^11.0.0
**Implements:** Caching Layer (L4) from architecture

### Phase 4: Rate Limiting & Security Polish
**Rationale:** With core functionality stable, add protection against abuse. Rate limiting requires agent identification which becomes clearer after the system is production-tested.
**Delivers:** Per-agent rate limiting, request size limits, error message sanitization
**Uses:** rate-limiter-flexible ^5.0.0
**Addresses:** Rate limiting (from FEATURES.md P2)

### Phase 5: Observability & Production Readiness
**Rationale:** Final polish for production deployment. Metrics and tracing help diagnose issues that only appear under real load.
**Delivers:** Prometheus metrics endpoint, OpenTelemetry tracing, structured audit trail
**Uses:** prom-client ^15.0.0, @opentelemetry/api ^1.9.0
**Implements:** Observability Layer (L2) from architecture

### Phase Ordering Rationale

- **Phase 1 must be first** — Logging and validation are cross-cutting concerns that all other layers depend on. Security pitfalls are critical and must be addressed immediately.
- **Phase 2 before Phase 3** — Caching can mask failures; resilience patterns must be in place first to ensure cache misses don't cause cascading failures.
- **Phase 4 after Phase 2** — Rate limiting is less urgent than core reliability and can be tuned based on real usage patterns from Phases 1-3.
- **Phase 5 last** — Observability is valuable but the system must be functional first. Metrics without a working system provide no value.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 4:** Rate limiting strategies for multi-agent scenarios — agent identification is challenging without explicit auth; may need `/gsd-research-phase` to explore patterns
- **Phase 5:** OpenTelemetry integration — while well-documented, MCP-specific integration patterns may need research

Phases with standard patterns (skip research-phase):
- **Phase 1:** Well-documented patterns for Zod validation, pino logging, Cypher sanitization
- **Phase 2:** Circuit breaker and retry patterns are standard, opossum documentation is comprehensive
- **Phase 3:** LRU caching with TTL is straightforward, lru-cache API is simple

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Verified against official MCP SDK docs, npm registry, Node.js LTS schedule |
| Features | MEDIUM | MCP ecosystem is new and evolving; feature priorities based on reference implementations |
| Architecture | HIGH | Based on current codebase analysis + MCP SDK best practices + production patterns |
| Pitfalls | HIGH | Verified against official security advisories (GHSA-345p-7cg4-v4c7, GHSA-w48q-cv73-mx4w, GHSA-cqwc-fm46-7fff) |

**Overall confidence:** HIGH

### Gaps to Address

- **Multi-agent coordination patterns:** MCP doesn't define a permission model yet. During Phase 4 planning, research community approaches or implement simple allowlists. May need to wait for spec evolution.
- **Cache invalidation on graph mutation:** Current design assumes read-only access. If graph mutations are added, cache invalidation strategy needs design. Defer to Phase 3 planning.
- **Session management for Streamable HTTP:** If remote deployment is prioritized, session management patterns need research. Currently using stdio transport which doesn't require sessions.

## Sources

### Primary (HIGH confidence)
- MCP Specification 2025-03-26 — transport requirements, protocol version, tools specification
- @modelcontextprotocol/sdk GitHub — v1.26.0 current, v1.x production branch verified
- TypeScript SDK Security Advisories — CVE-2026-25536 (GHSA-345p-7cg4-v4c7), CVE-2025-66414 (GHSA-w48q-cv73-mx4w)
- Node.js LTS schedule — runtime version requirements

### Secondary (MEDIUM confidence)
- MCP Reference Servers (modelcontextprotocol/servers) — feature patterns, error handling approaches
- MCP TypeScript SDK documentation — server.md, client.md patterns
- npm registry — version verification for supporting libraries (pino, opossum, lru-cache)

### Tertiary (context-specific)
- Current GitNexus codebase — server.ts (227 lines), tools.ts (226 lines), websocket-server.ts (398 lines)
- Production MCP server examples — resilience patterns, observability approaches

---
*Research completed: 2026-02-07*
*Ready for roadmap: yes*
