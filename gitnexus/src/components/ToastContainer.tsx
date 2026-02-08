/**
 * Toast Container
 * 
 * Renders toast notifications in bottom-right corner.
 * Listens to the global toast event system from useToast.
 * Supports: info, success, warning, error, changelog (expandable).
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import {
  X, ChevronDown, ChevronUp,
  Info, CheckCircle, AlertTriangle, AlertCircle,
  FileText, FilePlus, FileEdit, FileX,
  RefreshCw,
} from 'lucide-react';
import {
  type Toast,
  type ChangeLogEntry,
  onToast,
  onDismiss,
  onClear,
} from '../hooks/useToast';

const ICON_MAP = {
  info: Info,
  success: CheckCircle,
  warning: AlertTriangle,
  error: AlertCircle,
  changelog: FileText,
} as const;

const COLOR_MAP = {
  info: {
    border: 'border-blue-500/30',
    bg: 'bg-blue-500/10',
    icon: 'text-blue-400',
    bar: 'bg-blue-500',
  },
  success: {
    border: 'border-green-500/30',
    bg: 'bg-green-500/10',
    icon: 'text-green-400',
    bar: 'bg-green-500',
  },
  warning: {
    border: 'border-amber-500/30',
    bg: 'bg-amber-500/10',
    icon: 'text-amber-400',
    bar: 'bg-amber-500',
  },
  error: {
    border: 'border-red-500/30',
    bg: 'bg-red-500/10',
    icon: 'text-red-400',
    bar: 'bg-red-500',
  },
  changelog: {
    border: 'border-accent/30',
    bg: 'bg-accent/10',
    icon: 'text-accent',
    bar: 'bg-accent',
  },
} as const;

function ChangeIcon({ type }: { type: ChangeLogEntry['type'] }) {
  switch (type) {
    case 'added': return <FilePlus className="w-3.5 h-3.5 text-green-400 flex-shrink-0" />;
    case 'modified': return <FileEdit className="w-3.5 h-3.5 text-amber-400 flex-shrink-0" />;
    case 'deleted': return <FileX className="w-3.5 h-3.5 text-red-400 flex-shrink-0" />;
  }
}

function ToastItem({ toast, onClose }: { toast: Toast; onClose: () => void }) {
  const [expanded, setExpanded] = useState(false);
  const [exiting, setExiting] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const colors = COLOR_MAP[toast.type];
  const Icon = ICON_MAP[toast.type];

  const handleClose = useCallback(() => {
    setExiting(true);
    setTimeout(onClose, 200);
  }, [onClose]);

  // Auto-dismiss
  useEffect(() => {
    if (toast.duration && toast.duration > 0) {
      timerRef.current = setTimeout(handleClose, toast.duration);
      return () => {
        if (timerRef.current) clearTimeout(timerRef.current);
      };
    }
  }, [toast.duration, handleClose]);

  const hasChanges = toast.changes && toast.changes.length > 0;
  const addedCount = toast.changes?.filter(c => c.type === 'added').length ?? 0;
  const modifiedCount = toast.changes?.filter(c => c.type === 'modified').length ?? 0;
  const deletedCount = toast.changes?.filter(c => c.type === 'deleted').length ?? 0;

  return (
    <div
      className={`
        relative w-[380px] rounded-xl border ${colors.border} ${colors.bg}
        backdrop-blur-xl shadow-2xl overflow-hidden
        transition-all duration-200 ease-out
        ${exiting ? 'opacity-0 translate-x-8 scale-95' : 'opacity-100 translate-x-0 scale-100'}
      `}
      style={{ animation: exiting ? undefined : 'toast-enter 0.3s ease-out' }}
    >
      {/* Progress bar for auto-dismiss */}
      {toast.duration && toast.duration > 0 && (
        <div className="absolute top-0 left-0 right-0 h-0.5">
          <div
            className={`h-full ${colors.bar} opacity-50`}
            style={{
              animation: `toast-progress ${toast.duration}ms linear forwards`,
            }}
          />
        </div>
      )}

      <div className="px-4 py-3">
        <div className="flex items-start gap-3">
          <div className={`p-1.5 rounded-lg ${colors.bg} mt-0.5`}>
            <Icon className={`w-4 h-4 ${colors.icon}`} />
          </div>

          <div className="flex-1 min-w-0">
            <div className="flex items-center justify-between gap-2">
              <h4 className="text-sm font-semibold text-text-primary truncate">
                {toast.title}
              </h4>
              <button
                onClick={handleClose}
                className="p-1 text-text-muted hover:text-text-primary rounded transition-colors flex-shrink-0"
              >
                <X className="w-3.5 h-3.5" />
              </button>
            </div>

            {toast.message && (
              <p className="text-xs text-text-secondary mt-0.5 leading-relaxed">
                {toast.message}
              </p>
            )}

            {/* Change summary badges */}
            {hasChanges && (
              <div className="flex items-center gap-2 mt-2">
                {addedCount > 0 && (
                  <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium bg-green-500/15 text-green-400">
                    +{addedCount} added
                  </span>
                )}
                {modifiedCount > 0 && (
                  <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium bg-amber-500/15 text-amber-400">
                    ~{modifiedCount} modified
                  </span>
                )}
                {deletedCount > 0 && (
                  <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium bg-red-500/15 text-red-400">
                    -{deletedCount} deleted
                  </span>
                )}
                <button
                  onClick={() => setExpanded(!expanded)}
                  className="ml-auto p-0.5 text-text-muted hover:text-text-primary transition-colors"
                >
                  {expanded ? <ChevronUp className="w-3.5 h-3.5" /> : <ChevronDown className="w-3.5 h-3.5" />}
                </button>
              </div>
            )}

            {/* Expanded change log */}
            {hasChanges && expanded && (
              <div className="mt-2 max-h-48 overflow-y-auto space-y-0.5 pr-1 custom-scrollbar">
                {toast.changes!.map((change, i) => (
                  <div
                    key={i}
                    className="flex items-center gap-2 py-1 px-2 rounded-md bg-deep/50 text-[11px] font-mono text-text-secondary"
                  >
                    <ChangeIcon type={change.type} />
                    <span className="truncate">{change.path}</span>
                  </div>
                ))}
              </div>
            )}

            {/* Action button */}
            {toast.action && (
              <button
                onClick={() => {
                  toast.action!.onClick();
                  handleClose();
                }}
                className="mt-2 flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium bg-accent text-white hover:bg-accent-dim transition-colors"
              >
                <RefreshCw className="w-3 h-3" />
                {toast.action.label}
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

export function ToastContainer() {
  const [toasts, setToasts] = useState<Toast[]>([]);

  useEffect(() => {
    const unsub1 = onToast((toast) => {
      setToasts(prev => [...prev.slice(-4), toast]); // max 5 visible
    });
    const unsub2 = onDismiss((id) => {
      setToasts(prev => prev.filter(t => t.id !== id));
    });
    const unsub3 = onClear(() => {
      setToasts([]);
    });
    return () => { unsub1(); unsub2(); unsub3(); };
  }, []);

  if (toasts.length === 0) return null;

  return (
    <>
      {/* Keyframe animations */}
      <style>{`
        @keyframes toast-enter {
          from { opacity: 0; transform: translateX(20px) scale(0.95); }
          to   { opacity: 1; transform: translateX(0) scale(1); }
        }
        @keyframes toast-progress {
          from { width: 100%; }
          to   { width: 0%; }
        }
      `}</style>

      <div className="fixed bottom-4 right-4 z-[9999] flex flex-col-reverse gap-2 pointer-events-none">
        {toasts.map(toast => (
          <div key={toast.id} className="pointer-events-auto">
            <ToastItem
              toast={toast}
              onClose={() => setToasts(prev => prev.filter(t => t.id !== toast.id))}
            />
          </div>
        ))}
      </div>
    </>
  );
}
