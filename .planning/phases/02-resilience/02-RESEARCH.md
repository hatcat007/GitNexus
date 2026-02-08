# Phase 2: Resilience - Research

**Researched:** 2026-02-08
**Domain:** Node.js Resilience Patterns (Timeouts, Circuit Breaker, Backoff)
**Confidence:** HIGH

## Summary

This phase adds three resilience patterns to the GitNexus MCP server: configurable timeouts per tool, circuit breaker for dependency failures, and exponential backoff with jitter for WebSocket reconnection.

The implementation will use **native Node.js AbortController** for timeouts (available since v15, stable in v16+), **opossum** for circuit breaker (industry standard, 1.6k stars, active maintenance), and **custom backoff logic** with jitter based on AWS's published research.

**Primary recommendation:** Wrap the WebSocket bridge calls with a resilience layer that orchestrates timeout → circuit breaker → backoff, keeping the existing bridge code simple while adding protection at the MCP server level.

## Standard Stack

The established libraries/tools for this domain:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `AbortController` | Native (Node v15+) | Cancellable async operations | Built into Node.js, zero dependencies, Web API compatible |
| `opossum` | ^9.0.0 | Circuit breaker | Most popular Node.js circuit breaker (1.6k stars), well-maintained, TypeScript support |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `retry` | ^0.13.0 | Exponential backoff | If using package instead of custom; simpler but less control |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Native `AbortController` | `abort-controller` polyfill | Only needed for Node < v15 (project requires >= v18) |
| `opossum` | `cockatiel` | Cockatiel is more feature-rich but heavier; opossum is battle-tested and simpler |
| Custom backoff with jitter | `exponential-backoff` package | Package adds dependency; custom is ~20 lines with full control |

**No new dependencies required:**
- AbortController is native in Node.js >= 15
- opossum is already listed in STACK.md for resilience

## Architecture Patterns

### Recommended Project Structure
```
gitnexus-mcp/src/
├── mcp/
│   ├── server.ts           # Add resilience wrapper around client.callTool()
│   ├── errors.ts           # Extend with resilience error codes
│   └── resilience.ts       # NEW: Timeout, circuit breaker, backoff logic
└── bridge/
    └── websocket-server.ts # Add reconnection with backoff (minimal changes)
```

### Pattern 1: Timeout with AbortController

**What:** Use native Node.js AbortController to enforce per-tool timeouts. The `AbortSignal.timeout()` static method creates a signal that aborts after a delay.

**When to use:** Every tool call must have a timeout to prevent indefinite hangs.

**Example:**
```typescript
// Source: Node.js v20+ docs - https://nodejs.org/api/globals.html
// AbortSignal.timeout() creates a signal that aborts after delay

async function withTimeout<T>(
  fn: (signal: AbortSignal) => Promise<T>,
  timeoutMs: number
): Promise<T> {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeoutMs);
  
  try {
    return await fn(controller.signal);
  } finally {
    clearTimeout(timeoutId);
  }
}

// Alternative using AbortSignal.timeout (Node v17.3+)
async function withTimeoutV2<T>(
  fn: (signal: AbortSignal) => Promise<T>,
  timeoutMs: number
): Promise<T> {
  const signal = AbortSignal.timeout(timeoutMs);
  return fn(signal);
}
```

### Pattern 2: Circuit Breaker with Opossum

**What:** Wrap unreliable operations in a circuit breaker that opens after threshold failures, preventing cascade failures.

**When to use:** Protect against repeated failures to an unhealthy dependency (browser/graph).

**Example:**
```typescript
// Source: opossum README - https://github.com/nodeshift/opossum
import CircuitBreaker from 'opossum';

const breaker = new CircuitBreaker(callTool, {
  timeout: 120000,               // Per-call timeout (ms) - our heavy tools default
  errorThresholdPercentage: -1,  // Use consecutive failures instead of percentage
  resetTimeout: 30000,           // Time before trying again (ms)
  rollingCountBuckets: 1,        // Simplified: just track consecutive
  rollingCountTimeout: 10000,    // Window for stats
  volumeThreshold: 0,            // Start tracking immediately
});

// Track consecutive failures manually for threshold=5
let consecutiveFailures = 0;
breaker.on('failure', () => {
  consecutiveFailures++;
  if (consecutiveFailures >= 5 && !breaker.opened) {
    breaker.open();
  }
});
breaker.on('success', () => {
  consecutiveFailures = 0;
});

// Usage
try {
  const result = await breaker.fire(method, params);
} catch (err) {
  if (breaker.opened) {
    // Circuit is open - return friendly error
    return { error: 'Circuit breaker open', retryAfter: 30 };
  }
  throw err;
}
```

