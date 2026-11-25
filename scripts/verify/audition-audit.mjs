import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';

import { hitElement } from '../../packages/ui/src/score/utils/hitElement.ts';
import {
  adjacentAuditionGroup,
  auditionMembers,
  auditionSounds,
  sameVoice,
} from '../../packages/ui/src/score/utils/playbackTimeline.ts';

/** Notated attacks are the audition reference, even when a symbol changes the performed notes. */
export function auditionAttacks(playback) {
  const ornaments = new Set(playback.notes.flatMap((note) => note.ornamentAnchor ?? []));
  const onsets = [
    ...new Set(playback.groups.filter((g) => g.pitches.length).map((g) => g.startSeconds)),
  ];
  return Object.fromEntries(
    onsets
      .filter((time) => time !== null)
      .map((time) => {
        const ordinary = playback.notes
          .filter((note) => !note.ornamentAnchor && Math.abs(note.startSeconds - time) <= 0.001)
          .map((note) => note.pitch);
        const marked = playback.groups.filter(
          (g) =>
            g.startSeconds !== null &&
            Math.abs(g.startSeconds - time) <= 0.001 &&
            g.elementIds.some((id) => ornaments.has(id)),
        );
        const unique = new Map();
        for (const group of marked) {
          for (const pitch of group.pitches) {
            const key = JSON.stringify([
              group.partId,
              pitch,
              group.startSeconds,
              group.durationSeconds,
            ]);
            unique.set(key, pitch);
          }
        }
        return [time, [...ordinary, ...unique.values()].sort((a, b) => a - b)];
      }),
  );
}

/** Check every visible notehead against the page's click, single-note and keyboard contracts. */
export function auditAudition(playback, geometry) {
  if (playback.renderKey !== geometry.renderKey || playback.sourceHash !== geometry.sourceHash)
    throw new Error('playback and geometry are from different renders');
  const issues = [];
  const voices = new Map();
  let noteheads = 0,
    independent = 0,
    silent = 0,
    clickable = 0,
    coincidentUnisons = 0;
  for (const page of geometry.pages) {
    for (const element of page.elements) {
      if (
        !['NoteHead', 'TabNoteHead'].includes(element.region.elementKind) ||
        element.region.target.kind !== 'element'
      )
        continue;
      noteheads++;
      const id = element.region.target.elementId;
      const detail = {
        page: page.page,
        elementId: id,
        label: element.region.label,
        staffIndex: element.region.staffIndex,
        spans: element.region.spans,
      };
      const points = [element.bounds, ...element.hitBounds].flatMap((b) =>
        [0.25, 0.5, 0.75].flatMap((x) =>
          [0.25, 0.5, 0.75].map((y) => [b.x0 + (b.x1 - b.x0) * x, b.y0 + (b.y1 - b.y0) * y]),
        ),
      );
      const group = playback.groups.find((group) => group.elementIds.includes(id));
      const hit = points
        .map(([x, y]) => hitElement(page, x, y))
        .find((hit) => {
          if (hit?.region.target.kind !== 'element') return false;
          if (hit.region.target.elementId === id) return true;
          const other = playback.groups.find((group) =>
            group.elementIds.includes(hit.region.target.elementId),
          );
          return (
            group?.pitches.length === 1 &&
            other?.pitches.length === 1 &&
            group.pitches[0] === other.pitches[0] &&
            group.startSeconds === other.startSeconds &&
            hit.bounds.x0 === element.bounds.x0 &&
            hit.bounds.x1 === element.bounds.x1 &&
            hit.bounds.y0 === element.bounds.y0 &&
            hit.bounds.y1 === element.bounds.y1
          );
        });
      if (hit) {
        clickable++;
        if (hit.region.target.elementId !== id) coincidentUnisons++;
      } else issues.push({ kind: 'unhittable-head', ...detail });
      const matches = playback.groups.filter((group) => group.elementIds.includes(id));
      if (!matches.length) {
        issues.push({ kind: 'unauditionable-head', ...detail });
        continue;
      }
      if (matches.length !== 1)
        issues.push({ kind: 'ambiguous-audition', ...detail, groups: matches });

      if (!group.pitches.length) {
        silent++;
      } else if (group.pitches.length !== 1) {
        issues.push({
          kind: 'non-single-note-audition',
          ...detail,
          pitches: group.pitches,
          time: group.startSeconds,
        });
      } else independent++;
      if (
        group.startSeconds === null ||
        (group.pitches.length > 0 &&
          !playback.notes.some(
            (note) =>
              (note.pitch === group.pitches[0] &&
                Math.abs(note.startSeconds - group.startSeconds) <= 0.001) ||
              (group.elementIds.includes(note.ornamentAnchor) &&
                note.partId === group.partId &&
                note.startSeconds >= group.startSeconds - 0.001 &&
                note.startSeconds <= group.startSeconds + (group.durationSeconds ?? 0) + 0.001),
          ))
      )
        issues.push({ kind: 'missing-audition-attack', ...detail });
      const voiceKey = JSON.stringify([group.staffIndex, group.voice]);
      if (!voices.has(voiceKey)) {
        const sequence = playback.groups.filter((candidate) => sameVoice(candidate, group));
        const reached = new Map();
        for (const direction of ['next', 'previous']) {
          const visited = new Set();
          let step = direction === 'next' ? sequence[0] : sequence.at(-1);
          while (step && !visited.has(step)) {
            visited.add(step);
            step = adjacentAuditionGroup(playback, step, direction, false);
          }
          reached.set(direction, visited);
        }
        voices.set(voiceKey, { sequence, reached });
      }
      const { sequence: voice, reached } = voices.get(voiceKey);
      const directions = ['next', 'previous'].filter(
        (direction) => !reached.get(direction).has(group),
      );
      if (directions.length) issues.push({ kind: 'keyboard-skipped-head', directions, ...detail });
      if (
        voice.some(
          (candidate) =>
            candidate !== group &&
            candidate.elementIds.length === group.elementIds.length &&
            candidate.elementIds.every((id, i) => id === group.elementIds[i]),
        )
      )
        issues.push({ kind: 'ambiguous-keyboard-step', ...detail });
      const single = auditionMembers(playback, group, false);
      const combined = auditionMembers(playback, group, true);
      if (single.length !== 1 || single[0] !== group || !combined.includes(group))
        issues.push({ kind: 'invalid-audition-modifier', ...detail });
    }
  }
  const expectedAttacks = auditionAttacks(playback);
  const onsets = new Set();
  for (const group of playback.groups) {
    if (group.startSeconds === null || !group.pitches.length || onsets.has(group.startSeconds))
      continue;
    onsets.add(group.startSeconds);
    const expected = expectedAttacks[group.startSeconds];
    const actual = auditionSounds(auditionMembers(playback, group, true))
      .flatMap((member) => member.pitches)
      .sort((a, b) => a - b);
    if (actual.join(',') !== expected.join(','))
      issues.push({
        kind: 'simultaneous-audition-mismatch',
        time: group.startSeconds,
        expected,
        actual,
      });
  }
  return {
    noteheads,
    independent,
    silent,
    clickable,
    coincidentUnisons,
    simultaneousOnsets: onsets.size,
    issues,
  };
}

