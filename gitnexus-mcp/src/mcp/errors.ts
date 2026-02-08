import { CallToolResult } from '@modelcontextprotocol/sdk/types.js';

/**
 * GitNexus Error Types and Formatting
 * Provides structured error responses with actionable suggestions
 */

/**
 * Standard error codes for GitNexus MCP server
 */
export const ErrorCodes = {
    VALIDATION_ERROR: 'VALIDATION_ERROR',
    TOOL_NOT_FOUND: 'TOOL_NOT_FOUND',
    BROWSER_DISCONNECTED: 'BROWSER_DISCONNECTED',
    QUERY_TIMEOUT: 'QUERY_TIMEOUT',
    CYPHER_FORBIDDEN: 'CYPHER_FORBIDDEN',
    INTERNAL_ERROR: 'INTERNAL_ERROR',
    // Resilience error codes (Phase 2)
    TIMEOUT: 'TIMEOUT',
    CIRCUIT_OPEN: 'CIRCUIT_OPEN',
    CONNECTION_LOST: 'CONNECTION_LOST',
    RETRY_EXHAUSTED: 'RETRY_EXHAUSTED',
} as const;

export type ErrorCode = typeof ErrorCodes[keyof typeof ErrorCodes];

/**
 * GitNexus error interface with structured information
 */
export interface GitNexusError {
    /** Error code from ErrorCodes */
    code: ErrorCode;
    /** Human-readable error message */
    message: string;
    /** Additional error details */
    details?: Record<string, unknown>;
    /** Actionable suggestion for resolving the error */
    suggestion?: string;
    /** Whether the agent can retry this operation */
    retryable: boolean;
    /** How long to wait (seconds) before retrying, if applicable */
    retryAfter?: number;
}

/**
 * Formats a GitNexusError into an MCP CallToolResult
 * 
 * @param error - The GitNexusError to format
 * @returns CallToolResult with isError: true and JSON content
 */
export function formatError(error: GitNexusError): CallToolResult {
    const errorContent = {
        error: true,
        code: error.code,
        message: error.message,
        details: error.details,
        suggestion: error.suggestion,
        retryable: error.retryable,
        retryAfter: error.retryAfter,
    };

    return {
        content: [
            {
                type: 'text',
                text: JSON.stringify(errorContent, null, 2),
            },
        ],
        isError: true,
    };
}

/**
 * Creates a validation error
 */
export function validationError(message: string, details?: Record<string, unknown>): GitNexusError {
    return {
        code: ErrorCodes.VALIDATION_ERROR,
        message,
        details,
        suggestion: 'Check the input parameters against the expected schema and try again.',
        retryable: false,
    };
}

/**
 * Creates a tool not found error
 */
export function toolNotFoundError(toolName: string): GitNexusError {
    return {
        code: ErrorCodes.TOOL_NOT_FOUND,
        message: `Tool '${toolName}' not found`,
        suggestion: 'Use gitnexus_context to see available tools or check the tool name spelling.',
        retryable: false,
    };
}

/**
 * Creates a Cypher forbidden error
 */
export function cypherForbiddenError(reason: string): GitNexusError {
    return {
        code: ErrorCodes.CYPHER_FORBIDDEN,
        message: 'Cypher query rejected for security reasons',
        details: { reason },
        suggestion: 'Only read-only Cypher queries (MATCH, RETURN, WHERE, WITH, ORDER BY, LIMIT, SKIP) are allowed. Destructive operations (CREATE, MERGE, DELETE, DROP, SET) are forbidden.',
        retryable: false,
    };
}

/**
 * Creates an internal error
 */
export function internalError(message: string, details?: Record<string, unknown>): GitNexusError {
    return {
        code: ErrorCodes.INTERNAL_ERROR,
        message,
        details,
        suggestion: 'This is an unexpected error. Please try again or report the issue if it persists.',
        retryable: true,
    };
}

// ============================================================================
// Resilience Error Helpers (Phase 2)
// ============================================================================

/**
 * Creates a timeout error with retry guidance
 * retryable: true (timeouts are transient, agent can retry)
 */
export function timeoutError(tool: string, timeoutMs: number): GitNexusError {
    const isDebug = process.env.GITNEXUS_DEBUG === 'true';
    return {
        code: ErrorCodes.TIMEOUT,
        message: `Tool '${tool}' exceeded timeout of ${timeoutMs / 1000}s`,
        details: isDebug ? { tool, timeoutMs } : undefined,
        suggestion: 'The operation took too long. Try again with a simpler query or check if the browser is responsive.',
        retryable: true,
    };
}

/**
 * Creates a circuit breaker open error
 * retryable: true, retryAfter: seconds until circuit allows test request
 */
export function circuitOpenError(retryAfterSeconds: number = 30): GitNexusError {
    return {
        code: ErrorCodes.CIRCUIT_OPEN,
        message: `Circuit breaker open due to repeated failures. Will retry in ${retryAfterSeconds} seconds.`,
        details: { retryAfter: retryAfterSeconds },
        suggestion: 'Consider checking browser connection or graph status. Wait before retrying.',
        retryable: true,
        retryAfter: retryAfterSeconds,
    };
}

/**
 * Creates a connection lost error
 * retryable: true (reconnection will be attempted automatically)
 */
export function connectionLostError(reason: string = 'WebSocket disconnected'): GitNexusError {
    return {
        code: ErrorCodes.CONNECTION_LOST,
        message: `Connection lost: ${reason}`,
        suggestion: 'Ensure GitNexus is running in your browser. The connection will be retried automatically.',
        retryable: true,
    };
}

/**
 * Creates a retry exhausted error
 * retryable: false (max attempts reached, requires manual intervention)
 */
export function retryExhaustedError(attempts: number): GitNexusError {
    return {
        code: ErrorCodes.RETRY_EXHAUSTED,
        message: `Maximum reconnection attempts (${attempts}) reached`,
        suggestion: 'Could not reconnect to GitNexus. Please restart the MCP server and ensure GitNexus is running in your browser.',
        retryable: false,
    };
}