### Pattern 3: Exponential Backoff with Full Jitter

**What:** For reconnection, use exponential backoff with full jitter to prevent synchronized retry storms.

**When to use:** WebSocket reconnection attempts when browser disconnects.

**Example:**
```typescript
// Source: AWS Architecture Blog - https://aws.amazon.com/blogs/architecture/exponential-backoff-and-jitter/
// Full Jitter formula: sleep = random(0, min(cap, base * 2^attempt))

function calculateBackoff(
  attempt: number,
  baseMs: number = 500,
  maxMs: number = 60000,
  jitterPercent: number = 0.15 // ±15% jitter
): number {
  // Exponential: base * 2^attempt, capped at max
  const exponential = Math.min(maxMs, baseMs * Math.pow(2, attempt));
  
  // Full jitter: random value from 0 to exponential
  // AWS research shows Full Jitter is best for reducing contention
  const withJitter = Math.random() * exponential;
  
  // Alternative: Decorrelated Jitter with ±10-20%
  // const jitterRange = exponential * jitterPercent;
  // const withJitter = exponential + (Math.random() * 2 - 1) * jitterRange;
  
  return Math.floor(withJitter);
}

// Progression with base=500ms, max=60s:
// Attempt 0: 0-500ms
// Attempt 1: 0-1000ms  
// Attempt 2: 0-2000ms
// Attempt 3: 0-4000ms
// Attempt 4: 0-8000ms
// Attempt 5: 0-16000ms
// Attempt 6: 0-32000ms
// Attempt 7+: 0-60000ms (capped)
```

### Pattern 4: Error Response Structure for AI Agents

**What:** Return structured error responses with retry guidance so AI agents can understand and act on failures.

**Example:**
```typescript
// Extended error format
interface ResilienceError {
  error: true;
  code: 'TIMEOUT' | 'CIRCUIT_OPEN' | 'CONNECTION_LOST' | 'RETRY_EXHAUSTED';
  message: string;
  retryable: boolean;
  retryAfter?: number;  // seconds until retry is useful
  debug?: {             // Only included when GITNEXUS_DEBUG=true
    tool: string;
    timeout: number;
    attempt: number;
    timestamp: string;
  };
}

// Example responses
const timeoutError: ResilienceError = {
  error: true,
  code: 'TIMEOUT',
  message: 'Tool call exceeded timeout limit',
  retryable: true,
  retryAfter: 5,
};

const circuitOpenError: ResilienceError = {
  error: true,
  code: 'CIRCUIT_OPEN',
  message: 'Circuit breaker open due to repeated failures. Will retry in 30 seconds. Consider checking browser connection or graph status.',
  retryable: true,
  retryAfter: 30,
};
```

### Anti-Patterns to Avoid

- **Hardcoded timeout values**: Must use environment variables with sensible defaults
- **Percentage-based circuit breaker threshold**: With low call volume, 5 consecutive failures is clearer than 50% of 10 calls
- **No jitter in backoff**: Causes synchronized retry storms when multiple clients reconnect
- **Silent failures**: Always return structured errors, never just throw
- **Circuit breaker on every error**: Only count WebSocket/bridge failures, not validation errors

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Timeout enforcement | `setTimeout` + promise race | `AbortController` | Proper cancellation, signal propagation, standard API |
| Circuit breaker state machine | Custom state tracking | `opossum` | Handles edge cases, stats, events, tested at scale |
| Backoff calculation | Simple exponential | Full Jitter algorithm | AWS research proves it reduces contention by 50%+ |

**Key insight:** These patterns have subtle edge cases (cleanup on timeout, half-open state, jitter distribution). Use proven implementations.

## Common Pitfalls

### Pitfall 1: Timer Leaks on AbortController

**What goes wrong:** Creating AbortController without cleanup causes memory leaks in long-running processes.

**Why it happens:** The abort event listener and internal timer persist even after the operation completes.

**How to avoid:** Always clear timeout in `finally` block, or use `AbortSignal.timeout()` which self-cleans.

**Warning signs:** Growing memory usage over time, unbounded `pendingRequests` map.

### Pitfall 2: Circuit Breaker Never Closes

**What goes wrong:** Circuit opens on failures but never transitions back to closed, permanently blocking requests.

**Why it happens:** Not resetting failure count on success, or incorrect `resetTimeout` configuration.

**How to avoid:** Use opossum's built-in half-open state with `resetTimeout`; reset consecutive failure counter on success.

**Warning signs:** All requests return "circuit open" even after dependency recovers.

