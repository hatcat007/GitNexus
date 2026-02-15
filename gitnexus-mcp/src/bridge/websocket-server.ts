import { WebSocketServer, WebSocket } from 'ws';
import { createServer as createNetServer } from 'net';
import { BridgeMessage, isRequest, isResponse } from './protocol.js';
import { v4 as uuidv4 } from 'uuid';
import { randomBytes } from 'crypto';
import { calculateBackoff } from '../mcp/resilience.js';

/** Max message size in bytes (1 MB) */
const MAX_MESSAGE_SIZE = 1 * 1024 * 1024;
/** Max messages per second per client before rate-limiting */
const RATE_LIMIT_PER_SEC = 50;
/** Max pending requests before rejecting new ones */
const MAX_PENDING_REQUESTS = 100;

/**
 * Codebase context sent from the GitNexus browser app
 */
export interface CodebaseContext {
  projectName: string;
  stats: {
    fileCount: number;
    functionCount: number;
    classCount: number;
    interfaceCount: number;
    methodCount: number;
  };
  hotspots: Array<{
    name: string;
    type: string;
    filePath: string;
    connections: number;
  }>;
  folderTree: string;
}

/**
 * Check if a Port is available
 */
async function isPortAvailable(port: number): Promise<boolean> {
  return new Promise((resolve) => {
    const server = createNetServer();
    server.once('error', () => resolve(false));
    server.once('listening', () => {
      server.close();
      resolve(true);
    });
    server.listen(port, '127.0.0.1'); // Match Hub's bind address
  });
}

export class WebSocketBridge {
  private wss: WebSocketServer | null = null; // Used if we are the Hub
  private client: WebSocket | null = null;    // Used if we are a Peer (connecting to Hub), OR if we are Hub (clients connecting to us)

  // Hub State
  private browserClient: WebSocket | null = null;
  private peerClients: Map<string, WebSocket> = new Map();

  // Common State
  private pendingRequests: Map<string, { resolve: (val: any) => void, reject: (err: any) => void }> = new Map();
  private requestId = 0;
  private started = false;
  private _context: any | null = null; // CodebaseContext
  private contextListeners: Set<(context: any | null) => void> = new Set();
  private agentName: string;
  private isHub = false;
  private port = 54319;

  // Auth token for secure bridge communication
  private _authToken: string | null = null;

  // Reconnection state
  private reconnectAttempt = 0;
  private reconnectTimer: NodeJS.Timeout | null = null;
  private readonly maxReconnectDelay = 60000;  // 60 second cap

  constructor(port: number = 54319, agentName?: string) {
    this.port = port;
    this.agentName = agentName || process.env.GITNEXUS_AGENT || this.detectAgent();
  }

  private detectAgent(): string {
    if (process.env.CURSOR_SESSION_ID) return 'Cursor';
    if (process.env.CLAUDE_CODE) return 'Claude Code';
    if (process.env.WINDSURF_SESSION) return 'Windsurf';
    return 'Unknown Agent';
  }

  async start(): Promise<boolean> {
    const available = await isPortAvailable(this.port);

    if (available) {
      return this.startAsHub();
    } else {
      return this.startAsPeer();
    }
  }

  // -------------------------------------------------------------------------
  // Hub Implementation (Master)
  // -------------------------------------------------------------------------

  private async startAsHub(): Promise<boolean> {
    // Generate a cryptographic auth token for this Hub session
    this._authToken = process.env.GITNEXUS_TOKEN || randomBytes(24).toString('hex');
    console.error(`Starting as MCP Hub on port ${this.port}`);
    console.error(`Auth token: ${this._authToken}`);
    this.isHub = true;

    return new Promise((resolve) => {
      this.wss = new WebSocketServer({
        port: this.port,
        host: '127.0.0.1',          // Bind to localhost only — no network exposure
        maxPayload: MAX_MESSAGE_SIZE, // Reject oversized messages at the WS layer
      });

      this.wss.on('connection', (ws, req) => {
        // Security: Only allow connections from localhost origins
        const origin = req.headers.origin || '';
        if (origin && !origin.match(/^https?:\/\/(localhost|127\.0\.0\.1)(:\d+)?$/)) {
          console.error(`Rejected connection from non-local origin: ${origin}`);
          ws.close(4003, 'Forbidden: non-local origin');
          return;
        }

        // Rate limiting state per client
        (ws as any)._msgCount = 0;
        (ws as any)._msgResetTimer = setInterval(() => { (ws as any)._msgCount = 0; }, 1000);

        ws.on('message', (data) => {
          // Rate limit check
          (ws as any)._msgCount++;
          if ((ws as any)._msgCount > RATE_LIMIT_PER_SEC) {
            console.error('Rate limit exceeded, dropping message');
            return;
          }
          this.handleHubMessage(ws, data);
        });
        ws.on('close', () => {
          clearInterval((ws as any)._msgResetTimer);
          this.handleHubDisconnect(ws);
        });
        ws.on('error', (err) => console.error('Hub client error:', err));
      });

      this.wss.on('listening', () => {
        this.started = true;
        resolve(true);
      });

      this.wss.on('error', (err: any) => {
        console.error('Hub server error:', err.code || err);
        this.isHub = false;
        // Fall back to Peer mode on address-in-use
        if (err.code === 'EADDRINUSE') {
          console.error('Port in use, falling back to Peer mode');
          this.startAsPeer().then(resolve);
        } else {
          resolve(false);
        }
      });
    });
  }

