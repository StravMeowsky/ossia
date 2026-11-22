import { useCallback, useEffect, useRef, useState } from 'react';

import type { Drawing, DrawingStroke } from '../../generated/protocol.ts';
import { ApiError } from '../utils/api.ts';
import type { Api } from '../utils/api.ts';

type Edit = (strokes: DrawingStroke[]) => DrawingStroke[];

/** Keeps local edits visible until the backend has acknowledged them. */
export function useDrawing(api: Api, documentId: string | undefined) {
  const [drawing, setDrawing] = useState<Drawing>({ revision: 0, strokes: [] });
  const [error, setError] = useState(false);
  const [ready, setReady] = useState(false);
  const [saving, setSaving] = useState(false);
  const [dirty, setDirty] = useState(false);
  const requestRefresh = useRef<(revision?: number) => void>(() => {});
  const drainRefresh = useRef<() => void>(() => {});
  const state = useRef({
    documentId,
    base: drawing,
    pending: [] as Edit[],
    running: false,
    generation: 0,
  });

  const publish = useCallback(() => {
    const current = state.current;
    setDrawing({
      ...current.base,
      strokes: current.pending.reduce((strokes, edit) => edit(strokes), current.base.strokes),
    });
  }, []);
  const flush = useCallback(async () => {
    const current = state.current;
    if (current.running || !current.documentId || !current.pending.length) return;
    current.running = true;
    const generation = current.generation;
    try {
      while (current.pending.length && current.generation === generation) {
        const count = current.pending.length;
        const strokes = current.pending
          .slice(0, count)
          .reduce((items, edit) => edit(items), current.base.strokes);
        try {
          const saved = await api.saveDrawing(current.documentId, {
            revision: current.base.revision,
            strokes,
          });
          if (current.generation !== generation) return;
          current.base = saved;
          current.pending.splice(0, count);
          if (!current.pending.length) setDirty(false);
          setError(false);
          publish();
        } catch (reason) {
          if (reason instanceof ApiError && reason.code === 'conflict') {
            const latest = await api.drawing(current.documentId);
            if (current.generation !== generation) return;
            current.base = latest;
            publish();
            continue;
          }
          throw reason;
        }
      }
    } catch {
      if (current.generation === generation) setError(true);
    } finally {
      if (current.generation === generation) {
        current.running = false;
        setSaving(false);
        drainRefresh.current();
      }
    }
  }, [api, publish]);

  useEffect(() => {
    const current = state.current;
    current.generation++;
    const generation = current.generation;
    current.documentId = documentId;
    current.base = { revision: 0, strokes: [] };
    current.pending = [];
    current.running = false;
    setReady(false);
    setSaving(false);
    setDirty(false);
    setError(false);
    publish();
    if (!documentId) return;
    let reading = false;
    let needed = false;
    let forced = false;
    let latestRevision = 0;
    const refresh = async () => {
      if (!needed || reading || current.running || current.pending.length) return;
      if (!forced && latestRevision <= current.base.revision) {
        needed = false;
        return;
      }
      needed = false;
      forced = false;
      reading = true;
      try {
        const latest = await api.drawing(documentId);
        if (current.generation !== generation) return;
        if (current.running || current.pending.length) {
          needed = true;
          forced = true;
          return;
        }
        if (latest.revision >= current.base.revision) current.base = latest;
        publish();
        setReady(true);
        setError(false);
      } catch {
        if (current.generation === generation) setError(true);
      } finally {
        reading = false;
        if (current.generation === generation && needed) void refresh();
      }
    };
    const request = (revision?: number) => {
      if (revision === undefined) forced = true;
      else latestRevision = Math.max(latestRevision, revision);
      needed = true;
      void refresh();
    };
    requestRefresh.current = request;
    drainRefresh.current = () => void refresh();
    const changed = (event: Event) => {
      const detail = (event as CustomEvent<{ documentId: string; revision?: number }>).detail;
      if (detail.documentId === documentId) request(detail.revision);
    };
    window.addEventListener('ossia-drawing-changed', changed);
    request();
    return () => {
      window.removeEventListener('ossia-drawing-changed', changed);
      requestRefresh.current = () => {};
      drainRefresh.current = () => {};
      current.generation++;
    };
  }, [api, documentId, publish]);

  const edit = useCallback(
    (change: Edit) => {
      state.current.pending.push(change);
      setDirty(true);
      setSaving(true);
      publish();
      void flush();
    },
    [flush, publish],
  );

  const retry = useCallback(() => {
    if (state.current.pending.length) {
      setSaving(true);
      void flush();
      return;
    }
    requestRefresh.current();
  }, [flush]);

  useEffect(() => {
    if (!dirty) return;
    const beforeUnload = (event: BeforeUnloadEvent) => {
      event.preventDefault();
    };
    window.addEventListener('beforeunload', beforeUnload);
    return () => window.removeEventListener('beforeunload', beforeUnload);
  }, [dirty]);

  return { drawing, ready, error, saving, dirty, edit, retry };
}

export type DrawingController = ReturnType<typeof useDrawing>;
