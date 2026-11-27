import { invoke } from '@tauri-apps/api/core';
import { LoaderCircle, Plus, X } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';

import { describe } from '../errors.ts';
import type {
  ManagementCreatedScore,
  ManagementNewScoreArgs,
  NewScoreInstrument,
  NewScoreMode,
} from '../generated/protocol.ts';
import { translate, useI18n } from '../i18n.tsx';
import type { MessageKey } from '../i18n.tsx';
import { cancelDialog, closeDialog } from './dialog.ts';
import { Segmented } from './ui.tsx';

const INSTRUMENTS: [NewScoreInstrument, MessageKey][] = [
  ['piano', 'instrumentPiano'],
  ['violin', 'instrumentViolin'],
  ['viola', 'instrumentViola'],
  ['cello', 'instrumentCello'],
];

/** Tonics from seven flats to seven sharps, indexed by `fifths + 7`. */
const TONICS: Record<NewScoreMode, string[]> = {
  major: ['C♭', 'G♭', 'D♭', 'A♭', 'E♭', 'B♭', 'F', 'C', 'G', 'D', 'A', 'E', 'B', 'F♯', 'C♯'],
  minor: ['A♭', 'E♭', 'B♭', 'F', 'C', 'G', 'D', 'A', 'E', 'B', 'F♯', 'C♯', 'G♯', 'D♯', 'A♯'],
};

const TIME_SIGNATURES = ['4/4', '3/4', '2/4', '2/2', '3/8', '6/8', '9/8', '12/8', '5/4', '7/8'];

const BRIEF_LIMIT = 2000;

/** Key choice label, such as "E♭ major · 3♭". */
function keyLabel(language: Parameters<typeof translate>[0], mode: NewScoreMode, fifths: number) {
  const name = translate(language, mode === 'major' ? 'newScoreKeyMajor' : 'newScoreKeyMinor', {
    tonic: TONICS[mode][fifths + 7] ?? '',
  });
  if (fifths === 0) return name;
  return `${name} · ${Math.abs(fifths)}${fifths < 0 ? '♭' : '♯'}`;
}

/** Whole numbers within bounds; anything else is invalid. */
function boundedNumber(text: string, min: number, max: number) {
  if (!/^\d+$/.test(text.trim())) return null;
  const value = Number(text);
  return value >= min && value <= max ? value : null;
}

/**
 * Create a blank score from the library. The score opens in its own window; a composition brief
 * is passed along for the assistant there.
 */
