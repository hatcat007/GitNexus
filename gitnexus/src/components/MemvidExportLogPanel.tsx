import { Copy, Download, X } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import type { ExportJobStatus, ExportLogEvent, ExportStage } from '../types/memvid-export';

interface MemvidExportLogPanelProps {
  jobId: string;
  events: ExportLogEvent[];
  status: ExportJobStatus | null;
  onClose: () => void;
}

const formatClock = (iso: string): string => {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return '--:--:--';
  return date.toLocaleTimeString([], { hour12: false });
};

export const MemvidExportLogPanel = ({
  jobId,
  events,
  status,
  onClose,
}: MemvidExportLogPanelProps) => {
  const [search, setSearch] = useState('');
  const [stageFilter, setStageFilter] = useState<'all' | ExportStage>('all');
  const containerRef = useRef<HTMLDivElement | null>(null);

  const filteredEvents = useMemo(() => {
    return events.filter((event) => {
      if (stageFilter !== 'all' && event.stage !== stageFilter) return false;
      if (!search.trim()) return true;
      const q = search.toLowerCase();
      return (
        event.message.toLowerCase().includes(q) ||
        event.stage.toLowerCase().includes(q) ||
        event.type.toLowerCase().includes(q)
      );
    });
  }, [events, search, stageFilter]);

  const stageOptions = useMemo(() => {
    const set = new Set<ExportStage>();
    for (const event of events) set.add(event.stage);
    return Array.from(set.values());
  }, [events]);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    el.scrollTop = el.scrollHeight;
  }, [filteredEvents.length]);

  const handleCopyTimeline = async () => {
    const timelineText = filteredEvents
      .map(
        (event) =>
          `${formatClock(event.ts)} ${event.emoji} [${event.stage}] ${Math.round(
            event.progress
          )}%${event.stageProgress !== undefined ? ` (${Math.round(event.stageProgress)}%)` : ''} - ${
            event.message
          }`
      )
      .join('\n');

    await navigator.clipboard.writeText(timelineText || 'No timeline events.');
  };

  const handleDownloadJson = () => {
    const payload = {
      jobId,
      exportedAt: new Date().toISOString(),
      status,
      events: filteredEvents,
    };
    const blob = new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = `${jobId}.export-events.json`;
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="mt-2 pointer-events-auto bg-surface/95 backdrop-blur border border-border-subtle rounded-xl shadow-2xl overflow-hidden">
      <div className="flex items-center justify-between px-3 py-2 border-b border-border-subtle">
        <div className="text-xs text-text-secondary truncate pr-4">
          Live export timeline · <span className="text-text-primary font-medium">{jobId}</span>
        </div>
        <button
          onClick={onClose}
          className="p-1 rounded hover:bg-hover text-text-muted hover:text-text-primary transition-colors"
          aria-label="Close export timeline panel"
        >
          <X size={14} />
        </button>
      </div>

      <div className="flex items-center gap-2 px-3 py-2 border-b border-border-subtle">
        <input
          value={search}
          onChange={(event) => setSearch(event.target.value)}
          placeholder="Filter logs..."
          className="flex-1 bg-elevated border border-border-subtle rounded px-2 py-1 text-xs text-text-primary placeholder:text-text-muted outline-none focus:ring-1 focus:ring-accent"
        />
        <select
          value={stageFilter}
          onChange={(event) => setStageFilter(event.target.value as 'all' | ExportStage)}
          className="bg-elevated border border-border-subtle rounded px-2 py-1 text-xs text-text-secondary"
        >
          <option value="all">All stages</option>
          {stageOptions.map((stage) => (
            <option key={stage} value={stage}>
              {stage}
            </option>
          ))}
        </select>
        <button
          onClick={() => {
            void handleCopyTimeline();
          }}
          className="inline-flex items-center gap-1 px-2 py-1 text-xs rounded bg-elevated hover:bg-hover border border-border-subtle text-text-secondary"
          title="Copy timeline text"
        >
          <Copy size={12} />
          Copy
        </button>
        <button
          onClick={handleDownloadJson}
          className="inline-flex items-center gap-1 px-2 py-1 text-xs rounded bg-elevated hover:bg-hover border border-border-subtle text-text-secondary"
          title="Download timeline JSON"
        >
          <Download size={12} />
          JSON
        </button>
      </div>

      <div ref={containerRef} className="max-h-72 overflow-auto px-2 py-2 space-y-1.5">
        {filteredEvents.length === 0 ? (
          <div className="text-xs text-text-muted px-2 py-2">No events yet.</div>
        ) : (
          filteredEvents.map((event) => (
            <div
              key={`${event.seq}-${event.ts}`}
              className="grid grid-cols-[68px_20px_1fr_auto] items-start gap-2 px-2 py-1.5 rounded hover:bg-hover/70"
            >
              <span className="text-[11px] text-text-muted font-mono">{formatClock(event.ts)}</span>
              <span className="text-sm leading-none mt-0.5">{event.emoji}</span>
              <div className="min-w-0">
                <div className="text-xs text-text-primary truncate">{event.message}</div>
                <div className="text-[11px] text-text-muted">
                  {event.stage} · {event.type}
                </div>
              </div>
              <span className="text-[11px] text-text-secondary whitespace-nowrap">
                {Math.round(event.progress)}%
                {event.stageProgress !== undefined ? ` · ${Math.round(event.stageProgress)}%` : ''}
              </span>
            </div>
          ))
        )}
      </div>
    </div>
  );
};
