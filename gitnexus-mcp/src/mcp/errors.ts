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
    };
}
