import { useCallback, useEffect, useReducer, useRef, useState } from 'react';
import type { RefObject } from 'react';

import type { DocumentContext, UiLanguage } from '../../generated/protocol.ts';
import { matchSystemLanguage } from '../../i18n.tsx';
import { ApiError, delay } from '../utils/api.ts';
import type { Api } from '../utils/api.ts';
import { initialState, reducer } from '../utils/state.ts';

/** Publishes document data independently of status and version metadata. */
export function useScoreSync(
  api: Api,
  lifetime: AbortController,
  documentIdRef: RefObject<string | null>,
  onLanguage: (language: UiLanguage) => void,
  onError: (error: unknown) => void,
) {
  const [state, dispatch] = useReducer(reducer, initialState);
  const contextRef = useRef<DocumentContext | null>(null);
  const serial = useRef(0);
  const languageCursor = useRef<{ instanceId: string; seq: number } | null>(null);
  const [notFound, setNotFound] = useState(!documentIdRef.current);
  const applyContext = useCallback((context: DocumentContext) => {
    if (context.document.documentId !== documentIdRef.current) return;
    const previous = contextRef.current;
    if (
      previous?.instanceId === context.instanceId &&
      previous.document.documentId === context.document.documentId &&
      previous.seq > context.seq
    )
      return;
    contextRef.current = context;
    dispatch({ type: 'context', context });
  }, []);
  const refresh = useCallback(async () => {
    const generation = ++serial.current;
    const documentId = documentIdRef.current;
    const current = () => generation === serial.current && !lifetime.signal.aborted;
    dispatch({ type: 'start', documentId });
    if (contextRef.current?.document.documentId !== documentId) contextRef.current = null;
    const statusRequest = api
      .status()
      .then((status) => {
        if (current()) {
          dispatch({ type: 'status', status });
          const cursor = languageCursor.current;
          if (
            status.language &&
            (!cursor || cursor.instanceId !== status.instanceId || status.seq >= cursor.seq)
          ) {
            languageCursor.current = { instanceId: status.instanceId, seq: status.seq };
            onLanguage(status.language);
          }
        }
        return status;
      })
      .catch((error: unknown) => {
        if (current()) {
          dispatch({ type: 'status-error' });
          onError(error);
        }
        return null;
      });
    if (documentId) {
      void api
        .history(documentId)
        .then((versions) => {
          if (current()) dispatch({ type: 'versions', versions });
        })
        .catch((error: unknown) => {
          if (!current()) return;
          dispatch({ type: 'versions-error' });
          if (!(error instanceof ApiError && error.status === 404)) onError(error);
        });
      let context: DocumentContext;
      try {
        context = await api.context(documentId);
      } catch (error) {
        if (!current()) return null;
        if (!(error instanceof ApiError && (error.status === 404 || error.code === 'not-found')))
          throw error;
        contextRef.current = null;
        dispatch({ type: 'missing' });
        setNotFound(true);
        const status = await statusRequest;
        return current() && status ? { instanceId: status.instanceId, seq: status.seq } : null;
      }
      if (!current()) return null;
      applyContext(context);
      setNotFound(false);
      dispatch({ type: 'connection', connection: 'connected' });
      const latest = contextRef.current ?? context;
      return { instanceId: latest.instanceId, seq: latest.seq };
    }
    setNotFound(true);
    const status = await statusRequest;
    if (!current() || !status) return null;
    dispatch({ type: 'connection', connection: 'connected' });
    return { instanceId: status.instanceId, seq: status.seq };
  }, [api, applyContext, documentIdRef, lifetime, onError, onLanguage]);
  const refreshSelection = useCallback(
    async (documentId: string) => {
      const context = await api.context(documentId);
      if (!lifetime.signal.aborted) applyContext(context);
    },
    [api, applyContext, lifetime],
  );
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    let refreshing = false;
    let again = false;
    let subscribed = false;
    const requestRefresh = () => {
      again = true;
      if (timer || refreshing) return;
      timer = setTimeout(() => {
        timer = undefined;
        refreshing = true;
        void (async () => {
          try {
            do {
              again = false;
              await refresh();
            } while (again && !lifetime.signal.aborted);
          } catch (error) {
            onError(error);
          } finally {
            refreshing = false;
          }
        })();
      }, 40);
    };
    void (async () => {
      while (!lifetime.signal.aborted) {
        try {
          const snapshot = await refresh();
          if (!snapshot) {
            await delay(100, lifetime.signal);
            continue;
          }
          if (subscribed && documentIdRef.current) {
            window.dispatchEvent(
              new CustomEvent('ossia-score-settings-changed', {
                detail: { documentId: documentIdRef.current },
              }),
            );
            window.dispatchEvent(
              new CustomEvent('ossia-drawing-changed', {
                detail: { documentId: documentIdRef.current },
              }),
            );
          }
          subscribed = true;
          await api.events(snapshot, (event) => {
            if (event.type === 'ui-language-changed') {
              const cursor = languageCursor.current;
              if (cursor?.instanceId === event.instanceId && event.seq < cursor.seq) return;
              languageCursor.current = { instanceId: event.instanceId, seq: event.seq };
              const next = (event.payload as { language?: UiLanguage | null }).language;
              onLanguage(next ?? matchSystemLanguage(navigator.languages));
              return;
            }
            if (event.documentId && event.documentId !== documentIdRef.current) return;
            if (event.type === 'score-settings-changed' && event.documentId) {
              window.dispatchEvent(
                new CustomEvent('ossia-score-settings-changed', {
                  detail: { documentId: event.documentId },
                }),
              );
              return;
            }
            if (event.type === 'drawing-changed' && event.documentId) {
              const revision = (event.payload as { revision?: unknown }).revision;
              window.dispatchEvent(
                new CustomEvent('ossia-drawing-changed', {
                  detail: {
                    documentId: event.documentId,
                    revision: typeof revision === 'number' ? revision : undefined,
                  },
                }),
              );
              return;
            }
            if (event.type === 'selection-changed' && event.documentId) {
              const origin = (event.payload as { clientId?: string | null }).clientId;
              // The originating tab already applied the PUT response.
              if (origin && origin === api.clientId) return;
              if ((contextRef.current?.seq ?? -1) >= event.seq) return;
              void refreshSelection(event.documentId).catch(onError);
              return;
            }
            requestRefresh();
          });
          if (!lifetime.signal.aborted)
            dispatch({ type: 'connection', connection: 'reconnecting' });
        } catch (error) {
          onError(error);
          if (!lifetime.signal.aborted)
            dispatch({ type: 'connection', connection: 'reconnecting' });
        }
        await delay(800, lifetime.signal).catch(() => {});
      }
    })();
    return () => {
      if (timer) clearTimeout(timer);
      lifetime.abort();
    };
  }, [api, lifetime, refresh, refreshSelection, onError, onLanguage]);
  return { state, contextRef, applyContext, refresh, notFound };
}
