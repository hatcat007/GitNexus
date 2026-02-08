/**
 * Global Toast Notification System
 * 
 * Lightweight event-emitter pattern — no external deps.
 * Components call `addToast()` from anywhere; ToastContainer listens and renders.
 */

export type ToastType = 'info' | 'success' | 'warning' | 'error' | 'changelog';

export interface ChangeLogEntry {
  type: 'added' | 'modified' | 'deleted';
  path: string;
}

export interface Toast {
  id: string;
  type: ToastType;
  title: string;
  message?: string;
  /** For changelog toasts — expandable file list */
  changes?: ChangeLogEntry[];
  /** Auto-dismiss delay in ms. 0 = persist until dismissed. Default 5000. */
  duration?: number;
  /** Optional action button */
  action?: {
    label: string;
    onClick: () => void;
  };
  createdAt: number;
}

type ToastListener = (toast: Toast) => void;
type DismissListener = (id: string) => void;

const toastListeners = new Set<ToastListener>();
const dismissListeners = new Set<DismissListener>();
const clearListeners = new Set<() => void>();

let counter = 0;

/**
 * Add a toast notification (callable from anywhere)
 */
export function addToast(
  opts: Omit<Toast, 'id' | 'createdAt'>
): string {
  const id = `toast_${++counter}_${Date.now()}`;
  const toast: Toast = {
    ...opts,
    id,
    createdAt: Date.now(),
    duration: opts.duration ?? (opts.type === 'changelog' ? 0 : 5000),
  };
  toastListeners.forEach(l => l(toast));
  return id;
}

/**
 * Dismiss a specific toast
 */
export function dismissToast(id: string) {
  dismissListeners.forEach(l => l(id));
}

/**
 * Clear all toasts
 */
export function clearToasts() {
  clearListeners.forEach(l => l());
}

/** @internal — used by ToastContainer */
export function onToast(listener: ToastListener) {
  toastListeners.add(listener);
  return () => { toastListeners.delete(listener); };
}

/** @internal */
export function onDismiss(listener: DismissListener) {
  dismissListeners.add(listener);
  return () => { dismissListeners.delete(listener); };
}

/** @internal */
export function onClear(listener: () => void) {
  clearListeners.add(listener);
  return () => { clearListeners.delete(listener); };
}
