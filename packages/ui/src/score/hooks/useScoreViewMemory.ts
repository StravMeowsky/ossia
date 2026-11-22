import { useCallback, useLayoutEffect, useMemo, useRef, useState } from 'react';

import { clampZoom } from '../utils/scoreLayout.ts';
import type { ScoreLayout } from '../utils/scoreLayout.ts';

export interface ScoreViewPan {
  x: number;
  y: number;
}

interface StoredScoreView {
  layout: ScoreLayout;
  zoom: number;
  x: number;
  y: number;
}

interface ScoreViewMemoryState {
  key: string | null;
  view: StoredScoreView;
}

const STORAGE_PREFIX = 'ossia:score-view:v2:';
const SAVE_DELAY_MS = 150;
const DEFAULT_VIEW: StoredScoreView = {
  layout: 'vertical',
  zoom: 100,
  x: 0,
  y: 0,
};

function storageKey(documentId: string | null, versionId: string | null) {
  return documentId ? `${STORAGE_PREFIX}${documentId}:${versionId ?? 'draft'}` : null;
}

function usableOffset(value: unknown) {
  return typeof value === 'number' && Number.isFinite(value) ? value : 0;
}

function usableZoom(value: unknown) {
  return typeof value === 'number' && Number.isFinite(value) ? clampZoom(value) : DEFAULT_VIEW.zoom;
}

function readView(key: string | null): StoredScoreView {
  if (!key) return DEFAULT_VIEW;
  try {
    const parsed = JSON.parse(localStorage.getItem(key) ?? '') as Partial<StoredScoreView>;
    const layout = parsed.layout === 'horizontal' ? 'horizontal' : 'vertical';
    return {
      layout,
      zoom: usableZoom(parsed.zoom),
      x: usableOffset(parsed.x),
      y: usableOffset(parsed.y),
    };
  } catch {
    return DEFAULT_VIEW;
  }
}

/** Persists the layout, zoom and canvas pan offset independently for each score and version. */
export function useScoreViewMemory(documentId: string | null, versionId: string | null) {
  const key = storageKey(documentId, versionId);
  const loaded = useMemo(() => readView(key), [key]);
  const activeKey = useRef<string | null>(key);
  const current = useRef<StoredScoreView>(loaded);
  const timer = useRef<number | null>(null);
  const [state, setState] = useState<ScoreViewMemoryState>(() => ({ key, view: loaded }));
  const view = state.key === key ? state.view : loaded;

  const flush = useCallback(() => {
    if (timer.current !== null) {
      window.clearTimeout(timer.current);
      timer.current = null;
    }
    const storage = activeKey.current;
    if (!storage) return;
    try {
      localStorage.setItem(storage, JSON.stringify(current.current));
    } catch {
      // View memory is optional browser state; storage failures must not block the score page.
    }
  }, []);

  const schedule = useCallback(() => {
    if (timer.current !== null) window.clearTimeout(timer.current);
    timer.current = window.setTimeout(flush, SAVE_DELAY_MS);
  }, [flush]);

  useLayoutEffect(() => {
    flush();
    activeKey.current = key;
    current.current = loaded;
    setState({ key, view: loaded });
    return flush;
  }, [flush, key, loaded]);

  useLayoutEffect(() => {
    const onPageHide = () => flush();
    window.addEventListener('pagehide', onPageHide);
    return () => {
      window.removeEventListener('pagehide', onPageHide);
      flush();
    };
  }, [flush]);

  const setLayout = useCallback(
    (next: ScoreLayout) => {
      const nextView = { ...current.current, layout: next, x: 0, y: 0 };
      current.current = nextView;
      setState({ key, view: nextView });
      schedule();
    },
    [key, schedule],
  );

  const setZoom = useCallback(
    (next: number) => {
      const nextView = { ...current.current, zoom: usableZoom(next) };
      current.current = nextView;
      setState({ key, view: nextView });
      schedule();
    },
    [key, schedule],
  );

  const savePan = useCallback(
    (x: number, y: number) => {
      current.current = {
        ...current.current,
        x: usableOffset(x),
        y: usableOffset(y),
      };
      schedule();
    },
    [schedule],
  );

  return {
    layout: view.layout,
    zoom: view.zoom,
    initialPan: { x: view.x, y: view.y },
    setLayout,
    setZoom,
    savePan,
  };
}
