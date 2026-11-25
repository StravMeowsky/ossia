import type { PlaybackNoteGroup, ScorePlayback } from '../../generated/protocol.ts';

const SIMULTANEOUS_ONSET_SECONDS = 0.001;

export function measureSpanAt(timeline: ScorePlayback, position: number) {
  if (!timeline.systems.length) return null;
  const first = timeline.systems[0]!;
  return (
    timeline.systems.find(
      (candidate) => position >= candidate.startSeconds && position < candidate.endSeconds,
    ) ??
    (position < first.startSeconds ? first : timeline.systems.at(-1)) ??
    null
  );
}

export function groupAt(timeline: ScorePlayback, position: number) {
  const previous = timeline.groups.filter(
    (group) => group.startSeconds !== null && group.startSeconds <= position + 0.001,
  );
  if (!previous.length) return null;
  const latest = Math.max(...previous.map((group) => group.startSeconds ?? 0));
  const current = previous.filter((group) => Math.abs((group.startSeconds ?? 0) - latest) <= 0.001);
  return current.find((group) => group.pitches.length > 0) ?? current[0] ?? null;
}

export function sameNoteGroup(left: PlaybackNoteGroup, right: PlaybackNoteGroup) {
  return (
    left.elementIds.length === right.elementIds.length &&
    left.elementIds.every((elementId, index) => elementId === right.elementIds[index])
  );
}

export function sameVoice(left: PlaybackNoteGroup, right: PlaybackNoteGroup) {
  return left.staffIndex === right.staffIndex && left.voice === right.voice;
}

export function sameOnset(left: PlaybackNoteGroup, right: PlaybackNoteGroup) {
  return (
    left.startSeconds !== null &&
    right.startSeconds !== null &&
    Math.abs(left.startSeconds - right.startSeconds) <= SIMULTANEOUS_ONSET_SECONDS
  );
}

export function auditionMembers(
  timeline: ScorePlayback | null,
  group: PlaybackNoteGroup,
  simultaneous: boolean,
) {
  if (!simultaneous || group.startSeconds === null || !group.pitches.length) return [group];
  return (
    timeline?.groups.filter(
      (candidate) => candidate.pitches.length > 0 && sameOnset(candidate, group),
    ) ?? [group]
  );
}

export function combinedAuditionGroup(members: PlaybackNoteGroup[], primary: PlaybackNoteGroup) {
  if (members.length === 1) return primary;
  return {
    elementIds: [...new Set(members.flatMap((member) => member.elementIds))],
    pitches: [...new Set(members.flatMap((member) => member.pitches))].sort((a, b) => a - b),
    staffIndex: primary.staffIndex,
    voice: primary.voice,
    startSeconds: primary.startSeconds,
    durationSeconds: Math.max(...members.map((member) => member.durationSeconds ?? 0)) || null,
    partId: primary.partId,
  };
}

export function auditionDuration(group: PlaybackNoteGroup) {
  return Math.min(0.9, Math.max(0.18, group.durationSeconds ?? 0.6));
}
