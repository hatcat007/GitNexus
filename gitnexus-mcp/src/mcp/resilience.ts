import CircuitBreaker from 'opossum';

/**
 * Resilience utilities for GitNexus MCP Server
 * 
 * Provides:
 * - Timeout wrapper with configurable limits per tool type
 * - Circuit breaker factory for cascading failure protection
 * - Exponential backoff calculator with Full Jitter
 */

// Tool categorization constants
export const QUICK_TOOLS = ['search', 'grep', 'read', 'context', 'overview', 'highlight'];
export const HEAVY_TOOLS = ['cypher', 'impact', 'explore'];

/**
 * Get timeout duration for a tool based on its category.
 * 
 * @param toolName - Name of the tool
 * @returns Timeout duration in milliseconds
 */
export function getTimeout(toolName: string): number {
  const quick = parseInt(process.env.GITNEXUS_TIMEOUT_QUICK || '60000', 10);
  const heavy = parseInt(process.env.GITNEXUS_TIMEOUT_HEAVY || '120000', 10);
  
  if (QUICK_TOOLS.includes(toolName)) return quick;
  if (HEAVY_TOOLS.includes(toolName)) return heavy;
  return quick; // Default to quick
}

/**
 * Wrap a tool execution with timeout using AbortController.
 * 
 * @param toolName - Name of the tool (used to determine timeout)
 * @param fn - Async function that receives an AbortSignal
 * @returns Promise that rejects on timeout or resolves with function result
 */
export async function withToolTimeout<T>(
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

/**
 * Configuration for circuit breaker behavior.
 */
export interface CircuitBreakerConfig {
  failureThreshold: number;  // Number of consecutive failures before opening
  resetTimeoutMs: number;    // Time in ms before attempting to close
}

/**
 * Create a circuit breaker with consecutive failure tracking.
 * 
 * The circuit breaker protects against cascading failures by:
 * 1. Tracking consecutive failures (not just percentage)
 * 2. Opening after reaching the failure threshold
 * 3. Auto-closing on first successful call after reset timeout
 * 
 * @param callTool - Function to wrap with circuit breaker
 * @param config - Circuit breaker configuration
 * @returns CircuitBreaker instance with event-based failure tracking
 */
export function createCircuitBreaker(
  callTool: (method: string, params: any) => Promise<any>,
  config: CircuitBreakerConfig = { failureThreshold: 5, resetTimeoutMs: 30000 }
): CircuitBreaker {
  const breaker = new CircuitBreaker(callTool, {
    timeout: false,  // We handle timeout separately with AbortController
    errorThresholdPercentage: -1,  // Disable percentage-based
    resetTimeout: config.resetTimeoutMs,
    rollingCountBuckets: 1,
    volumeThreshold: 0,
  });

  // Track consecutive failures for threshold (opossum doesn't do this natively)
  let consecutiveFailures = 0;
  
  breaker.on('failure', () => {
    consecutiveFailures++;
    if (consecutiveFailures >= config.failureThreshold) {
      if (!breaker.opened) {
        breaker.open();
      }
    }
  });
  
  // Reset on success (immediate close - per CONTEXT.md decision)
  breaker.on('success', () => {
    consecutiveFailures = 0;
    if (breaker.halfOpen) {
      breaker.close();  // Close immediately on successful test
    }
  });

  return breaker;
}
