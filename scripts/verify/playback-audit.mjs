import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';

import { groupAt } from '../../packages/ui/src/score/utils/playbackTimeline.ts';
import { locatePlayhead } from '../../packages/ui/src/score/utils/playheadPosition.ts';

const EPSILON = 0.001;

/** Compare manually verified onset positions with the observed trace. */
export function compareExpected(trace, expected) {
  return expected.flatMap((item) => {
    const actual = trace.find((step) => Math.abs(step.time - item.time) <= EPSILON);
    const position = actual?.position;
    if (
      !position ||
      position.page !== item.page ||
      position.system !== item.system ||
      !position.elements.some((element) =>
        element.matches.some((match) =>
          match.spans.some((span) => span.byteStart === item.sourceByteStart),
        ),
      )
    ) {
      return [{ kind: 'expected-mismatch', expected: item, actual: actual ?? null }];
    }
    return [];
  });
}

/** Report observable mapping gaps and backwards movements for manual review. */
export function auditPlayback(playback, geometry) {
  if (playback.renderKey !== geometry.renderKey || playback.sourceHash !== geometry.sourceHash) {
    throw new Error('playback and geometry are from different renders');
  }
  const groups = playback.groups.filter((group) => group.startSeconds !== null);
  const onsets = [...new Set(groups.map((group) => group.startSeconds))].sort((a, b) => a - b);
  const issues = [];
  for (const group of groups) {
    const end = group.startSeconds + (group.durationSeconds ?? 0);
    if (
      Number.isFinite(playback.durationSeconds) &&
      (group.startSeconds >= playback.durationSeconds || end > playback.durationSeconds + EPSILON)
    )
      issues.push({
        kind: 'out-of-range-group',
        time: group.startSeconds,
        end,
        duration: playback.durationSeconds,
        elementIds: group.elementIds,
      });
  }
  const trace = [];
  const elements = new Map();
  const links = new Map();
  const tieIds = new Set();
  for (const page of geometry.pages) {
    for (const element of page.elements) {
      if (element.region.target.kind !== 'element') continue;
      const id = element.region.target.elementId;
      if (element.region.elementKind === 'Tie') tieIds.add(id);
      links.set(
        id,
        (element.linkedTargets ?? [])
          .filter((target) => target.kind === 'element')
          .map((target) => target.elementId),
      );
      const matches = elements.get(id) ?? [];
      matches.push({
        page: page.page,
        system: element.systemIndex,
        kind: element.region.elementKind,
        bounds: element.bounds,
        label: element.region.label,
        spans: element.region.spans,
      });
      elements.set(id, matches);
    }
  }
  let located = 0;
  let previous = null;
  for (const time of onsets) {
    const group = groupAt(playback, time);
    if (!group) continue;
    const position = locatePlayhead(geometry.pages, playback.systems, new Set(group.elementIds));
    if (!position) {
      issues.push({
        kind: 'unlocated',
        time,
        pitches: group.pitches,
        voice: group.voice,
        elements: group.elementIds.map((id) => ({ id, matches: elements.get(id) ?? [] })),
      });
      trace.push({
        time,
        position: null,
        held: previous,
        pitches: group.pitches,
        voice: group.voice,
      });
      continue;
    }
    located++;
    const current = {
      time,
      page: position.span.page,
      system: position.span.systemIndex,
      x: position.x,
      voice: group.voice,
      pitches: group.pitches,
      elements: group.elementIds.map((id) => ({ id, matches: elements.get(id) ?? [] })),
    };
    trace.push({ time, position: current, held: null });
    if (
      previous &&
      (current.page < previous.page ||
        (current.page === previous.page && current.system < previous.system) ||
        (current.page === previous.page &&
          current.system === previous.system &&
          current.x < previous.x - 0.01))
    ) {
      issues.push({ kind: 'backward', from: previous, to: current });
    }
    previous = current;
  }
  for (const note of playback.notes) {
    if (
      !groups.some(
        (group) =>
          Math.abs(group.startSeconds - note.startSeconds) <= EPSILON &&
          group.pitches.includes(note.pitch),
      )
    ) {
      issues.push({
        kind: 'unmapped-note',
        time: note.startSeconds,
        pitch: note.pitch,
        partId: note.partId,
      });
    }
  }
  for (const group of groups) {
    if (group.elementIds.length < 2) continue;
    const connected = new Set([group.elementIds[0]]);
    let size;
    do {
      size = connected.size;
      for (const id of connected) {
        const own = links.get(id) ?? [];
        for (const other of group.elementIds) {
          const theirs = links.get(other) ?? [];
          if (
            own.includes(other) &&
            theirs.includes(id) &&
            own.some((tie) => tieIds.has(tie) && theirs.includes(tie))
          )
            connected.add(other);
        }
      }
    } while (connected.size !== size);
    if (connected.size === group.elementIds.length) continue;
    const positions = group.elementIds.flatMap((id) =>
      (elements.get(id) ?? []).map((match) => ({ id, ...match })),
    );
    const sourcePositions = [];
    for (const position of positions) {
      const span = position.spans?.[0];
      const counterpart =
        span &&
        sourcePositions.find((existing) => {
          const other = existing.spans?.[0];
          return (
            other &&
            span.byteStart === other.byteStart &&
            span.byteEnd === other.byteEnd &&
            position.page === existing.page &&
            position.system === existing.system &&
            ((position.kind === 'NoteHead' && existing.kind === 'TabNoteHead') ||
              (position.kind === 'TabNoteHead' && existing.kind === 'NoteHead'))
          );
        });
      if (!counterpart) sourcePositions.push(position);
    }
    const distinct = new Set(
      sourcePositions.map(
        (position) =>
          `${position.page}:${position.system}:${(position.bounds.x0 + position.bounds.x1) / 2}`,
      ),
    );
    if (distinct.size > 1) {
      issues.push({
        kind: 'multiple-positions',
        time: group.startSeconds,
        pitches: group.pitches,
        voice: group.voice,
        elements: positions,
      });
    }
  }
  const soundingOnsets = [...new Set(playback.notes.map((note) => note.startSeconds))];
  const missingSoundingOnsets = soundingOnsets.filter(
    (time) =>
      !groups.some(
        (group) => Math.abs(group.startSeconds - time) <= EPSILON && group.pitches.length,
      ),
  ).length;
  const referenced = new Set(playback.groups.flatMap((group) => group.elementIds));
  let noteheads = 0;
  for (const page of geometry.pages) {
    for (const element of page.elements) {
      if (
        !['NoteHead', 'TabNoteHead'].includes(element.region.elementKind) ||
        element.region.target.kind !== 'element'
      )
        continue;
      noteheads++;
      if (!referenced.has(element.region.target.elementId)) {
        issues.push({
          kind: 'unmapped-head',
          page: page.page,
          label: element.region.label,
          elementId: element.region.target.elementId,
        });
      }
    }
  }
  const untimedGroups = playback.groups.filter((group) => group.startSeconds === null).length;
  return {
    sourceHash: playback.sourceHash,
    renderKey: playback.renderKey,
    onsets: onsets.length,
    located,
    soundingNotes: playback.notes.length,
    soundingOnsets: soundingOnsets.length,
    missingSoundingOnsets,
    noteheads,
    untimedGroups,
    issues,
    trace,
  };
}

