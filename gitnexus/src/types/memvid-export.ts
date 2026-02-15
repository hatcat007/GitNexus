import type { GraphNode, GraphRelationship } from '../core/graph/types';
import type { SessionSource } from '../services/session-store';

export type ExportJobState =
  | 'queued'
  | 'running'
  | 'completed'
  | 'failed'
  | 'canceled'
  | 'expired';

export interface ExportSourceDescriptor {
  type: SessionSource['type'];
  baseName: string;
  displayName: string;
  url?: string;
  branch?: string;
  originalFileName?: string;
  folderName?: string;
}

export interface ExportOptions {
  semanticEnabled: boolean;
  maxSnippetChars: number;
  maxNodeFrames: number;
  maxRelationFrames: number;
}

export interface ExportRequest {
  sessionId: string;
  projectName: string;
  source: ExportSourceDescriptor;
  nodes: GraphNode[];
  relationships: GraphRelationship[];
  fileContents: Record<string, string>;
  options: ExportOptions;
}

export interface ExportArtifact {
  fileName: string;
  downloadUrl: string;
  expiresAt: string;
  sizeBytes?: number;
}

export interface ExportErrorPayload {
  code: string;
  message: string;
}

export interface ExportJobAccepted {
  jobId: string;
  status: Extract<ExportJobState, 'queued' | 'running'>;
  progress?: number;
  message?: string;
  createdAt?: string;
}

export interface ExportJobStatus {
  jobId: string;
  status: ExportJobState;
  progress: number;
  message?: string;
  createdAt?: string;
  updatedAt?: string;
  artifact?: ExportArtifact;
  error?: ExportErrorPayload;
}
