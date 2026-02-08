import pino from 'pino';

/**
 * Logger configuration for GitNexus MCP server
 * Uses pino for structured JSON logging with request context
 */

const logLevel = process.env.LOG_LEVEL || 'info';

// Configure pino-pretty for development (colorized output)
// IMPORTANT: All transports write to stderr (fd 2) to avoid polluting
// the MCP stdio transport which uses stdout for JSON-RPC messages.
const transport = process.env.NODE_ENV !== 'production'
    ? {
        target: 'pino-pretty',
        options: {
            colorize: true,
            translateTime: 'SYS:standard',
            ignore: 'pid,hostname',
            destination: 2, // stderr
        },
    }
    : undefined;

/**
 * Base logger instance with level from LOG_LEVEL env var (default: 'info')
 * Writes to stderr to keep stdout clean for MCP protocol.
 */
export const logger = pino({
    level: logLevel,
    transport,
}, transport ? undefined : pino.destination(2));

/**
 * Creates a child logger with request context for tracing
 * 
 * @param requestId - Unique identifier for the request
 * @param toolName - Optional name of the MCP tool being invoked
 * @returns Child logger with requestId, tool, and agent context
 */
export function createRequestLogger(requestId: string, toolName?: string): pino.Logger {
    const agentName = process.env.GITNEXUS_AGENT || 'unknown';
    
    return logger.child({
        requestId,
        tool: toolName,
        agent: agentName,
    });
}

export default logger;