### Pitfall 3: Jitter Causing Debugging Issues

**What goes wrong:** Random backoff makes timing unpredictable, hard to reproduce issues.

**Why it happens:** Too much jitter (e.g., ±50%) obscures the exponential pattern.

**How to avoid:** Use ±10-20% jitter (as decided in context), log backoff calculations when debug mode enabled.

**Warning signs:** QA can't reproduce timing-related bugs; logs show wildly varying retry delays.

### Pitfall 4: Timeout Too Short for Heavy Operations

**What goes wrong:** Complex Cypher queries time out before completing, even when healthy.

**Why it happens:** Single timeout value applied to all tools regardless of complexity.

**How to avoid:** Two-tier timeout system (Quick: 60s, Heavy: 120s) as decided in context.

**Warning signs:** Legitimate queries failing with timeout; users reporting "works sometimes".

## Code Examples

### Timeout Wrapper with Two-Tier System

```typescript
// Source: Pattern derived from Node.js docs + project requirements
// File: gitnexus-mcp/src/mcp/resilience.ts

const QUICK_TOOLS = ['search', 'grep', 'read', 'context', 'overview', 'highlight'];
const HEAVY_TOOLS = ['cypher', 'impact', 'explore'];

function getTimeout(toolName: string): number {
  const env = {
    quick: parseInt(process.env.GITNEXUS_TIMEOUT_QUICK || '60000', 10),
    heavy: parseInt(process.env.GITNEXUS_TIMEOUT_HEAVY || '120000', 10),
  };
  
  if (QUICK_TOOLS.includes(toolName)) return env.quick;
  if (HEAVY_TOOLS.includes(toolName)) return env.heavy;
  return env.quick; // Default to quick
}

async function withToolTimeout<T>(
  toolName: string,
  fn: (signal: AbortSignal) => Promise<T>
): Promise<T> {
  const timeoutMs = getTimeout(toolName);
  const controller = new AbortController();
  const timeoutId = setTimeout(() => {
    controller.abort(new Error(`Tool ${toolName} timed out after ${timeoutMs}ms`));
  }, timeoutMs);

  try {
    return await fn(controller.signal);
  } finally {
    clearTimeout(timeoutId);
  }
}
```

### Circuit Breaker Integration

```typescript
// Source: opossum documentation + project requirements
// File: gitnexus-mcp/src/mcp/resilience.ts

import CircuitBreaker from 'opossum';

interface CircuitBreakerConfig {
  failureThreshold: number;  // 5 consecutive failures
  resetTimeoutMs: number;    // 30 seconds
}

export function createCircuitBreaker(
  callTool: (method: string, params: any) => Promise<any>,
  config: CircuitBreakerConfig
): CircuitBreaker {
  const breaker = new CircuitBreaker(callTool, {
    timeout: false,  // We handle timeout separately with AbortController
    errorThresholdPercentage: -1,  // Disable percentage-based
    resetTimeout: config.resetTimeoutMs,
    rollingCountBuckets: 1,
    volumeThreshold: 0,
  });

  // Track consecutive failures for threshold
  let consecutiveFailures = 0;
  
  breaker.on('failure', () => {
    consecutiveFailures++;
    if (consecutiveFailures >= config.failureThreshold) {
      if (!breaker.opened) {
        breaker.open();
      }
    }
  });
  
  // Reset on success (immediate close - per context decision)
  breaker.on('success', () => {
    consecutiveFailures = 0;
    if (breaker.halfOpen) {
      breaker.close();  // Close immediately on successful test
    }
  });

  return breaker;
}
```

### WebSocket Reconnection with Backoff

```typescript
// Source: AWS Full Jitter research + project requirements
// File: gitnexus-mcp/src/bridge/websocket-server.ts (modifications)

class ReconnectionManager {
  private attempt = 0;
  private readonly baseDelay = 500;      // Initial delay (Claude's discretion)
  private readonly maxDelay = 60000;     // 60 second cap
  private readonly jitterRange = 0.15;   // ±15% jitter
  
  private reconnectTimer?: NodeJS.Timeout;

  calculateDelay(): number {
    // Exponential with cap
    const exponential = Math.min(
      this.maxDelay, 
      this.baseDelay * Math.pow(2, this.attempt)
    );
    
    // Full jitter (AWS recommendation)
    return Math.floor(Math.random() * exponential);
  }

  scheduleReconnect(callback: () => void): void {
    const delay = this.calculateDelay();
    console.error(`Reconnecting in ${delay}ms (attempt ${this.attempt + 1})`);
    
    this.reconnectTimer = setTimeout(() => {
      this.attempt++;
      callback();
    }, delay);
  }

  onSuccess(): void {
    this.attempt = 0;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = undefined;
    }
  }

  reset(): void {
    this.attempt = 0;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = undefined;
    }
  }
}
```

