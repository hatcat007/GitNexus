/**
 * Session Store
 * 
 * IndexedDB-backed persistence for full working sessions.
 * Uses the `idb` library for ergonomic async/await access.
 * 
 * Each session stores: graph data, file contents, chat messages, UI state,
 * and source metadata (zip filename or GitHub URL + branch).
 */

import { openDB, type IDBPDatabase } from 'idb';
import type { GraphNode, GraphRelationship, NodeLabel } from '../core/graph/types';
import type { ChatMessage } from '../core/llm/types';
import type { EdgeType } from '../lib/constants';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface SessionSource {
  type: 'zip' | 'github';
  fileName?: string;   // for zip
  url?: string;        // for github
  branch?: string;     // for github (default: 'main')
}

export interface SessionUIState {
  visibleLabels: NodeLabel[];
  visibleEdgeTypes: EdgeType[];
  depthFilter: number | null;
  isRightPanelOpen: boolean;
  rightPanelTab: 'code' | 'chat';
  isCodePanelOpen: boolean;
}

export interface SavedSession {
  id: string;
  name: string;
  createdAt: number;
  updatedAt: number;
  source: SessionSource;
  graph: {
    nodes: GraphNode[];
    relationships: GraphRelationship[];
  };
  fileContents: Record<string, string>;
  chatMessages: ChatMessage[];
  uiState: SessionUIState;
}

/** Lightweight metadata returned by listSessions (no heavy payload). */
export interface SessionMeta {
  id: string;
  name: string;
  createdAt: number;
  updatedAt: number;
  source: SessionSource;
  nodeCount: number;
  fileCount: number;
}

// ---------------------------------------------------------------------------
// Database setup
// ---------------------------------------------------------------------------

const DB_NAME = 'gitnexus-sessions';
const DB_VERSION = 1;

let dbPromise: Promise<IDBPDatabase> | null = null;

function getDB(): Promise<IDBPDatabase> {
  if (!dbPromise) {
    dbPromise = openDB(DB_NAME, DB_VERSION, {
      upgrade(db) {
        if (!db.objectStoreNames.contains('sessions')) {
          db.createObjectStore('sessions', { keyPath: 'id' });
        }
        if (!db.objectStoreNames.contains('meta')) {
          db.createObjectStore('meta');
        }
      },
    });
  }
  return dbPromise;
}

// ---------------------------------------------------------------------------
// CRUD operations
// ---------------------------------------------------------------------------

/**
 * List all sessions (metadata only — no graph/file payloads).
 * Sorted by updatedAt descending (most recent first).
 */
export async function listSessions(): Promise<SessionMeta[]> {
  const db = await getDB();
  const all: SavedSession[] = await db.getAll('sessions');

  return all
    .map((s) => ({
      id: s.id,
      name: s.name,
      createdAt: s.createdAt,
      updatedAt: s.updatedAt,
      source: s.source,
      nodeCount: s.graph?.nodes?.length ?? 0,
      fileCount: Object.keys(s.fileContents ?? {}).length,
    }))
    .sort((a, b) => b.updatedAt - a.updatedAt);
}

/** Get a full session by ID. */
export async function getSession(id: string): Promise<SavedSession | undefined> {
  const db = await getDB();
  return db.get('sessions', id);
}

/** Save (create or update) a session. */
export async function saveSession(session: SavedSession): Promise<void> {
  const db = await getDB();
  await db.put('sessions', session);
}

/** Delete a session by ID. */
export async function deleteSession(id: string): Promise<void> {
  const db = await getDB();
  await db.delete('sessions', id);

  // If this was the last session, clear lastSessionId
  const lastId = await getLastSessionId();
  if (lastId === id) {
    await setLastSessionId(null);
  }
}

// ---------------------------------------------------------------------------
// Last-active session tracking
// ---------------------------------------------------------------------------

const LAST_SESSION_KEY = 'lastSessionId';

/** Get the ID of the most recently active session. */
export async function getLastSessionId(): Promise<string | null> {
  const db = await getDB();
  const value = await db.get('meta', LAST_SESSION_KEY);
  return (value as string) ?? null;
}

/** Set the most recently active session ID. */
export async function setLastSessionId(id: string | null): Promise<void> {
  const db = await getDB();
  if (id === null) {
    await db.delete('meta', LAST_SESSION_KEY);
  } else {
    await db.put('meta', id, LAST_SESSION_KEY);
  }
}
