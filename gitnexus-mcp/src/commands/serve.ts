/**
 * Serve Command
 * 
 * Starts the MCP server that bridges external AI agents to GitNexus.
 * - Listens on stdio for MCP protocol (from AI tools)
 * - Hosts a local WebSocket bridge for the GitNexus browser app
 */

import { startMCPServer } from '../mcp/server.js';
import { WebSocketBridge } from '../bridge/websocket-server.js';

interface ServeOptions {
  port: string;
}

export async function serveCommand(options: ServeOptions) {
  const port = parseInt(options.port, 10);
  
  // Start local WebSocket bridge (browser connects to ws://localhost:<port>)
  const client = new WebSocketBridge(port);
  const started = await client.start();

  if (!started) {
    // Bridge handles Hub→Peer fallback internally on EADDRINUSE.
    // If we still get false, it means even Peer mode failed.
    console.error(`Failed to start GitNexus bridge on port ${port} (Hub and Peer both failed).`);
    console.error('Continuing in stdio-only mode — tool calls will fail until browser connects.');
  }
  
  // Start MCP server on stdio (AI tools connect here)
  await startMCPServer(client);
}
