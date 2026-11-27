import { Check, LoaderCircle, Redo2, Undo2, X } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';

import type { NoteInputAction, NoteInputValue } from '../../generated/protocol.ts';
import { useI18n } from '../../i18n.tsx';
import type { MessageKey } from '../../i18n.tsx';
import type { NoteInput } from '../hooks/useNoteInput.ts';
import { isEditableKeyboardTarget } from '../utils/annotationInput.ts';
import { digitForValue, noteKeyCommand, valueForDigit } from '../utils/noteInputKeys.ts';
import { engraveMei, offsetOfId, voiceOrder } from '../utils/verovio.ts';

const VALUE_LABELS: [number, MessageKey][] = [
  [7, 'noteValueWhole'],
  [6, 'noteValueHalf'],
  [5, 'noteValueQuarter'],
  [4, 'noteValueEighth'],
  [3, 'noteValueSixteenth'],
  [2, 'noteValueThirtySecond'],
];
const VALUE_TEXT: Record<number, string> = { 7: '1', 6: '½', 5: '¼', 4: '⅛', 3: '1/16', 2: '1/32' };
const DYNAMICS = ['pp', 'p', 'mp', 'mf', 'f', 'ff'];

const NEAR_PX = 28;

/**
 * The id of the preview element under a click: a note head, chord or rest, or the nearest one
 * when the click lands between them, as on a staff line or a stem.
 */
function hitId(event: ReactMouseEvent, root: HTMLElement) {
  const target = event.target as Element | null;
  for (let node = target; node && node !== root; node = node.parentElement) {
    const id = node.getAttribute?.('id');
    if (id && offsetOfId(id) !== null) return id;
  }
  let best: { id: string; distance: number } | null = null;
  for (const element of Array.from(root.querySelectorAll('g.note, g.rest, g.mRest'))) {
    const id = element.getAttribute('id');
    if (!id || offsetOfId(id) === null) continue;
    const box = element.getBoundingClientRect();
    const dx = Math.max(box.left - event.clientX, 0, event.clientX - box.right);
    const dy = Math.max(box.top - event.clientY, 0, event.clientY - box.bottom);
    const distance = Math.hypot(dx, dy);
    if (distance <= NEAR_PX && (!best || distance < best.distance)) best = { id, distance };
  }
  return best?.id ?? null;
}

/**
 * Edit mode: a quick Verovio engraving of the edit, a note-input toolbar and keyboard entry.
 * The LilyPond score is rendered once more when the user finishes.
 */
