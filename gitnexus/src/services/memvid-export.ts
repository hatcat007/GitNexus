import type { SessionSource } from './session-store';
import type {
  ExportArtifact,
  ExportEventsResponse,
  ExportJobAccepted,
  ExportJobStatus,
  ExportLogEvent,
} from '../types/memvid-export';

const getApiBaseUrl = (): string => {
  const backendMode =
    import.meta.env.VITE_MEMVID_EXPORT_BACKEND_MODE?.trim().toLowerCase() || 'legacy_vps';
  const value =
    (backendMode === 'runpod_queue'
      ? import.meta.env.VITE_MEMVID_EXPORT_API_URL_RUNPOD?.trim()
      : import.meta.env.VITE_MEMVID_EXPORT_API_URL_LEGACY?.trim()) ||
    import.meta.env.VITE_MEMVID_EXPORT_API_URL?.trim();
  if (!value) {
    throw new Error(
      'Missing Memvid API URL environment variable. Set VITE_MEMVID_EXPORT_API_URL or mode-specific URL variables.'
    );
  }
  const normalized = value.replace(/\/+$/, '');
  if (!/^https?:\/\//i.test(normalized)) {
    throw new Error(
      'Invalid Memvid API URL. Include protocol, for example: https://memcapsule-core.fotomagiai.dk'
    );
  }
  return normalized;
};

const getApiHeaders = (contentType: boolean = true): HeadersInit => {
  const headers: Record<string, string> = {};
  if (contentType) {
    headers['Content-Type'] = 'application/json';
  }
  const key = import.meta.env.VITE_MEMVID_EXPORT_API_KEY?.trim();
  if (!key) {
    throw new Error('Missing VITE_MEMVID_EXPORT_API_KEY environment variable.');
  }
  headers.Authorization = `Bearer ${key}`;
  return headers;
};

const parseApiError = async (response: Response): Promise<never> => {
  let detail = response.statusText || 'Request failed';
  try {
    const payload = await response.json();
    detail = payload?.error?.message ?? payload?.message ?? detail;
  } catch {
    // Keep response status text fallback.
  }
  throw new Error(`Memvid export API error (${response.status}): ${detail}`);
};

const toAbsoluteDownloadUrl = (downloadUrl: string): string => {
  if (/^https?:\/\//i.test(downloadUrl)) {
    return downloadUrl;
  }
  return `${getApiBaseUrl()}${downloadUrl.startsWith('/') ? '' : '/'}${downloadUrl}`;
};

const normalizeSlug = (value: string): string =>
  value
    .toLowerCase()
    .replace(/\.zip$/i, '')
    .replace(/\.git$/i, '')
    .replace(/[^a-z0-9._-]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 120);

