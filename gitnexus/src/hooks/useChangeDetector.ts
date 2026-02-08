/**
 * Change Detector Hook
 * 
 * Polls GitHub every 30s using git.listServerRefs() (lightweight — refs only, no objects).
 * When the HEAD SHA changes on the tracked branch, shows a toast with "Reindex now" action.
 * Only active for GitHub sessions in exploring view.
 */

import { useEffect, useRef } from 'react';
import git from 'isomorphic-git';
import http from 'isomorphic-git/http/web';
import { addToast, dismissToast } from './useToast';

const POLL_INTERVAL_MS = 30_000;

// Hosted proxy URL for CORS
const HOSTED_PROXY_URL = 'https://gitnexus.vercel.app/api/proxy';

const createProxiedHttp = (): typeof http => {
  const isDev = typeof window !== 'undefined' && window.location.hostname === 'localhost';
  return {
    request: async (config) => {
      const proxyBase = isDev ? HOSTED_PROXY_URL : '/api/proxy';
      const proxyUrl = `${proxyBase}?url=${encodeURIComponent(config.url)}`;
      return http.request({ ...config, url: proxyUrl });
    },
  };
};

interface UseChangeDetectorOptions {
  /** GitHub repo URL */
  url: string | undefined;
  /** Branch to track */
  branch: string;
  /** Whether polling should be active */
  enabled: boolean;
  /** Callback when user clicks "Reindex now" */
  onReindex: () => void;
}

export function useChangeDetector({ url, branch, enabled, onReindex }: UseChangeDetectorOptions) {
  const lastShaRef = useRef<string | null>(null);
  const toastIdRef = useRef<string | null>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    if (!enabled || !url) {
      // Cleanup
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
      return;
    }

    const checkForChanges = async () => {
      try {
        const proxiedHttp = createProxiedHttp();
        const repoUrl = url.endsWith('.git') ? url : `${url}.git`;

        const refs = await git.listServerRefs({
          http: proxiedHttp,
          url: repoUrl,
          prefix: `refs/heads/${branch}`,
        });

        const headRef = refs.find(r => r.ref === `refs/heads/${branch}`);
        if (!headRef) return;

        const currentSha = headRef.oid;

        // First check — just store the SHA
        if (lastShaRef.current === null) {
          lastShaRef.current = currentSha;
          return;
        }

        // No change
        if (currentSha === lastShaRef.current) return;

        // Change detected!
        console.log(`[ChangeDetector] New commit on ${branch}: ${currentSha.slice(0, 8)} (was ${lastShaRef.current.slice(0, 8)})`);
        lastShaRef.current = currentSha;

        // Dismiss previous "changes detected" toast if still showing
        if (toastIdRef.current) {
          dismissToast(toastIdRef.current);
        }

        toastIdRef.current = addToast({
          type: 'warning',
          title: `New changes on ${branch}`,
          message: `Commit ${currentSha.slice(0, 7)} pushed. Click to update your graph.`,
          duration: 0,
          action: {
            label: 'Reindex now',
            onClick: onReindex,
          },
        });
      } catch (err) {
        // Silently skip — could be rate limit, network error, etc.
        // Will retry on next tick
        console.debug('[ChangeDetector] Poll failed (will retry):', err);
      }
    };

    // Initial check
    checkForChanges();

    // Start polling
    intervalRef.current = setInterval(checkForChanges, POLL_INTERVAL_MS);

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    };
  }, [enabled, url, branch, onReindex]);

  // Reset stored SHA when URL/branch changes
  useEffect(() => {
    lastShaRef.current = null;
  }, [url, branch]);
}
