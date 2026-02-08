/**
 * MCP Server
 * 
 * Model Context Protocol server that runs on stdio.
 * External AI tools (Cursor, Claude Code) spawn this process and
 * communicate via stdin/stdout using the MCP protocol.
 * 
 * Exposes:
 * - Tools: search, cypher, blastRadius, highlight
 * - Resources: codebase context (stats, hotspots, folder tree)
 */

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
} from '@modelcontextprotocol/sdk/types.js';
import { GITNEXUS_TOOLS } from './tools.js';
import { logger, createRequestLogger } from './logger.js';
import { formatError, ErrorCodes } from './errors.js';
import { validateToolInput } from './schemas.js';
import { sanitizeCypher } from './cypher-sanitizer.js';
import type { CodebaseContext } from '../bridge/websocket-server.js';

// Interface for anything that can call tools (DaemonClient or WebSocketBridge)
interface ToolCaller {
  callTool(method: string, params: any): Promise<any>;
  disconnect?(): void;
  context?: CodebaseContext | null;
  onContextChange?: (listener: (context: CodebaseContext | null) => void) => () => void;
  isConnected?: boolean;
  mode?: 'hub' | 'peer';
}

/**
 * Format context as markdown for the resource
 */
function formatContextAsMarkdown(context: CodebaseContext): string {
  const { projectName, stats, hotspots, folderTree } = context;
  
  const lines: string[] = [];
  
  lines.push(`# GitNexus: ${projectName}`);
  lines.push('');
  lines.push('This codebase is currently loaded in GitNexus. Use the tools below to explore it.');
  lines.push('');
  
  // Stats
  lines.push('## 📊 Statistics');
  lines.push(`- **Files**: ${stats.fileCount}`);
  lines.push(`- **Functions**: ${stats.functionCount}`);
  if (stats.classCount > 0) lines.push(`- **Classes**: ${stats.classCount}`);
  if (stats.interfaceCount > 0) lines.push(`- **Interfaces**: ${stats.interfaceCount}`);
  if (stats.methodCount > 0) lines.push(`- **Methods**: ${stats.methodCount}`);
  lines.push('');
  
  // Hotspots
  if (hotspots.length > 0) {
    lines.push('## 🔥 Hotspots (Most Connected Nodes)');
    lines.push('');
    hotspots.forEach(h => {
      lines.push(`- \`${h.name}\` (${h.type}) — ${h.connections} connections — ${h.filePath}`);
    });
    lines.push('');
  }
  
  // Folder tree
  if (folderTree) {
    lines.push('## 📁 Project Structure');
    lines.push('```');
    lines.push(projectName + '/');
    lines.push(folderTree);
    lines.push('```');
    lines.push('');
  }
  
  // Usage hints
  lines.push('## 🛠️ Available Tools');
  lines.push('');
  lines.push('- **search**: Semantic + keyword search across codebase');
  lines.push('- **cypher**: Execute Cypher queries on knowledge graph');
  lines.push('- **grep**: Regex pattern search in files');
  lines.push('- **read**: Read file contents');
  lines.push('- **explore**: Deep dive on symbol, cluster, or process');
  lines.push('- **overview**: Codebase map (all clusters + processes)');
  lines.push('- **impact**: Analyze change impact (upstream/downstream)');
  lines.push('- **highlight**: Visualize nodes in graph');
  lines.push('');
  lines.push('## 📝 Graph Schema');
  lines.push('');
  lines.push('**Node Types**: File, Folder, Function, Class, Interface, Method, Community, Process');
  lines.push('');
  lines.push('**Relation**: `CodeRelation` with `type` property:');
  lines.push('- CALLS, IMPORTS, EXTENDS, IMPLEMENTS, CONTAINS, DEFINES');
  lines.push('- MEMBER_OF (symbol → community), STEP_IN_PROCESS (symbol → process)');
  lines.push('');
  lines.push('**Example Cypher Queries**:');
  lines.push('```cypher');
  lines.push('MATCH (f:Function) RETURN f.name LIMIT 10');
  lines.push("MATCH (f:File)-[:CodeRelation {type: 'IMPORTS'}]->(g:File) RETURN f.name, g.name");
  lines.push("MATCH (s)-[:CodeRelation {type: 'MEMBER_OF'}]->(c:Community) RETURN c.label, count(s)");
  lines.push('```');
  
  return lines.join('\n');
}

/**
 * Start the MCP server on stdio transport.
 * 
 * SECURITY NOTE (CVE-2026-25536):
 * This implementation is safe for stdio transport because each AI agent
 * spawns a fresh MCP process. Server and transport instances are never
 * reused across clients - each process invocation gets its own isolated
 * Server and Transport instances.
 * 
 * If migrating to HTTP transport in the future, ensure fresh Server and
 * Transport instances are created per session to prevent cross-client
 * data leakage.
 */
