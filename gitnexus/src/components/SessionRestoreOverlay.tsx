import { useState, useEffect, useRef } from 'react';
import { Database, GitBranch, Brain, Sparkles, Check, Anchor, Rocket, Ship } from 'lucide-react';

export type RestorePhase =
  | 'opening-vault'
  | 'rebuilding-map'
  | 'waking-crew'
  | 'checking-treasure'
  | 'finalizing'
  | 'complete-no-changes'
  | 'complete-cached';

export interface RestoreProgress {
  phase: RestorePhase;
  percent: number;
  sessionName?: string;
  stats?: {
    nodeCount: number;
    relationshipCount: number;
    fileCount: number;
    embeddingsRestored: boolean;
    embeddingCount: number;
    chatMessageCount: number;
  };
}

interface SessionRestoreOverlayProps {
  progress: RestoreProgress;
  onContinue: () => void;
}

const PHASE_CONFIG: Record<RestorePhase, {
  icon: typeof Database;
  title: string;
  subtitle: string;
  emoji: string;
}> = {
  'opening-vault': {
    icon: Database,
    title: 'Opening the vault...',
    subtitle: 'Dusting off your saved session from the archives',
    emoji: '🏛️',
  },
  'rebuilding-map': {
    icon: GitBranch,
    title: 'Rebuilding the map...',
    subtitle: 'Connecting all the dots and drawing the treasure map',
    emoji: '🗺️',
  },
  'waking-crew': {
    icon: Ship,
    title: 'Waking up the crew...',
    subtitle: 'KuzuDB is stretching, BM25 is brewing coffee',
    emoji: '⚓',
  },
  'checking-treasure': {
    icon: Brain,
    title: 'Checking the treasure...',
    subtitle: 'Counting all your precious embedding vectors',
    emoji: '💎',
  },
  'finalizing': {
    icon: Sparkles,
    title: 'Polishing the deck...',
    subtitle: 'Almost there, making everything shipshape',
    emoji: '✨',
  },
  'complete-no-changes': {
    icon: Anchor,
    title: 'Nothing new found, captain!',
    subtitle: 'Your codebase is exactly as you left it. Smooth sailing ahead!',
    emoji: '🏴‍☠️',
  },
  'complete-cached': {
    icon: Rocket,
    title: 'Found some gold!',
    subtitle: 'Your embeddings were cached — skipping the heavy lifting!',
    emoji: '🪙',
  },
};

const COMPLETED_PHASES: RestorePhase[] = ['complete-no-changes', 'complete-cached'];

// Completed step checkmarks for the timeline
const PHASE_ORDER: RestorePhase[] = [
  'opening-vault',
  'rebuilding-map',
  'waking-crew',
  'checking-treasure',
  'finalizing',
];

const PHASE_SHORT_LABELS: Record<string, string> = {
  'opening-vault': 'Read DB',
  'rebuilding-map': 'Build Graph',
  'waking-crew': 'Hydrate Engine',
  'checking-treasure': 'Restore Vectors',
  'finalizing': 'Finalize',
};