async function main() {
  const args = process.argv.slice(2);
  const input = args.find((arg) => arg.startsWith('--input='))?.slice(8);
  if (input) {
    const { playback, geometry } = JSON.parse(await readFile(input, 'utf8'));
    const report = auditAudition(playback, geometry);
    console.log(JSON.stringify(report, null, 2));
    if (report.issues.length) process.exitCode = 1;
    return;
  }
  const base = new URL(
    args.find((arg) => arg.startsWith('--base='))?.slice(7) ?? 'http://127.0.0.1:3211/api/v1/',
  );
  if (!base.pathname.endsWith('/')) base.pathname += '/';
  const get = async (path) => {
    const response = await fetch(new URL(path, base));
    if (!response.ok) throw new Error(`${response.status} ${await response.text()}`);
    return response.json();
  };
  const ids = args.filter((arg) => !arg.startsWith('--'));
  const documents = ids.map((documentId) => ({ documentId }));
  if (!ids.length) {
    let offset = 0;
    do {
      const page = await get(`documents?limit=200&offset=${offset}`);
      documents.push(...page.items);
      offset = page.nextOffset;
    } while (offset !== null);
  }
  const reports = [];
  for (const document of documents) {
    try {
      const context = await get(`documents/${encodeURIComponent(document.documentId)}/context`);
      if (!context.render) throw new Error('no render');
      const [playback, geometry] = await Promise.all(
        ['playback', 'geometry'].map((kind) => get(`renders/${context.render.renderKey}/${kind}`)),
      );
      reports.push({
        documentId: document.documentId,
        title: context.document.title,
        ...auditAudition(playback, geometry),
      });
    } catch (error) {
      reports.push({ documentId: document.documentId, error: String(error) });
    }
  }
  console.log(JSON.stringify(reports, null, 2));
  if (reports.some((report) => report.error || report.issues.length)) process.exitCode = 1;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href)
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