export async function startMCPServer(client: ToolCaller): Promise<void> {
  const server = new Server(
    {
      name: 'gitnexus',
      version: '0.1.0',
    },
    {
      capabilities: {
        tools: {},
        resources: {},
      },
    }
  );

  // Handle list resources request
  server.setRequestHandler(ListResourcesRequestSchema, async () => {
    const context = client.context;
    
    const resources: any[] = [
      {
        uri: 'gitnexus://codebase/health',
        name: 'GitNexus Health',
        description: 'Connection status and graph availability',
        mimeType: 'application/json',
      },
    ];
    
    if (context) {
      resources.unshift({
        uri: 'gitnexus://codebase/context',
        name: `GitNexus: ${context.projectName}`,
        description: `Codebase context for ${context.projectName} (${context.stats.fileCount} files, ${context.stats.functionCount} functions)`,
        mimeType: 'text/markdown',
      });
    }
    
    return { resources };
  });

  // Handle read resource request
  server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
    const { uri } = request.params;
    
    // Health check resource
    if (uri === 'gitnexus://codebase/health') {
      const context = client.context;
      const health = {
        status: client.isConnected ? (context ? 'healthy' : 'no_context') : 'disconnected',
        timestamp: new Date().toISOString(),
        connection: {
          browser: client.isConnected || false,
          mode: client.mode || 'unknown',
        },
        context: context ? {
          project: context.projectName,
          files: context.stats.fileCount,
          functions: context.stats.functionCount,
        } : null,
      };
      return {
        contents: [{ uri, mimeType: 'application/json', text: JSON.stringify(health, null, 2) }],
      };
    }
    
    if (uri === 'gitnexus://codebase/context') {
      const context = client.context;
      
      if (!context) {
        return {
          contents: [
            {
              uri,
              mimeType: 'text/plain',
              text: 'No codebase loaded. Open GitNexus in your browser and load a repository.',
            },
          ],
        };
      }
      
      return {
        contents: [
          {
            uri,
            mimeType: 'text/markdown',
            text: formatContextAsMarkdown(context),
          },
        ],
      };
    }
    
    throw new Error(`Unknown resource: ${uri}`);
  });

  // Handle list tools request
  server.setRequestHandler(ListToolsRequestSchema, async () => ({
    tools: GITNEXUS_TOOLS.map((tool) => ({
      name: tool.name,
      description: tool.description,
      inputSchema: tool.inputSchema,
    })),
  }));

  // Handle tool calls
  server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const { name, arguments: args = {} } = request.params;
    
    // Generate request ID and create child logger
    const requestId = `req_${Date.now()}`;
    const log = createRequestLogger(requestId, name);
    const startTime = Date.now();
    
    log.info({ args }, 'Tool call started');

    try {
      // Step 1: Validate input against Zod schema BEFORE dispatch
      const prefixedName = `gitnexus_${name}` as Parameters<typeof validateToolInput>[0];
      const validation = validateToolInput(prefixedName, args);
      
      if (!validation.success) {
        log.warn({ errors: validation.error.issues }, 'Validation failed');
        return formatError({
          code: ErrorCodes.VALIDATION_ERROR,
          message: 'Invalid input parameters',
          details: { issues: validation.error.issues },
          suggestion: 'Check parameter types and constraints against the tool schema',
          retryable: false,
        });
      }
      
      // Step 2: For cypher tool, sanitize query BEFORE execution
      // Use validated data which has proper types
      const validatedArgs = validation.data;
      if (name === 'cypher' && validatedArgs && typeof validatedArgs === 'object' && 'query' in validatedArgs) {
        const query = (validatedArgs as { query: string }).query;
        const sanitized = sanitizeCypher(query);
        if (!sanitized.valid) {
          log.warn({ query, error: sanitized.error }, 'Cypher query rejected');
          return formatError({
            code: ErrorCodes.CYPHER_FORBIDDEN,
            message: 'Forbidden Cypher operation',
            details: { reason: sanitized.error, keyword: sanitized.forbiddenKeyword },
            suggestion: 'Only read-only queries (MATCH, RETURN, WHERE, WITH, ORDER BY, LIMIT, SKIP) are allowed',
            retryable: false,
          });
        }
        // Update args with sanitized query
        args.query = sanitized.query;
      }
      
      // Step 3: Call the tool handler
      const result = await client.callTool(name, args);
      
      const duration = Date.now() - startTime;
      log.info({ duration }, 'Tool call completed');

      return {
        content: [
          {
            type: 'text',
            text: typeof result === 'string' ? result : JSON.stringify(result, null, 2),
          },
        ],
      };
    } catch (error) {
      const duration = Date.now() - startTime;
      const message = error instanceof Error ? error.message : 'Unknown error';
      
      log.error({ error: message, duration }, 'Tool call failed');
      
      return formatError({
        code: ErrorCodes.INTERNAL_ERROR,
        message: 'Internal server error',
        details: { error: message },
        suggestion: 'This is an unexpected error. Please try again or report the issue if it persists.',
        retryable: true,
      });
    }
  });

  // Connect to stdio transport
  const transport = new StdioServerTransport();
  await server.connect(transport);

  // Graceful shutdown handling
  let isShuttingDown = false;

  async function gracefulShutdown(signal: string) {
    if (isShuttingDown) return;
    isShuttingDown = true;
    
    logger.info({ signal }, 'Starting graceful shutdown');
    
    // 1. Stop accepting new requests
    try {
      await server.close();
      logger.info('MCP server closed');
    } catch (e) {
      logger.error({ error: e }, 'Error closing MCP server');
    }
    
    // 2. Wait briefly for pending requests (WebSocketBridge tracks these)
    await new Promise(resolve => setTimeout(resolve, 2000));
    
    // 3. Close WebSocket connections
    client.disconnect?.();
    logger.info('WebSocket disconnected');
    
    logger.info('Graceful shutdown complete');
    process.exit(0);
  }

  process.on('SIGINT', () => gracefulShutdown('SIGINT'));
  process.on('SIGTERM', () => gracefulShutdown('SIGTERM'));

  // Handle uncaught exceptions
  process.on('uncaughtException', (error) => {
    logger.error({ error }, 'Uncaught exception');
    gracefulShutdown('uncaughtException');
  });

  process.on('unhandledRejection', (reason, promise) => {
    logger.error({ reason }, 'Unhandled rejection');
  });
}