export const SessionRestoreOverlay = ({ progress, onContinue }: SessionRestoreOverlayProps) => {
  const [countdown, setCountdown] = useState(6);
  const countdownRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const isComplete = COMPLETED_PHASES.includes(progress.phase);
  const config = PHASE_CONFIG[progress.phase];
  const Icon = config.icon;

  // Countdown timer for auto-proceed
  useEffect(() => {
    if (!isComplete) return;
    countdownRef.current = setInterval(() => {
      setCountdown((prev) => {
        if (prev <= 1) {
          clearInterval(countdownRef.current!);
          return 0;
        }
        return prev - 1;
      });
    }, 1000);
    return () => {
      if (countdownRef.current) clearInterval(countdownRef.current);
    };
  }, [isComplete]);

  // Trigger onContinue when countdown reaches 0 (outside render phase)
  useEffect(() => {
    if (countdown === 0 && isComplete) {
      onContinue();
    }
  }, [countdown, isComplete, onContinue]);

  // Find current phase index for timeline
  const currentPhaseIdx = PHASE_ORDER.indexOf(progress.phase);

  return (
    <div className="fixed inset-0 flex flex-col items-center justify-center bg-void z-50 overflow-hidden">
      {/* Background gradient blobs */}
      <div className="absolute inset-0 pointer-events-none">
        <div className="absolute top-1/4 left-1/4 w-[30rem] h-[30rem] bg-accent/8 rounded-full blur-3xl animate-pulse" />
        <div className="absolute bottom-1/4 right-1/4 w-[30rem] h-[30rem] bg-node-interface/8 rounded-full blur-3xl animate-pulse" style={{ animationDelay: '1s' }} />
        {isComplete && (
          <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[40rem] h-[40rem] bg-node-function/5 rounded-full blur-3xl animate-pulse" style={{ animationDelay: '0.5s' }} />
        )}
      </div>

      {/* Main content */}
      <div className="relative z-10 flex flex-col items-center max-w-lg w-full px-6">
        {/* Session name */}
        {progress.sessionName && (
          <p className="text-xs font-mono text-text-muted mb-6 tracking-wider uppercase">
            {progress.sessionName}
          </p>
        )}

        {/* Animated icon */}
        <div className="relative mb-8">
          <div className={`
            w-24 h-24 rounded-2xl flex items-center justify-center
            ${isComplete
              ? 'bg-gradient-to-br from-node-function/20 to-accent/20 border border-node-function/30'
              : 'bg-gradient-to-br from-accent/20 to-node-interface/20 border border-accent/30'
            }
            transition-all duration-700
          `}>
            <Icon className={`w-10 h-10 ${isComplete ? 'text-node-function' : 'text-accent'} transition-colors duration-500`} />
          </div>
          {/* Spinning ring during loading */}
          {!isComplete && (
            <div className="absolute -inset-2 border-2 border-accent/20 border-t-accent rounded-2xl animate-spin" style={{ animationDuration: '2s' }} />
          )}
          {/* Glow pulse on complete */}
          {isComplete && (
            <div className="absolute -inset-3 bg-node-function/10 rounded-2xl blur-xl animate-pulse" />
          )}
        </div>

        {/* Emoji */}
        <p className="text-3xl mb-3 animate-bounce" style={{ animationDuration: '2s' }}>
          {config.emoji}
        </p>

        {/* Title */}
        <h2 className={`
          text-xl font-semibold mb-2 text-center transition-colors duration-500
          ${isComplete ? 'text-node-function' : 'text-text-primary'}
        `}>
          {config.title}
        </h2>

        {/* Subtitle */}
        <p className="text-sm text-text-muted text-center mb-8 max-w-sm">
          {config.subtitle}
        </p>

        {/* Progress bar (only during loading) */}
        {!isComplete && (
          <div className="w-full mb-6">
            <div className="h-1.5 bg-elevated rounded-full overflow-hidden">
              <div
                className="h-full bg-gradient-to-r from-accent via-node-interface to-node-function rounded-full transition-all duration-500 ease-out"
                style={{ width: `${progress.percent}%` }}
              />
            </div>
            <div className="flex justify-between mt-2">
              <span className="text-xs font-mono text-text-muted">
                {progress.percent}%
              </span>
              <span className="text-xs font-mono text-text-muted animate-pulse">
                Loading<span className="inline-block w-6 text-left">...</span>
              </span>
            </div>
          </div>
        )}

        {/* Phase timeline (step indicators) */}
        {!isComplete && (
          <div className="flex items-center gap-1 mb-8 w-full">
            {PHASE_ORDER.map((phase, idx) => {
              const isDone = idx < currentPhaseIdx;
              const isCurrent = idx === currentPhaseIdx;
              return (
                <div key={phase} className="flex-1 flex flex-col items-center gap-1.5">
                  <div className={`
                    w-full h-1 rounded-full transition-all duration-500
                    ${isDone ? 'bg-node-function' : isCurrent ? 'bg-accent animate-pulse' : 'bg-elevated'}
                  `} />
                  <span className={`
                    text-[9px] font-mono transition-colors duration-300 text-center leading-tight
                    ${isDone ? 'text-node-function' : isCurrent ? 'text-accent' : 'text-text-muted/50'}
                  `}>
                    {isDone && <Check className="w-2.5 h-2.5 inline mr-0.5" />}
                    {PHASE_SHORT_LABELS[phase]}
                  </span>
                </div>
              );
            })}
          </div>
        )}

        {/* Stats (on complete) */}
        {isComplete && progress.stats && (
          <div className="grid grid-cols-3 gap-3 mb-8 w-full">
            <StatCard label="Nodes" value={progress.stats.nodeCount} color="text-accent" />
            <StatCard label="Edges" value={progress.stats.relationshipCount} color="text-node-interface" />
            <StatCard label="Files" value={progress.stats.fileCount} color="text-node-file" />
            {progress.stats.embeddingsRestored && (
              <StatCard label="Embeddings" value={progress.stats.embeddingCount} color="text-node-function" icon="cached" />
            )}
            {progress.stats.chatMessageCount > 0 && (
              <StatCard label="Messages" value={progress.stats.chatMessageCount} color="text-node-class" />
            )}
          </div>
        )}

        {/* Funny cached embeddings callout */}
        {isComplete && progress.stats?.embeddingsRestored && (
          <div className="flex items-center gap-2 px-4 py-2.5 bg-node-function/10 border border-node-function/20 rounded-xl mb-6">
            <Sparkles className="w-4 h-4 text-node-function shrink-0" />
            <p className="text-xs text-node-function">
              Embeddings loaded from cache — saved you a trip to the GPU mines!
            </p>
          </div>
        )}

        {/* Continue button + countdown (on complete) */}
        {isComplete && (
          <div className="flex flex-col items-center gap-3">
            <button
              onClick={() => {
                if (countdownRef.current) clearInterval(countdownRef.current);
                onContinue();
              }}
              className="px-6 py-2.5 bg-accent hover:bg-accent/90 text-void font-medium rounded-xl transition-all duration-200 hover:scale-105 active:scale-95 shadow-lg shadow-accent/20"
            >
              Let's go!
            </button>
            <p className="text-xs text-text-muted font-mono">
              Auto-proceeding in <span className="text-accent font-semibold">{countdown}s</span>
            </p>
          </div>
        )}
      </div>
    </div>
  );
};

// Small stat card for the results screen
const StatCard = ({ label, value, color, icon }: { label: string; value: number; color: string; icon?: string }) => (
  <div className="flex flex-col items-center p-3 bg-surface/50 border border-border-subtle rounded-xl">
    <span className={`text-lg font-semibold font-mono ${color}`}>
      {value.toLocaleString()}
    </span>
    <span className="text-[10px] text-text-muted uppercase tracking-wider flex items-center gap-1">
      {icon === 'cached' && <Sparkles className="w-2.5 h-2.5" />}
      {label}
    </span>
  </div>
);