async function get(base, path) {
  const response = await fetch(new URL(path, base));
  if (!response.ok) throw new Error(`${response.status} ${await response.text()}`);
  return response.json();
}

async function main() {
  const args = process.argv.slice(2);
  const expectPath = args.find((arg) => arg.startsWith('--expect='))?.slice(9);
  const expectations = expectPath ? JSON.parse(await readFile(expectPath, 'utf8')) : {};
  const base = new URL(
    args.find((arg) => arg.startsWith('--base='))?.slice(7) ?? 'http://127.0.0.1:3211/api/v1/',
  );
  if (!base.pathname.endsWith('/')) base.pathname += '/';
  const ids = args.filter((arg) => !arg.startsWith('--'));
  let documents = [];
  if (ids.length) {
    documents = ids.map((documentId) => ({ documentId }));
  } else {
    let offset = 0;
    do {
      const page = await get(base, `documents?limit=200&offset=${offset}`);
      documents.push(...page.items);
      offset = page.nextOffset;
    } while (offset !== null);
  }
  const reports = [];
  for (const document of documents) {
    try {
      const context = await get(
        base,
        `documents/${encodeURIComponent(document.documentId)}/context`,
      );
      if (!context.render) throw new Error('no render');
      const key = context.render.renderKey;
      const [geometry, playback] = await Promise.all([
        get(base, `renders/${key}/geometry`),
        get(base, `renders/${key}/playback`),
      ]);
      const audit = auditPlayback(playback, geometry);
      const expected = expectations[document.documentId];
      if (expected) {
        if (expected.sourceHash !== playback.sourceHash || !Array.isArray(expected.checkpoints)) {
          throw new Error('expectations do not match this source snapshot');
        }
        audit.issues.push(...compareExpected(audit.trace, expected.checkpoints));
      }
      reports.push({
        title: context.document.title,
        documentId: document.documentId,
        ...audit,
      });
    } catch (error) {
      reports.push({
        title: document.title,
        documentId: document.documentId,
        error: String(error),
      });
    }
  }
  for (const documentId of Object.keys(expectations)) {
    if (!documents.some((document) => document.documentId === documentId)) {
      reports.push({ documentId, error: 'expected score was not audited' });
    }
  }
  console.log(JSON.stringify(reports, null, 2));
  if (reports.some((report) => report.error || report.issues.length || report.untimedGroups))
    process.exitCode = 1;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}