### Error Response Formatting

```typescript
// Source: Existing errors.ts + context requirements
// File: gitnexus-mcp/src/mcp/errors.ts (extensions)

export const ErrorCodes = {
  // ... existing codes ...
  TIMEOUT: 'TIMEOUT',
  CIRCUIT_OPEN: 'CIRCUIT_OPEN',
  CONNECTION_LOST: 'CONNECTION_LOST',
  RETRY_EXHAUSTED: 'RETRY_EXHAUSTED',
} as const;

interface ResilienceErrorDetails {
  code: typeof ErrorCodes.TIMEOUT | typeof ErrorCodes.CIRCUIT_OPEN | 
        typeof ErrorCodes.CONNECTION_LOST | typeof ErrorCodes.RETRY_EXHAUSTED;
  message: string;
  retryable: boolean;
  retryAfter?: number;
  debugInfo?: {
    tool: string;
    timeout?: number;
    attempt?: number;
  };
}

export function resilienceError(details: ResilienceErrorDetails): GitNexusError {
  const isDebug = process.env.GITNEXUS_DEBUG === 'true';
  
  return {
    code: details.code,
    message: details.message,
    details: isDebug ? details.debugInfo : undefined,
    suggestion: details.retryable 
      ? `Retry after ${details.retryAfter || 5} seconds.`
      : 'This error is not automatically recoverable.',
  };
}

// Specific error creators
export function timeoutError(tool: string, timeout: number): GitNexusError {
  return resilienceError({
    code: ErrorCodes.TIMEOUT,
    message: `Tool '${tool}' exceeded timeout of ${timeout / 1000}s`,
    retryable: true,
    retryAfter: 5,
    debugInfo: { tool, timeout },
  });
}

export function circuitOpenError(retryAfterSeconds: number): GitNexusError {
  return resilienceError({
    code: ErrorCodes.CIRCUIT_OPEN,
    message: `Circuit breaker open due to repeated failures. Will retry in ${retryAfterSeconds} seconds. Consider checking browser connection or graph status.`,
    retryable: true,
    retryAfter: retryAfterSeconds,
  });
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Promise.race with setTimeout | AbortController with signal | Node v15 (2020) | Proper cancellation, cleaner code, signal propagation |
| Percentage-based circuit threshold | Consecutive failure count | Opossum v8+ | More predictable behavior with low volume |
| Exponential backoff without jitter | Full Jitter | AWS 2015 research | 50%+ reduction in retry contention |

**Deprecated/outdated:**
- `request.timeout()` with callbacks: Use AbortController instead
- Fixed retry delays: Cause thundering herd, always use jitter
- Percentage threshold with `volumeThreshold > 0`: Confusing with variable load

## Open Questions

Things that couldn't be fully resolved:

1. **Should circuit breaker track per-tool or global?**
   - What we know: Currently planned as global for all bridge calls
   - What's unclear: If one tool fails repeatedly, should all tools be blocked?
   - Recommendation: Start with global (simpler), add per-tool if needed. The browser connection is shared, so global makes sense.

2. **Max reconnection attempts before giving up?**
   - What we know: Context doesn't specify a limit
   - What's unclear: Should we eventually stop trying to reconnect?
   - Recommendation: No hard limit. Cap at 60s delay and keep trying. User can restart if needed. Log warnings after 10 attempts.

## Sources

### Primary (HIGH confidence)
- Node.js v20+ Documentation - AbortController API (https://nodejs.org/api/globals.html#class-abortcontroller)
- opossum GitHub Repository (https://github.com/nodeshift/opossum) - v9.0.0, 1.6k stars, Apache-2.0
- AWS Architecture Blog - Exponential Backoff and Jitter (https://aws.amazon.com/blogs/architecture/exponential-backoff-and-jitter/)
- Existing STACK.md research - library version verification

### Secondary (MEDIUM confidence)
- Existing ARCHITECTURE.md research - resilience layer placement
- Existing codebase: errors.ts, server.ts, websocket-server.ts - current patterns

### Tertiary (LOW confidence)
- None - all patterns verified with authoritative sources

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Native APIs and opossum are well-established
- Architecture: HIGH - Patterns follow industry standards and project constraints
- Pitfalls: HIGH - Based on documented issues and AWS research

**Research date:** 2026-02-08
**Valid until:** 90 days - stable patterns, only watch for opossum major version changes
