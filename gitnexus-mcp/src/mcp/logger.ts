import pino from 'pino';

/**
 * Logger configuration for GitNexus MCP server
 * Uses pino for structured JSON logging with request context
 */

const logLevel = process.env.LOG_LEVEL || 'info';

// Configure pino-pretty for development (colorized output)
const transport = process.env.NODE_ENV !== 'production'
    ? {
        target: 'pino-pretty',
        options: {
            colorize: true,
            translateTime: 'SYS:standard',
            ignore: 'pid,hostname',
        },
    }
    : undefined;

/**
 * Base logger instance with level from LOG_LEVEL env var (default: 'info')
 */
export const logger = pino({
    level: logLevel,
    transport,
});

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