  private handleHubMessage(ws: WebSocket, data: any) {
    try {
      const msg: BridgeMessage = JSON.parse(data.toString());

      if (msg.type === 'handshake') {
        // Validate auth token from peer
        if (this._authToken && msg.token !== this._authToken) {
          console.error('Rejected peer: invalid auth token');
          ws.send(JSON.stringify({ type: 'handshake_nack', id: msg.id, error: 'Invalid auth token' }));
          ws.close(4001, 'Unauthorized');
          return;
        }
        ws.send(JSON.stringify({ type: 'handshake_ack', id: msg.id }));
        return;
      }

      if (msg.type === 'register_peer') {
        // Peer registering itself
        const peerId = uuidv4();
        this.peerClients.set(peerId, ws);
        (ws as any).peerId = peerId;
        (ws as any).agentName = msg.agentName;
        console.error(`Peer connected: ${msg.agentName} (${peerId})`);

        // Forward current context to new peer if available
        if (this._context) {
          ws.send(JSON.stringify({ type: 'context', params: this._context }));
        }
        return;
      }

      // Handle Context updates (from Browser)
      if (msg.type === 'context') {
        // Browser identified itself (implicitly)
        if (this.browserClient !== ws) {
          if (this.browserClient) this.browserClient.close();
          this.browserClient = ws;
          console.error('Browser connected to Hub');
          this.resetReconnectState();  // Reset backoff on reconnection
        }

        this._context = msg.params;
        this.notifyContextListeners();

        // Broadcast context to all peers
        this.broadcastToPeers(msg);
        return;
      }

      // Handle Tool Calls (Peer/Hub -> Browser)
      if (isRequest(msg)) {
        // If it came from a ws client (Peer), validation needed?
        // We assume it's destined for the Browser
        if (this.browserClient && this.browserClient.readyState === WebSocket.OPEN) {
          // Attach agent info if missing (for UI)
          if (!msg.agentName && (ws as any).agentName) {
            msg.agentName = (ws as any).agentName;
          }
          // Attach peerId so we can route response back
          if (!msg.peerId && (ws as any).peerId) {
            msg.peerId = (ws as any).peerId;
          }

          this.browserClient.send(JSON.stringify(msg));
        } else {
          // Browser not connected, fail
          if (msg.id) {
            ws.send(JSON.stringify({
              id: msg.id,
              error: { message: "Browser not connected. Open GitNexus." }
            }));
          }
        }
        return;
      }

      // Handle Tool Results (Browser -> Peer/Hub)
      if (isResponse(msg)) {
        // Route to the correct peer
        if (msg.peerId && this.peerClients.has(msg.peerId)) {
          const peer = this.peerClients.get(msg.peerId);
          if (peer?.readyState === WebSocket.OPEN) {
            peer.send(JSON.stringify(msg));
          }
        } else {
          // It might be for Us (the Hub)
          this.handleResponseLocal(msg);
        }
        return;
      }

    } catch (e) {
      console.error('Hub: Failed to parse message', e);
    }
  }

  private handleHubDisconnect(ws: WebSocket) {
    if (ws === this.browserClient) {
      console.error('Browser disconnected from Hub');
      this.browserClient = null;
      this._context = null;
      this.notifyContextListeners();

      // Schedule reconnection with exponential backoff
      // Note: Server is passive - we log expected timing, browser initiates actual reconnect
      this.scheduleReconnect(this.reconnectAttempt, () => {
        // The browser needs to reconnect - we just wait
        // When it does, onContextChange will reset the counter
        console.error('Waiting for browser to reconnect...');
      });
    } else {
      const peerId = (ws as any).peerId;
      if (peerId) {
        this.peerClients.delete(peerId);
        console.error(`Peer disconnected: ${peerId}`);
      }
    }
  }

  private broadcastToPeers(msg: any) {
    for (const client of this.peerClients.values()) {
      if (client.readyState === WebSocket.OPEN) {
        client.send(JSON.stringify(msg));
      }
    }
  }

  // -------------------------------------------------------------------------
  // Peer Implementation (Spoke)
  // -------------------------------------------------------------------------