const parseGitHubRepoName = (url?: string): string | null => {
  if (!url) return null;
  const match = url.match(/github\.com\/[^/]+\/([^/#?]+)/i);
  return match?.[1] ? normalizeSlug(match[1]) : null;
};

export const sourceBaseName = (
  source: SessionSource | null,
  projectName: string
): string => {
  if (!source) {
    return normalizeSlug(projectName || 'project') || 'project';
  }
  if (source.type === 'github') {
    return parseGitHubRepoName(source.url) || normalizeSlug(projectName || 'repo') || 'repo';
  }
  if (source.type === 'zip') {
    return normalizeSlug(source.fileName || projectName || 'archive') || 'archive';
  }
  return normalizeSlug(source.name || projectName || 'folder') || 'folder';
};

export const dateStamp = (date: Date = new Date()): string => {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, '0');
  const d = String(date.getDate()).padStart(2, '0');
  return `${y}-${m}-${d}`;
};

export const buildMemvidFileName = (
  source: SessionSource | null,
  projectName: string,
  date: Date = new Date()
): string => {
  return `${sourceBaseName(source, projectName)}-gitnexus-mem_capsule-${dateStamp(date)}.mv2`;
};

export async function startMemvidExportJob<TRequest extends object>(
  payload: TRequest
): Promise<ExportJobAccepted> {
  const response = await fetch(`${getApiBaseUrl()}/v1/exports`, {
    method: 'POST',
    headers: getApiHeaders(true),
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    await parseApiError(response);
  }

  return response.json() as Promise<ExportJobAccepted>;
}

export async function getMemvidExportJobStatus(jobId: string): Promise<ExportJobStatus> {
  const response = await fetch(`${getApiBaseUrl()}/v1/exports/${encodeURIComponent(jobId)}`, {
    method: 'GET',
    headers: getApiHeaders(false),
  });

  if (!response.ok) {
    await parseApiError(response);
  }

  return response.json() as Promise<ExportJobStatus>;
}

export async function getMemvidExportEvents(
  jobId: string,
  sinceSeq: number = 0,
  limit: number = 200
): Promise<ExportEventsResponse> {
  const clampedLimit = Math.max(1, Math.min(2000, limit));
  const url = new URL(`${getApiBaseUrl()}/v1/exports/${encodeURIComponent(jobId)}/events`);
  url.searchParams.set('sinceSeq', String(Math.max(0, sinceSeq)));
  url.searchParams.set('limit', String(clampedLimit));

  const response = await fetch(url.toString(), {
    method: 'GET',
    headers: getApiHeaders(false),
  });

  if (!response.ok) {
    await parseApiError(response);
  }

  return response.json() as Promise<ExportEventsResponse>;
}

interface StreamMemvidExportEventsOptions {
  sinceSeq?: number;
  onOpen?: () => void;
  onEvent: (event: ExportLogEvent) => void;
  onError?: (error: Error) => void;
  onClose?: () => void;
}

export function streamMemvidExportEvents(
  jobId: string,
  options: StreamMemvidExportEventsOptions
): () => void {
  const controller = new AbortController();
  const sinceSeq = Math.max(0, options.sinceSeq ?? 0);

  (async () => {
    try {
      const url = new URL(
        `${getApiBaseUrl()}/v1/exports/${encodeURIComponent(jobId)}/events/stream`
      );
      url.searchParams.set('sinceSeq', String(sinceSeq));

      const response = await fetch(url.toString(), {
        method: 'GET',
        headers: getApiHeaders(false),
        signal: controller.signal,
      });

      if (!response.ok) {
        await parseApiError(response);
      }

      if (!response.body) {
        throw new Error('Memvid export API stream body is not available.');
      }

      options.onOpen?.();

      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let buffer = '';

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const frames = buffer.split('\n\n');
        buffer = frames.pop() ?? '';

        for (const frame of frames) {
          const lines = frame.split('\n');
          let data = '';
          for (const line of lines) {
            if (line.startsWith('data:')) {
              data += `${line.slice(5).trimStart()}\n`;
            }
          }
          const payload = data.trim();
          if (!payload || payload === 'ping') continue;
          try {
            const parsed = JSON.parse(payload) as ExportLogEvent;
            options.onEvent(parsed);
          } catch {
            // Ignore malformed event payloads from keepalive or proxies.
          }
        }
      }

      options.onClose?.();
    } catch (error) {
      if (controller.signal.aborted) {
        options.onClose?.();
        return;
      }
      const streamError =
        error instanceof Error ? error : new Error('Unknown export stream error');
      options.onError?.(streamError);
    }
  })();

  return () => {
    controller.abort();
  };
}

export async function cancelMemvidExportJob(jobId: string): Promise<ExportJobStatus> {
  const response = await fetch(`${getApiBaseUrl()}/v1/exports/${encodeURIComponent(jobId)}`, {
    method: 'DELETE',
    headers: getApiHeaders(false),
  });

  if (!response.ok) {
    await parseApiError(response);
  }

  return response.json() as Promise<ExportJobStatus>;
}

export async function downloadMemvidArtifact(
  artifact: ExportArtifact,
  fallbackFileName: string
): Promise<void> {
  const response = await fetch(toAbsoluteDownloadUrl(artifact.downloadUrl), {
    method: 'GET',
    headers: getApiHeaders(false),
  });

  if (!response.ok) {
    await parseApiError(response);
  }

  const blob = await response.blob();
  const fileName = artifact.fileName || fallbackFileName;
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = fileName;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}
