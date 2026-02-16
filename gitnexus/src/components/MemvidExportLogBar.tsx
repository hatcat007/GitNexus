import { ChevronDown, ChevronUp, Clock3, Wifi, WifiOff } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { useAppState } from '../hooks/useAppState';
import { MemvidExportLogPanel } from './MemvidExportLogPanel';
import type { ExportLogEvent } from '../types/memvid-export';

const formatElapsed = (ms: number): string => {
  const seconds = Math.max(0, Math.floor(ms / 1000));
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;
  return `${mins}:${String(secs).padStart(2, '0')}`;
};

const fallbackEvent = (
  message: string,
  progress: number,
  stage: string
): ExportLogEvent => ({
  seq: 0,
  ts: new Date().toISOString(),
  jobId: 'unknown',
  type: 'stage_progress',
  stage: stage as ExportLogEvent['stage'],
  progress,
  stageProgress: progress,
  emoji: '⏳',
  message,
});

export const MemvidExportLogBar = () => {
  const {
    exportStatus,
    exportError,
    exportEventsByJob,
    isExportLogStreamConnected,
    exportStallState,
    activeOrLatestExportJobId,
  } = useAppState();
  const [expanded, setExpanded] = useState(false);
  const [elapsedMs, setElapsedMs] = useState(0);

  const jobId = exportStatus?.jobId ?? activeOrLatestExportJobId;
  const events = useMemo(
    () => (jobId ? exportEventsByJob[jobId] ?? [] : []),
    [exportEventsByJob, jobId]
  );

  const isActive = !!exportStatus && ['queued', 'running'].includes(exportStatus.status);

  useEffect(() => {
    if (!isActive || !exportStatus?.createdAt) {
      setElapsedMs(0);
      return;
    }

    const tick = () => {
      const started = Date.parse(exportStatus.createdAt || '');
      if (Number.isFinite(started)) {
        setElapsedMs(Math.max(0, Date.now() - started));
      }
    };

    tick();
    const timer = setInterval(tick, 1000);
    return () => clearInterval(timer);
  }, [exportStatus?.createdAt, isActive]);

  if (!jobId) return null;

  const lastEvent =
    events[events.length - 1] ??
    fallbackEvent(
      exportStatus?.message || exportError || 'Waiting for export events...',
      exportStatus?.progress ?? 0,
      exportStatus?.currentStage ?? exportStatus?.status ?? 'queued'
    );

  const progress = Math.round(exportStatus?.progress ?? lastEvent.progress ?? 0);
  const stageProgress =
    exportStatus?.stageProgress ??
    lastEvent.stageProgress ??
    (exportStatus?.status === 'completed' ? 100 : undefined);

  return (
    <div className="fixed bottom-9 left-3 right-3 z-40 pointer-events-none">
      <div className="pointer-events-auto rounded-xl border border-border-subtle bg-surface/95 backdrop-blur shadow-xl">
        <button
          onClick={() => setExpanded((prev) => !prev)}
          className="w-full flex items-center gap-2 px-3 py-2 text-left hover:bg-hover/60 transition-colors rounded-xl"
        >
          <span className={`text-base ${isActive ? 'animate-pulse' : ''}`}>{lastEvent.emoji}</span>
          <div className="min-w-0 flex-1">
            <div className="text-xs text-text-primary truncate">
              {lastEvent.message}
              {exportStallState.stalled && (
                <span className="ml-2 text-[11px] px-1.5 py-0.5 rounded bg-warning/20 text-warning">
                  Still working...
                </span>
              )}
            </div>
            <div className="text-[11px] text-text-muted flex items-center gap-2">
              <span>{progress}%</span>
              {stageProgress !== undefined && <span>Stage {Math.round(stageProgress)}%</span>}
              {isActive && (
                <>
                  <span className="inline-flex items-center gap-1">
                    <Clock3 size={11} />
                    {formatElapsed(elapsedMs)}
                  </span>
                  <span className="inline-flex items-center gap-1">
                    {isExportLogStreamConnected ? (
                      <>
                        <Wifi size={11} className="text-node-function" /> Live
                      </>
                    ) : (
                      <>
                        <WifiOff size={11} className="text-warning" /> Fallback
                      </>
                    )}
                  </span>
                </>
              )}
            </div>
          </div>
          <span className="text-text-muted">
            {expanded ? <ChevronDown size={16} /> : <ChevronUp size={16} />}
          </span>
        </button>

        {expanded && (
          <MemvidExportLogPanel
            jobId={jobId}
            events={events}
            status={exportStatus}
            onClose={() => setExpanded(false)}
          />
        )}
      </div>
    </div>
  );
};
