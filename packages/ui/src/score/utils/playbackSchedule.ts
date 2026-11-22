import type { PlaybackNote } from '../../generated/protocol.ts';

/** Pick the next contiguous run of notes inside the scheduler horizon. */
export function scheduleWindow(
  notes: readonly PlaybackNote[],
  index: number,
  position: number,
  horizon: number,
) {
  let next = index;
  while (next < notes.length && notes[next]!.startSeconds < position) next += 1;
  const start = next;
  while (next < notes.length && notes[next]!.startSeconds <= horizon) next += 1;
  return { notes: notes.slice(start, next), nextIndex: next };
}
