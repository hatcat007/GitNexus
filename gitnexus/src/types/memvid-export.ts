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

export type ExportStage =
  | 'queued'
  | 'transform'
  | 'frame_prep'
  | 'write_capsule'
  | 'build_sidecar'
  | 'finalize'
  | 'download_ready'
  | 'failed'
  | 'canceled'
  | 'expired';

export type ExportEventType =
  | 'job_started'
  | 'stage_progress'
  | 'stage_heartbeat'
  | 'job_completed'
  | 'job_failed'
  | 'job_canceled'
  | 'job_expired';

export interface ExportLogEvent {
  seq: number;
  ts: string;
  jobId: string;
  type: ExportEventType;
  stage: ExportStage;
  progress: number;
  stageProgress?: number;
  emoji: string;
  message: string;
  meta?: Record<string, unknown>;
}

export interface ExportJobAccepted {
  jobId: string;
  status: Extract<ExportJobState, 'queued' | 'running'>;
  progress?: number;
  currentStage?: ExportStage;
  stageProgress?: number;
  message?: string;
  createdAt?: string;
}

export interface ExportJobStatus {
  jobId: string;
  status: ExportJobState;
  progress: number;
  currentStage?: ExportStage;
  stageProgress?: number;
  elapsedMs?: number;
  lastEventSeq?: number;
  message?: string;
  createdAt?: string;
  updatedAt?: string;
  artifact?: ExportArtifact;
  error?: ExportErrorPayload;
}

export interface ExportEventsResponse {
  jobId: string;
  events: ExportLogEvent[];
  nextSeq: number;
  statusSnapshot: ExportJobStatus;
}
