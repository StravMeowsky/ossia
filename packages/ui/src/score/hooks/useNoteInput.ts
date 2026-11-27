import { useCallback, useEffect, useRef, useState } from 'react';

import type {
  DocumentState,
  EditSession,
  NoteInputAction,
  NoteInputOutcome,
  NoteInputValue,
} from '../../generated/protocol.ts';
import { useI18n } from '../../i18n.tsx';
import { ApiError } from '../utils/api.ts';
import type { makeApi } from '../utils/api.ts';

type Api = ReturnType<typeof makeApi>;

export type NoteInputStatus = 'starting' | 'ready' | 'saving';

export type NoteInput = {
  status: NoteInputStatus;
  mei: string | null;
  cursor: number | null;
  value: NoteInputValue;
  changed: boolean;
  canUndo: boolean;
  canRedo: boolean;
  setCursor: (offset: number) => void;
  setValue: (value: NoteInputValue) => void;
  run: (action: NoteInputAction) => void;
  undo: () => void;
  redo: () => void;
  finish: () => void;
  discard: () => void;
};

/**
 * One note-input session: an edit begun on entry, actions applied in order, and a LilyPond check
 * and commit when the user finishes. Every source is kept so undo can put it back.
 */
export function useNoteInput(
  api: Api,
  doc: DocumentState | undefined,
  active: boolean,
  onDone: () => void,
  onError: (error: unknown) => void,
): NoteInput {
  const { t } = useI18n();
  const [status, setStatus] = useState<NoteInputStatus>('starting');
  const [outcome, setOutcome] = useState<NoteInputOutcome | null>(null);
  const [cursor, setCursor] = useState<number | null>(null);
  const [value, setValue] = useState<NoteInputValue>({ log: 2, dots: 0 });
  const [position, setPosition] = useState({ index: 0, length: 0 });
  const session = useRef<{ edit: EditSession; documentId: string; startHash: string } | null>(null);
  const history = useRef<string[]>([]);
  const index = useRef(0);
  const hash = useRef('');
  const cursorRef = useRef<number | null>(null);
  cursorRef.current = cursor;
  const queue = useRef<Promise<unknown>>(Promise.resolve());
  const latest = useRef({ onDone, onError });
  latest.current = { onDone, onError };

  const enqueue = useCallback((task: () => Promise<void>) => {
    queue.current = queue.current.then(task).catch((error) => latest.current.onError(error));
  }, []);

  const apply = useCallback((next: NoteInputOutcome, record: boolean) => {
    hash.current = next.sourceHash;
    if (record) {
      history.current = [...history.current.slice(0, index.current + 1), next.source];
      index.current = history.current.length - 1;
    }
    setPosition({ index: index.current, length: history.current.length });
    setOutcome(next);
    setCursor(next.cursor);
  }, []);

  const documentId = doc?.documentId;
  useEffect(() => {
    if (!active || !doc || session.current) return;
    let live = true;
    setStatus('starting');
    enqueue(async () => {
      try {
        // A note-input edit left by a closed page or a restart is taken back with its notes.
        const resumed = doc.editState !== 'idle';
        const edit = await api
          .noteInputBegin(doc.documentId, {
            baseVersionId: doc.headVersionId,
            expectedSourceHash: doc.sourceHash,
          })
          .catch((error: unknown) => {
            throw error instanceof ApiError && error.code === 'edit-busy'
              ? new ApiError(error.status, error.code, t('noteInputBusy'))
              : error;
          });
        session.current = {
          edit,
          documentId: doc.documentId,
          // A resumed edit may hold unsaved notes, so finishing always commits it.
          startHash: resumed ? '' : doc.sourceHash,
        };
        const first = await api.noteInput(doc.documentId, {
          editId: edit.editId,
          expectedSourceHash: doc.sourceHash,
          target: 0,
        });
        history.current = [first.source];
        index.current = 0;
        apply(first, false);
        setCursor(null);
        if (live) setStatus('ready');
      } catch (error) {
        session.current = null;
        latest.current.onDone();
        throw error;
      }
    });
    return () => {
      live = false;
    };
    // The session starts once per entry; later document refreshes come from its own writes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, documentId]);

  useEffect(() => {
    if (active) return;
    session.current = null;
    history.current = [];
    setOutcome(null);
    setCursor(null);
  }, [active]);

  const send = useCallback(
    (action: NoteInputAction | undefined, record: boolean) => {
      enqueue(async () => {
        const current = session.current;
        if (!current) return;
        const next = await api.noteInput(current.documentId, {
          editId: current.edit.editId,
          expectedSourceHash: hash.current,
          target: cursorRef.current ?? 0,
          action,
        });
        apply(next, record);
      });
    },
    [api, apply, enqueue],
  );

  const run = useCallback(
    (action: NoteInputAction) => {
      if (cursorRef.current === null) return;
      send(action, true);
    },
    [send],
  );

  const step = useCallback(
    (delta: -1 | 1) => {
      const target = index.current + delta;
      const source = history.current[target];
      if (source === undefined) return;
      index.current = target;
      send({ kind: 'restore', source }, false);
    },
    [send],
  );

  const end = useCallback(
    (commit: boolean) => {
      enqueue(async () => {
        const current = session.current;
        if (!current) return;
        setStatus('saving');
        try {
          if (commit && hash.current !== current.startHash) {
            const job = await api.check(current.documentId, {
              expect: null,
              reference: null,
              documentId: current.documentId,
              editId: current.edit.editId,
              expectedSourceHash: hash.current,
              waitMs: null,
            });
            await api.waitJob(job);
            await api.commit(current.documentId, {
              editId: current.edit.editId,
              expectedSourceHash: hash.current,
              summary: t('noteInputSummary'),
            });
          } else {
            await api.edit({
              action: 'discard',
              documentId: current.documentId,
              baseVersionId: current.edit.baseVersionId,
              expectedSourceHash: hash.current,
              editId: current.edit.editId,
              adoptWorkspace: false,
              author: 'user',
            });
          }
          session.current = null;
          latest.current.onDone();
        } finally {
          setStatus('ready');
        }
      });
    },
    [api, enqueue, t],
  );

  return {
    status,
    mei: outcome?.mei ?? null,
    cursor,
    value,
    changed: position.index > 0 || position.length > 1,
    canUndo: position.index > 0,
    canRedo: position.index < position.length - 1,
    setCursor,
    setValue,
    run,
    undo: () => step(-1),
    redo: () => step(1),
    finish: () => end(true),
    discard: () => end(false),
  };
}
