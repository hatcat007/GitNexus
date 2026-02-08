# Phase 2: Resilience - Context

**Gathered:** 2026-02-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Add resilience patterns to the MCP server so it handles failures gracefully without hanging or cascading. Covers configurable timeouts per tool, circuit breaker for dependency failures, and exponential backoff for WebSocket reconnection. Rate limiting and caching are separate phases.

</domain>

<decisions>
## Implementation Decisions

### Timeout Configuration

- **Two-tier timeout system**: Quick tools (60s default) vs Heavy tools (120s default)
- **Quick tools**: search, grep, read, context, overview, highlight
- **Heavy tools**: cypher, impact, explore
- **Environment variables with fallbacks**:
  - `GITNEXUS_TIMEOUT_QUICK` — defaults to 60 seconds
  - `GITNEXUS_TIMEOUT_HEAVY` — defaults to 120 seconds
- Implementation via AbortController, no hardcoded values

### Circuit Breaker Behavior

- **Failure threshold**: 5 consecutive failures before circuit opens
- **Cooldown period**: 30 seconds in open state before allowing test request
- **Recovery**: Close immediately on successful test request (Claude's discretion)
- **Open circuit error response**: "Circuit breaker open due to repeated failures. Will retry in X seconds. Consider checking browser connection or graph status."

### Reconnection Strategy

- **Initial delay**: 500ms before first reconnection attempt (Claude's discretion)
- **Maximum delay cap**: 60 seconds
- **Backoff multiplier**: 2x exponential (Claude's discretion)
  - Progression: 500ms → 1s → 2s → 4s → 8s → 16s → 32s → 60s (capped)
- **Jitter**: Small (±10-20%) to break pathological patterns without complicating debugging (Claude's discretion)

### Failure Error Responses

- **Distinct error codes** for each failure mode:
  - `TIMEOUT` — Tool call exceeded timeout
  - `CIRCUIT_OPEN` — Circuit breaker blocking requests
  - `CONNECTION_LOST` — WebSocket/browser disconnected
  - `RETRY_EXHAUSTED` — Max reconnection attempts reached
- **Always include retry guidance**:
  - `retryable: boolean` — Can the agent retry?
  - `retryAfter?: number` — How long to wait (seconds) before retrying
- **Debug info is configurable**:
  - Debug mode includes: tool name, arguments (sanitized), timeout used, attempt number
  - Production mode: minimal details (tool name + failure reason)
- **Debug mode control**:
  - Environment variable: `GITNEXUS_DEBUG=true` enables verbose responses
  - Per-request capability: Agent can request debug info on specific calls

### Claude's Discretion

- **Circuit breaker recovery**: Close immediately on one successful test (vs. gradual)
- **Reconnection initial delay**: 500ms (responsive without being aggressive)
- **Backoff multiplier**: 2x (standard exponential, industry norm)
- **Jitter amount**: ±10-20% (prevents sync'd retries, keeps timing predictable)

</decisions>

<specifics>
## Specific Ideas

- Standard resilience patterns — no exotic requirements
- Error messages should help the AI agent understand what happened and what to do
- Debug mode useful for troubleshooting without cluttering normal responses

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 02-resilience*
*Context gathered: 2026-02-08*