export function NewScoreDialog({
  onClose,
  onCreated,
}: {
  onClose: () => void;
  onCreated: (created: ManagementCreatedScore) => void;
}) {
  const { language, t } = useI18n();
  const dialog = useRef<HTMLDialogElement>(null);
  const [title, setTitle] = useState('');
  const [composer, setComposer] = useState('');
  const [instrument, setInstrument] = useState<NewScoreInstrument>('piano');
  const [mode, setMode] = useState<NewScoreMode>('major');
  const [fifths, setFifths] = useState(0);
  const [time, setTime] = useState('4/4');
  const [tempoText, setTempoText] = useState('');
  const [bpm, setBpm] = useState('');
  const [measures, setMeasures] = useState('16');
  const [brief, setBrief] = useState('');
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    dialog.current?.showModal();
  }, []);

  const measureCount = boundedNumber(measures, 1, 999);
  const tempo = bpm.trim() === '' ? undefined : boundedNumber(bpm, 20, 400);
  const valid = measureCount !== null && tempo !== null;

  // The dialog's close event reports the closing to the page.
  const close = () => {
    if (dialog.current) void closeDialog(dialog.current);
  };

  async function create() {
    if (!valid || measureCount === null || tempo === null || creating) return;
    const [numerator, denominator] = time.split('/').map(Number);
    const composeBrief = brief.trim();
    const args: ManagementNewScoreArgs = {
      score: {
        title: title.trim() || t('newScoreUntitled'),
        composer: composer.trim() || undefined,
        instrument,
        fifths,
        mode,
        timeNumerator: numerator ?? 4,
        timeDenominator: denominator ?? 4,
        tempoText: tempoText.trim() || undefined,
        tempoBpm: tempo,
        measures: measureCount,
      },
      compose: composeBrief
        ? translate(language, 'newScoreComposeMessage', { brief: composeBrief })
        : undefined,
    };
    setCreating(true);
    setError('');
    try {
      const created = await invoke<ManagementCreatedScore>('management_create_score', { args });
      onCreated(created);
      close();
    } catch (caught) {
      setError(`${t('newScoreFailed')}: ${describe(caught)}`);
    } finally {
      setCreating(false);
    }
  }

  return (
    <dialog
      ref={dialog}
      className="service-dialog new-score-dialog"
      aria-labelledby="new-score-title"
      onCancel={cancelDialog}
      onClose={onClose}
    >
      <form
        onSubmit={(event) => {
          event.preventDefault();
          void create();
        }}
      >
        <div className="dialog-heading">
          <h2 id="new-score-title" tabIndex={-1}>
            {t('newScore')}
          </h2>
          <button type="button" className="icon-button" aria-label={t('close')} onClick={close}>
            <X />
          </button>
        </div>
        <div className="dialog-content">
          {error && (
            <p className="error" role="alert">
              {error}
            </p>
          )}
          <div className="settings-list">
            <label className="settings-row">
              <span className="setting-label">{t('newScoreTitle')}</span>
              <input
                className="github-token-input new-score-input"
                type="text"
                autoFocus
                maxLength={200}
                placeholder={t('newScoreUntitled')}
                value={title}
                disabled={creating}
                onChange={(event) => setTitle(event.target.value)}
              />
            </label>
            <label className="settings-row">
              <span className="setting-label">{t('newScoreComposer')}</span>
              <input
                className="github-token-input new-score-input"
                type="text"
                maxLength={200}
                placeholder={t('newScoreOptional')}
                value={composer}
                disabled={creating}
                onChange={(event) => setComposer(event.target.value)}
              />
            </label>
            <label className="settings-row">
              <span className="setting-label">{t('newScoreInstrument')}</span>
              <select
                className="settings-select"
                value={instrument}
                disabled={creating}
                onChange={(event) => setInstrument(event.target.value as NewScoreInstrument)}
              >
                {INSTRUMENTS.map(([value, label]) => (
                  <option key={value} value={value}>
                    {t(label)}
                  </option>
                ))}
              </select>
            </label>
            <div className="settings-row">
              <span className="setting-label">{t('newScoreKey')}</span>
              <div className="new-score-key">
                <Segmented
                  ariaLabel={t('newScoreKey')}
                  value={mode}
                  disabled={creating}
                  options={[
                    { value: 'major', label: t('newScoreMajor') },
                    { value: 'minor', label: t('newScoreMinor') },
                  ]}
                  onChange={(value) => {
                    if (value === 'major' || value === 'minor') setMode(value);
                  }}
                />
                <select
                  className="settings-select"
                  aria-label={t('newScoreKey')}
                  value={fifths}
                  disabled={creating}
                  onChange={(event) => setFifths(Number(event.target.value))}
                >
                  {TONICS[mode].map((_, index) => (
                    <option key={index} value={index - 7}>
                      {keyLabel(language, mode, index - 7)}
                    </option>
                  ))}
                </select>
              </div>
            </div>
            <label className="settings-row">
              <span className="setting-label">{t('newScoreTime')}</span>
              <select
                className="settings-select"
                value={time}
                disabled={creating}
                onChange={(event) => setTime(event.target.value)}
              >
                {TIME_SIGNATURES.map((value) => (
                  <option key={value} value={value}>
                    {value}
                  </option>
                ))}
              </select>
            </label>
            <div className="settings-row">
              <span className="setting-label">{t('newScoreTempo')}</span>
              <div className="new-score-tempo">
                <input
                  className="github-token-input new-score-input"
                  type="text"
                  maxLength={60}
                  aria-label={t('newScoreTempo')}
                  placeholder={t('newScoreTempoText')}
                  value={tempoText}
                  disabled={creating}
                  onChange={(event) => setTempoText(event.target.value)}
                />
                <input
                  className="github-token-input new-score-number"
                  type="text"
                  inputMode="numeric"
                  aria-label={t('newScoreBpm')}
                  aria-invalid={tempo === null}
                  placeholder={t('newScoreBpm')}
                  value={bpm}
                  disabled={creating}
                  onChange={(event) => setBpm(event.target.value)}
                />
              </div>
            </div>
            <label className="settings-row">
              <span className="setting-label">{t('newScoreMeasures')}</span>
              <input
                className="github-token-input new-score-number"
                type="text"
                inputMode="numeric"
                aria-invalid={measureCount === null}
                value={measures}
                disabled={creating}
                onChange={(event) => setMeasures(event.target.value)}
              />
            </label>
          </div>
          <label className="new-score-compose">
            <span className="setting-copy">
              <span className="setting-label">{t('newScoreCompose')}</span>
              <span className="setting-note">{t('newScoreComposeHint')}</span>
            </span>
            <textarea
              rows={4}
              maxLength={BRIEF_LIMIT}
              placeholder={t('newScoreComposePlaceholder')}
              value={brief}
              disabled={creating}
              onChange={(event) => setBrief(event.target.value)}
            />
          </label>
        </div>
        <footer className="dialog-footer action-row">
          <button type="button" disabled={creating} onClick={close}>
            {t('cancel')}
          </button>
          <button className="primary" type="submit" disabled={!valid || creating}>
            {creating ? (
              <LoaderCircle className="spinner" aria-hidden="true" />
            ) : (
              <Plus aria-hidden="true" />
            )}
            <span>{creating ? t('newScoreCreating') : t('newScoreCreate')}</span>
          </button>
        </footer>
      </form>
    </dialog>
  );
}
