import { useEffect, useCallback, useMemo } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useReplayStore } from '../store/replayStore';
import { getReplaySession } from '../api/replayApi';

export interface SessionSelectionResult {
  sessionId: number | null;
  selectSession: (id: number, options?: { replace?: boolean }) => void;
  clearSession: () => void;
  getSessionPath: (basePath: string, customId?: number | null) => string;
  isValidating: boolean;
}

/**
 * Single validated session selection controller across Replay, Journal, and Analytics (R02-01).
 * Priorities & Invariants:
 * 1. Valid URL `?session=<id>` wins and synchronizes store.
 * 2. Valid persisted `store.sessionId` fallback canonicalizes into URL.
 * 3. Invalid syntax, missing/deleted session, or failed API validation clears selection to `null` and shows actionable picker.
 */
export function useSessionSelection(): SessionSelectionResult {
  const [searchParams, setSearchParams] = useSearchParams();
  const queryClient = useQueryClient();
  const storeSessionId = useReplayStore((state) => state.sessionId);
  const setStoreSession = useReplayStore((state) => state.setSession);
  const clearStoreSession = useReplayStore((state) => state.clearSession);

  const rawUrlParam = searchParams.get('session');
  const urlSessionId = useMemo(() => {
    if (!rawUrlParam || !/^\d+$/.test(rawUrlParam)) return null;
    const parsed = parseInt(rawUrlParam, 10);
    return parsed > 0 ? parsed : null;
  }, [rawUrlParam]);

  // Determine candidate session ID
  const candidateId = urlSessionId !== null ? urlSessionId : storeSessionId;

  // Validate candidate session ID against backend API
  const { data: sessionData, isError, isLoading } = useQuery({
    queryKey: ['replay-session-validation', candidateId],
    queryFn: () => getReplaySession(candidateId!),
    enabled: candidateId !== null,
    retry: false,
    staleTime: 30000,
  });

  const clearSession = useCallback(() => {
    queryClient.removeQueries({ queryKey: ['replay-session-validation'] });
    clearStoreSession();
    useReplayStore.getState().clearSession();
    setSearchParams(
      (prev) => {
        const next = new URLSearchParams(prev);
        next.delete('session');
        return next;
      },
      { replace: true }
    );
  }, [clearStoreSession, queryClient, setSearchParams]);

  // Synchronize store & URL based on validation result
  useEffect(() => {
    if (candidateId === null) return;

    if (isError) {
      clearSession();
      return;
    }

    if (sessionData && sessionData.id === candidateId) {
      if (urlSessionId !== null && storeSessionId !== candidateId) {
        setStoreSession(candidateId);
      } else if (urlSessionId === null && storeSessionId === candidateId) {
        setSearchParams(
          (prev) => {
            const next = new URLSearchParams(prev);
            next.set('session', String(candidateId));
            return next;
          },
          { replace: true }
        );
      }
    }
  }, [candidateId, sessionData, isError, urlSessionId, storeSessionId, setStoreSession, setSearchParams, clearSession]);

  const activeSessionId = sessionData && !isError ? candidateId : null;

  const selectSession = useCallback(
    (id: number, options?: { replace?: boolean }) => {
      setStoreSession(id);
      setSearchParams(
        (prev) => {
          const next = new URLSearchParams(prev);
          next.set('session', String(id));
          return next;
        },
        { replace: options?.replace ?? false }
      );
    },
    [setStoreSession, setSearchParams]
  );

  const getSessionPath = useCallback(
    (basePath: string, customId?: number | null) => {
      const targetId = customId !== undefined ? customId : activeSessionId;
      if (!targetId) return basePath;
      const separator = basePath.includes('?') ? '&' : '?';
      return `${basePath}${separator}session=${targetId}`;
    },
    [activeSessionId]
  );

  return {
    sessionId: activeSessionId,
    selectSession,
    clearSession,
    getSessionPath,
    isValidating: isLoading,
  };
}