  private async startAsPeer(): Promise<boolean> {
    console.error(`Port ${this.port} busy. Attempting to connect as Peer...`);

    return new Promise((resolve) => {
      const ws = new WebSocket(`ws://localhost:${this.port}`);

      const timeout = setTimeout(() => {
        console.error('Handshake timeout. Port is busy by unknown app.');
        ws.close();
        resolve(false);
      }, 1000);

      ws.on('open', () => {
        // Send Handshake with auth token
        const token = process.env.GITNEXUS_TOKEN || '';
        ws.send(JSON.stringify({ type: 'handshake', id: 'init', token }));
      });

      ws.on('message', (data) => {
        try {
          const msg = JSON.parse(data.toString());

          // Handshake success?
          if (msg.type === 'handshake_ack') {
            clearTimeout(timeout);
            console.error('Handshake successful. Joining as Peer.');

            // Register ourselves
            ws.send(JSON.stringify({
              type: 'register_peer',
              agentName: this.agentName
            }));

            this.client = ws;
            this.started = true;
            resolve(true);
            return;
          }

          // Normal messages from Hub
          this.handlePeerMessage(msg);

        } catch (e) {
          // ignore garbage
        }
      });

      ws.on('error', (err) => {
        console.error('Peer connection error:', err);
        resolve(false);
      });

      // If connection fails immediately
      ws.on('close', () => {
        if (!this.started) resolve(false);
        else {
          this.client = null;
          this._context = null;
          this.notifyContextListeners();
        }
      });
    });
  }

  private handlePeerMessage(msg: BridgeMessage) {
    if (msg.type === 'context') {
      this._context = msg.params;
      this.notifyContextListeners();
      return;
    }

    if (isResponse(msg)) {
      this.handleResponseLocal(msg);
    }
  }

  // -------------------------------------------------------------------------
  // Shared / Public API
  // -------------------------------------------------------------------------

  private handleResponseLocal(msg: any) {
    if (msg.id && this.pendingRequests.has(msg.id)) {
      const { resolve, reject } = this.pendingRequests.get(msg.id)!;
      this.pendingRequests.delete(msg.id);

      if (msg.error) {
        // We'll reject the promise so caller knows
        reject(new Error(msg.error.message));
      } else {
        resolve(msg.result);
      }
    }
  }

  get isConnected(): boolean {
    if (this.isHub) {
      return this.browserClient !== null && this.browserClient.readyState === WebSocket.OPEN;
    } else {
      return this.client !== null && this.client.readyState === WebSocket.OPEN;
    }
  }

  get context(): any {
    return this._context;
  }

  get mode(): 'hub' | 'peer' {
    return this.isHub ? 'hub' : 'peer';
  }

  /** Returns the auth token generated by this Hub session (null if Peer). */
  get authToken(): string | null {
    return this._authToken;
  }

  onContextChange(listener: (context: any) => void) {
    this.contextListeners.add(listener);
    return () => this.contextListeners.delete(listener);
  }

  private notifyContextListeners() {
    this.contextListeners.forEach((listener) => listener(this._context));
  }

  async callTool(method: string, params: any): Promise<any> {
    if (!this.isConnected) {
      if (this.isHub) throw new Error('GitNexus Browser not connected.');
      else throw new Error('GitNexus Hub disonnected.');
    }

    const id = `req_${++this.requestId}`;

    return new Promise((resolve, reject) => {
      // Cap pending requests to prevent memory exhaustion
      if (this.pendingRequests.size >= MAX_PENDING_REQUESTS) {
        reject(new Error('Too many pending requests'));
        return;
      }
      this.pendingRequests.set(id, { resolve, reject });

      const msg: BridgeMessage = {
        id,
        method,
        params,
        agentName: this.agentName,
        // type is implicitly request because of method
      };

      if (this.isHub) {
        // Send directly to browser
        if (this.browserClient && this.browserClient.readyState === WebSocket.OPEN) {
          this.browserClient.send(JSON.stringify(msg));
        } else {
          this.pendingRequests.delete(id);
          reject(new Error('Browser not connected'));
        }
      } else {
        // Send to Hub (who forwards to browser)
        if (this.client && this.client.readyState === WebSocket.OPEN) {
          this.client.send(JSON.stringify(msg));
        } else {
          this.pendingRequests.delete(id);
          reject(new Error('Hub disconnected'));
        }
      }

      setTimeout(() => {
        if (this.pendingRequests.has(id)) {
          this.pendingRequests.delete(id);
          reject(new Error('Request timeout'));
        }
      }, 30000);
    });
  }

  /**
   * Schedule a reconnection attempt with exponential backoff
   */
  private scheduleReconnect(attempt: number, connectFn: () => void): void {
    const delay = calculateBackoff(attempt);
    console.error(`Reconnecting in ${delay}ms (attempt ${attempt + 1})`);

    this.reconnectTimer = setTimeout(() => {
      this.reconnectAttempt = attempt + 1;
      connectFn();
    }, delay);
  }

  /**
   * Reset reconnection state on successful connection
   */
  private resetReconnectState(): void {
    this.reconnectAttempt = 0;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }

  /**
   * Cancel any pending reconnection
   */
  private cancelReconnect(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }

  close() {
    this.cancelReconnect();
    this.wss?.close();
    this.client?.close();
  }

  disconnect() {
    this.close();
  }
}
