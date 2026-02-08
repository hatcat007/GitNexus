---
phase: 02-resilience
plan: 01
subsystem: resilience
tags: [timeout, circuit-breaker, backoff, mcp, resilience]
completed: 2026-02-08
duration: 7 minutes
---

# Phase 2 Plan 1: Core Resilience Module Summary

## One-liner
Core resilience utilities with configurable timeout wrapper (AbortController), opossum-based circuit breaker with consecutive failure tracking, and Full Jitter exponential backoff.

## What Was Built

### Tasks Completed (3/3)

| Task | Name | Commit | Files Modified |
|------|------|--------|----------------|
| 1 | Create timeout wrapper with tool categorization | 554ede8 | resilience.ts, package.json |
| 2 | Add circuit breaker factory | 4c54c54 | resilience.ts, server.ts, package.json |
| 3 | Add exponential backoff calculator | 0e42de3 | resilience.ts |

### Deliverables

**File:** `gitnexus-mcp/src/mcp/resilience.ts` (147 lines)

**Exports:**
- `QUICK_TOOLS` - Array of fast tool names (search, grep, read, context, overview, highlight)
- `HEAVY_TOOLS` - Array of slow tool names (cypher, impact, explore)
- `getTimeout(toolName)` - Returns timeout in ms based on tool category
- `withToolTimeout(toolName, fn)` - Wraps async function with AbortController timeout
- `CircuitBreakerConfig` - Interface for circuit breaker settings
- `createCircuitBreaker(callTool, config)` - Factory creating opossum breaker with consecutive failure tracking
- `BackoffConfig` - Interface for backoff settings
- `calculateBackoff(attempt, config)` - Returns delay in ms with Full Jitter

**Environment Variables:**
- `GITNEXUS_TIMEOUT_QUICK` - Timeout for quick tools (default: 60000ms)
- `GITNEXUS_TIMEOUT_HEAVY` - Timeout for heavy tools (default: 120000ms)

**Dependencies Added:**
- `opossum@^5.0.1` - Circuit breaker library
- `@types/opossum` - TypeScript definitions

## Technical Details

### Timeout Wrapper
- Uses native Node.js `AbortController` (available since v15)
- Passes `AbortSignal` to wrapped function for cooperative cancellation
- Automatically clears timeout on completion/failure

### Circuit Breaker
- Consecutive failure tracking (not percentage-based)
- Opens after 5 consecutive failures
- 30-second cooldown before allowing test request
- Auto-closes immediately on successful test call

### Exponential Backoff
- Formula: `random(0, min(60000, 500 * 2^attempt))`
- Full Jitter chosen over ±10-20% jitter per AWS research
- Caps at 60 seconds maximum delay

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed missing 'retryable' property in server.ts**

- **Found during:** Task 2 (build failed)
- **Issue:** Three inline GitNexusError objects in server.ts were missing required `retryable` property
- **Fix:** Added `retryable: false` to validation and cypher errors, `retryable: true` to internal error
- **Files modified:** gitnexus-mcp/src/mcp/server.ts
- **Commit:** 4c54c54

**2. [Rule 3 - Blocking] Installed missing opossum type definitions**

- **Found during:** Task 2 (TypeScript error)
- **Issue:** `@types/opossum` not installed, causing TS7016 error
- **Fix:** Ran `npm install --save-dev @types/opossum`
- **Files modified:** package.json, package-lock.json
- **Commit:** 4c54c54

## Verification Results

All verification criteria met:
- ✅ `npm run build` succeeds without errors
- ✅ All exports present: QUICK_TOOLS, HEAVY_TOOLS, getTimeout, withToolTimeout, createCircuitBreaker, calculateBackoff
- ✅ No hardcoded timeout values (all use env vars with defaults)
- ✅ opossum in package.json dependencies
- ✅ File size: 147 lines (exceeds 80 line minimum)

## Decisions Made

| Decision | Rationale | Impact |
|----------|-----------|--------|
| Full Jitter over ±10-20% jitter | AWS research shows 50%+ better contention reduction | Simpler code, better performance under load |
| AbortController over AbortSignal.timeout() | Better error handling control, consistent behavior | More explicit timeout management |
| Consecutive failure tracking over percentage | More predictable circuit breaker behavior | Opens after exactly 5 failures |

## Next Phase Readiness

**Ready for:** Plan 02-02 (Resilience error helpers)

**Dependencies satisfied:**
- ✅ Timeout wrapper available for use in error handling
- ✅ Circuit breaker factory ready for server integration
- ✅ Backoff calculator ready for retry logic

**Integration points:**
- `server.ts` will use `withToolTimeout()` to wrap tool calls
- `server.ts` will create circuit breaker instance via `createCircuitBreaker()`
- Retry logic will use `calculateBackoff()` for delays
