import type { NoteInputAction, NoteInputValue } from '../../generated/protocol.ts';

/** What a key press asks the note editor to do. */
export type NoteKeyCommand =
  | { kind: 'action'; action: NoteInputAction }
  | { kind: 'value'; value: NoteInputValue }
  | { kind: 'move'; delta: -1 | 1 }
  | { kind: 'undo' }
  | { kind: 'redo' };

const STEPS = ['c', 'd', 'e', 'f', 'g', 'a', 'b'];

/** MuseScore's number row: 1 = 64th … 5 = quarter … 7 = whole. */
export function valueForDigit(digit: number): NoteInputValue | null {
  if (digit < 1 || digit > 7) return null;
  return { log: 7 - digit, dots: 0 };
}

export function digitForValue(value: NoteInputValue) {
  return 7 - value.log;
}

type KeyLike = Pick<KeyboardEvent, 'key' | 'code' | 'shiftKey' | 'ctrlKey' | 'metaKey' | 'altKey'>;

/** Map a key press to a note-input command, given the value new notes are written with. */
export function noteKeyCommand(event: KeyLike, value: NoteInputValue): NoteKeyCommand | null {
  const command = event.metaKey || event.ctrlKey;
  const key = event.key.toLowerCase();
  if (command && key === 'z') return { kind: event.shiftKey ? 'redo' : 'undo' };
  if (command && key === 'y') return { kind: 'redo' };
  if (event.key === 'ArrowUp' || event.key === 'ArrowDown') {
    const sign = event.key === 'ArrowUp' ? 1 : -1;
    // Up and down move along the key's scale; Alt moves by a semitone and Ctrl by an octave.
    if (command) return { kind: 'action', action: { kind: 'transpose', semitones: sign * 12 } };
    if (event.altKey) return { kind: 'action', action: { kind: 'transpose', semitones: sign } };
    return { kind: 'action', action: { kind: 'step', steps: sign } };
  }
  if (event.key === 'ArrowLeft') return { kind: 'move', delta: -1 };
  if (event.key === 'ArrowRight') return { kind: 'move', delta: 1 };
  if (command || event.altKey) return null;
  if (event.key === 'Delete' || event.key === 'Backspace') {
    return { kind: 'action', action: { kind: 'delete' } };
  }
  const step = STEPS.indexOf(key);
  if (step >= 0 && key.length === 1) {
    return { kind: 'action', action: { kind: 'enter', step, value, chord: event.shiftKey } };
  }
  // Digits by physical key, so Shift or a keyboard layout does not change them.
  const digit = /^Digit([0-9])$/.exec(event.code)?.[1] ?? (/^[0-9]$/.test(key) ? key : null);
  if (digit === '0') return { kind: 'action', action: { kind: 'rest', value } };
  if (digit) {
    const next = valueForDigit(Number(digit));
    return next ? { kind: 'value', value: next } : null;
  }
  if (key === '.') return { kind: 'value', value: { log: value.log, dots: value.dots ? 0 : 1 } };
  if (key === 't') return { kind: 'action', action: { kind: 'toggle-tie' } };
  if (key === 's') return { kind: 'action', action: { kind: 'toggle-slur' } };
  return null;
}