export function NoteEditor({ input }: { input: NoteInput }) {
  const { t } = useI18n();
  const view = useRef<HTMLDivElement>(null);
  const [pages, setPages] = useState<string[]>([]);
  const [width, setWidth] = useState(0);
  const { mei, cursor, value, status } = input;
  const busy = status !== 'ready';

  useEffect(() => {
    const element = view.current;
    if (!element) return;
    const observer = new ResizeObserver(([entry]) => {
      if (entry) setWidth(Math.round(entry.contentRect.width / 100) * 100);
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (!mei || !width) return;
    let live = true;
    void engraveMei(mei, width - 32).then(
      (next) => live && setPages(next),
      () => live && setPages([]),
    );
    return () => {
      live = false;
    };
  }, [mei, width]);

  const selectedId = useMemo(() => {
    if (cursor === null) return null;
    const root = view.current;
    const exact = root?.querySelector(`[id="o${cursor}"]`);
    return exact ? `o${cursor}` : null;
    // Pages are a dependency so the lookup runs after each engraving.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [cursor, pages]);

  useEffect(() => {
    const root = view.current;
    if (!root) return;
    root
      .querySelectorAll('.is-note-cursor')
      .forEach((node) => node.classList.remove('is-note-cursor'));
    if (!selectedId) return;
    const element = root.querySelector(`[id="${selectedId}"]`);
    element?.classList.add('is-note-cursor');
    element?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  }, [selectedId, pages]);

  const voices = useMemo(() => (mei ? voiceOrder(mei) : new Map<string, string[]>()), [mei]);
  const selectedKind = useMemo(() => {
    if (!selectedId) return null;
    const element = view.current?.querySelector(`[id="${selectedId}"]`);
    return element?.getAttribute('class')?.split(' ')[0] ?? null;
  }, [selectedId]);

  // Start on the first note or rest of the first voice.
  useEffect(() => {
    if (cursor !== null || !pages.length) return;
    const first = voices.values().next().value?.[0];
    const offset = first ? offsetOfId(first) : null;
    if (offset !== null) input.setCursor(offset);
  }, [cursor, input, pages, voices]);

  const latest = useRef({ input, voices, selectedId, selectedKind });
  latest.current = { input, voices, selectedId, selectedKind };

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (
        event.defaultPrevented ||
        event.isComposing ||
        document.querySelector('dialog[open]') ||
        isEditableKeyboardTarget(event.target)
      )
        return;
      const { input, voices, selectedId, selectedKind } = latest.current;
      const command = noteKeyCommand(event, input.value);
      if (!command) return;
      event.preventDefault();
      if (input.status !== 'ready') return;
      switch (command.kind) {
        case 'action':
          input.run(command.action);
          break;
        case 'value':
          input.setValue(command.value);
          // A value typed on a note changes that note; on a rest it sets what comes next.
          if (selectedKind === 'note' || selectedKind === 'chord')
            input.run({ kind: 'set-value', value: command.value });
          break;
        case 'move': {
          if (!selectedId) break;
          for (const ids of voices.values()) {
            const at = ids.indexOf(selectedId);
            if (at < 0) continue;
            const next = ids[at + command.delta];
            const offset = next ? offsetOfId(next) : null;
            if (offset !== null) input.setCursor(offset);
            break;
          }
          break;
        }
        case 'undo':
          input.undo();
          break;
        case 'redo':
          input.redo();
          break;
      }
    };
    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
  }, []);

  const choose = (event: ReactMouseEvent<HTMLDivElement>) => {
    const root = view.current;
    const id = root ? hitId(event, root) : null;
    const offset = id ? offsetOfId(id) : null;
    if (offset !== null) input.setCursor(offset);
  };
  const act = (action: NoteInputAction) => {
    if (!busy) input.run(action);
  };
  const pickValue = (next: NoteInputValue) => {
    input.setValue(next);
    if (!busy && (selectedKind === 'note' || selectedKind === 'chord'))
      input.run({ kind: 'set-value', value: next });
  };
  const digit = digitForValue(value);

  return (
    <div className="note-editor">
      <div className="note-editor-toolbar" role="toolbar" aria-label={t('noteInputTools')}>
        <div className="note-editor-group">
          {VALUE_LABELS.map(([d, label]) => {
            const next = valueForDigit(d);
            return (
              <button
                key={d}
                type="button"
                className="note-editor-button"
                aria-pressed={digit === d}
                aria-label={t(label)}
                data-tooltip={`${t(label)} (${d})`}
                onClick={() => next && pickValue({ ...next, dots: value.dots })}
              >
                {VALUE_TEXT[d]}
              </button>
            );
          })}
          <button
            type="button"
            className="note-editor-button"
            aria-pressed={value.dots > 0}
            aria-label={t('noteValueDot')}
            data-tooltip={`${t('noteValueDot')} (.)`}
            onClick={() => pickValue({ log: value.log, dots: value.dots ? 0 : 1 })}
          >
            ·
          </button>
          <button
            type="button"
            className="note-editor-button"
            aria-label={t('noteInputRest')}
            data-tooltip={`${t('noteInputRest')} (0)`}
            onClick={() => act({ kind: 'rest', value })}
          >
            𝄽
          </button>
        </div>
        <div className="note-editor-group">
          <button
            type="button"
            className="note-editor-button"
            aria-label={t('noteInputTie')}
            data-tooltip={`${t('noteInputTie')} (T)`}
            onClick={() => act({ kind: 'toggle-tie' })}
          >
            ⁀
          </button>
          <button
            type="button"
            className="note-editor-button"
            aria-label={t('noteInputSlur')}
            data-tooltip={`${t('noteInputSlur')} (S)`}
            onClick={() => act({ kind: 'toggle-slur' })}
          >
            ⌒
          </button>
        </div>
        <div className="note-editor-group">
          {DYNAMICS.map((mark) => (
            <button
              key={mark}
              type="button"
              className="note-editor-button note-editor-dynamic"
              aria-label={mark}
              onClick={() => act({ kind: 'dynamic', mark })}
            >
              {mark}
            </button>
          ))}
          <button
            type="button"
            className="note-editor-button"
            aria-label={t('noteInputCrescendo')}
            data-tooltip={t('noteInputCrescendo')}
            onClick={() => act({ kind: 'dynamic', mark: '<' })}
          >
            {'<'}
          </button>
          <button
            type="button"
            className="note-editor-button"
            aria-label={t('noteInputDiminuendo')}
            data-tooltip={t('noteInputDiminuendo')}
            onClick={() => act({ kind: 'dynamic', mark: '>' })}
          >
            {'>'}
          </button>
        </div>
        <div className="note-editor-group">
          <button
            type="button"
            className="note-editor-button"
            aria-label={t('noteInputUndo')}
            data-tooltip={t('noteInputUndo')}
            disabled={!input.canUndo || busy}
            onClick={input.undo}
          >
            <Undo2 aria-hidden="true" size={16} />
          </button>
          <button
            type="button"
            className="note-editor-button"
            aria-label={t('noteInputRedo')}
            data-tooltip={t('noteInputRedo')}
            disabled={!input.canRedo || busy}
            onClick={input.redo}
          >
            <Redo2 aria-hidden="true" size={16} />
          </button>
        </div>
        <div className="note-editor-group note-editor-end">
          <span className="note-editor-hint">{t('noteInputHint')}</span>
          <button
            type="button"
            className="note-editor-button note-editor-text"
            disabled={busy}
            onClick={input.discard}
          >
            <X aria-hidden="true" size={15} />
            {t('noteInputDiscard')}
          </button>
          <button
            type="button"
            className="note-editor-button note-editor-text note-editor-primary"
            disabled={busy}
            onClick={input.finish}
          >
            {status === 'saving' ? (
              <LoaderCircle className="score-playback-spinner" aria-hidden="true" size={15} />
            ) : (
              <Check aria-hidden="true" size={15} />
            )}
            {status === 'saving' ? t('noteInputSaving') : t('noteInputDone')}
          </button>
        </div>
      </div>
      <div className="note-editor-view" ref={view} onClick={choose}>
        {status === 'starting' && !pages.length && (
          <div className="note-editor-loading">
            <LoaderCircle className="score-playback-spinner" aria-hidden="true" size={20} />
          </div>
        )}
        {pages.map((svg, index) => (
          // The SVG is engraved locally from MEI the backend wrote; it holds no user markup.
          // eslint-disable-next-line react/no-danger
          <div key={index} className="note-editor-page" dangerouslySetInnerHTML={{ __html: svg }} />
        ))}
      </div>
    </div>
  );
}
