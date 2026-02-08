# Phase 2 Plan 2: Resilience Error Codes Summary

**Plan:** 02-02
**Phase:** 02-resilience
**Subsystem:** error-handling
**Status:** Complete
**Duration:** ~6 minutes
**Completed:** 2026-02-08

## One-Liner

Extended error module with resilience-specific error codes (TIMEOUT, CIRCUIT_OPEN, CONNECTION_LOST, RETRY_EXHAUSTED) and helper functions that provide AI agents with structured retry guidance.

## Tasks Completed

| Task | Name | Commit | Files |
| ---- | ---- | ------ | ----- |
| 1 | Add resilience error codes to ErrorCodes | b188e1f | errors.ts |
| 2 | Add resilience error helper functions with retry guidance | 1421220 | errors.ts |

## Deliverables

### Error Codes Added

- `TIMEOUT` - Transient timeout failures
- `CIRCUIT_OPEN` - Circuit breaker protection triggered
- `CONNECTION_LOST` - WebSocket/browser disconnection
- `RETRY_EXHAUSTED` - Maximum retry attempts reached

### GitNexusError Interface Extended

```typescript
interface GitNexusError {
  code: ErrorCode;
  message: string;
  details?: Record<string, unknown>;
  suggestion?: string;
  retryable: boolean;      // NEW: Can agent retry?
  retryAfter?: number;     // NEW: Seconds to wait
}
```

### Helper Functions Created

| Function | retryable | retryAfter | Purpose |
|----------|-----------|------------|---------|
| `timeoutError(tool, timeoutMs)` | true | - | Tool exceeded timeout |
| `circuitOpenError(retryAfterSeconds)` | true | 30 | Circuit breaker open |
| `connectionLostError(reason)` | true | - | WebSocket disconnected |
| `retryExhaustedError(attempts)` | false | - | Max retries reached |

## Key Decisions

| Decision | Rationale |
|----------|-----------|
| `retryable` as required field | All errors must explicitly state if retry is possible |
| `retryAfter` as optional number | Only relevant for time-based backoff (circuit breaker) |
| Debug info behind `GITNEXUS_DEBUG` | Reduces noise in production, helps debugging |
| TIMEOUT vs QUERY_TIMEOUT | Keep both for backward compatibility |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Updated existing error helpers**

- **Found during:** Task 2 (interface update)
- **Issue:** Adding `retryable` as required field broke existing helper functions
- **Fix:** Added `retryable: false` to validationError, toolNotFoundError, cypherForbiddenError; `retryable: true` to internalError
- **Files modified:** errors.ts
- **Commit:** 1421220

**2. [Rule 2 - Missing Critical] Updated server.ts inline errors**

- **Found during:** Build verification
- **Issue:** Inline error objects in server.ts lacked `retryable` field after interface change
- **Fix:** Added `retryable: false` to VALIDATION_ERROR and CYPHER_FORBIDDEN; `retryable: true` to INTERNAL_ERROR
- **Files modified:** server.ts
- **Note:** Changes were applied automatically (possibly by parallel process)

## Tech Stack

### Added
- None (pure TypeScript extension)

### Patterns
- Structured error responses with retry guidance for AI agents
- Debug-mode conditional for verbose error details

## Dependencies

```
requires:
  - 02-01 (circuit breaker uses these error codes)
provides:
  - Structured resilience error responses
  - Retry guidance for AI agents
affects:
  - Future reconnection logic
  - Timeout handling
```

## Next Steps

1. **Plan 02-03**: Connection state machine will use `connectionLostError` and `retryExhaustedError`
2. **Plan 02-04**: Health monitoring will report circuit state using `circuitOpenError`

## Files

### Created
- None

### Modified
- `gitnexus-mcp/src/mcp/errors.ts` - Extended with resilience errors

## Verification

- [x] `npm run build` succeeds without errors
- [x] ErrorCodes contains 4 new resilience codes
- [x] All 4 error helper functions exported
- [x] Each error includes `suggestion` field with actionable guidance
- [x] `retryable` field present on all errors
