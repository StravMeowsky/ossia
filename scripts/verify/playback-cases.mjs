import { readFileSync } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

import { auditPlayback } from './playback-audit.mjs';
import { fixtureRenderer, fixtures } from './render-fixture.mjs';

/** Run the selected playback cases and return their command exit status. */
export function runPlaybackCases(ids) {
  const cases = JSON.parse(readFileSync(path.join(fixtures, 'cases.json'), 'utf8'));
  if (ids.some((id) => !cases.some((entry) => entry.id === id))) throw new Error('unknown case');
  const selected = cases.filter((entry) => !ids.length || ids.includes(entry.id));
  if (!selected.length) throw new Error('no playback cases selected');
  const render = fixtureRenderer();
  let failed = false;
  for (const entry of selected) {
    const { source, playback, geometry, midiNoteCount } = render(entry);
    const report = auditPlayback(playback, geometry);
    const elements = geometry.pages.flatMap((page) =>
      page.elements.map((element) => ({ page: page.page, ...element })),
    );
    const failures = [];
    for (const check of entry.checks) {
      if (check.kind === 'forward') {
        const steps = report.trace.filter(
          (item) => item.time >= check.from - 0.001 && item.time <= check.to + 0.001,
        );
        const backwards = report.issues.some(
          (issue) =>
            issue.kind === 'backward' &&
            issue.to.time >= check.from - 0.001 &&
            issue.to.time <= check.to + 0.001,
        );
        if (!steps.length || steps.some((item) => !item.position) || backwards)
          failures.push(check);
        continue;
      }
      if (check.kind === 'rest') {
        const offset = source.indexOf(check.needle);
        if (offset < 0 || source.indexOf(check.needle, offset + 1) !== -1)
          throw new Error(`${entry.id}: needle must be unique`);
        const sourceByte = Buffer.byteLength(source.slice(0, offset + (check.offset ?? 0)));
        const ids = new Set(
          elements
            .filter(
              (element) =>
                ['Rest', 'MultiMeasureRest'].includes(element.region.elementKind) &&
                element.region.spans.some(
                  (span) => span.byteStart <= sourceByte && sourceByte < span.byteEnd,
                ),
            )
            .map((element) => element.region.target.elementId),
        );
        if (
          !playback.groups.some(
            (group) =>
              group.elementIds.some((id) => ids.has(id)) &&
              !group.pitches.length &&
              group.startSeconds !== null &&
              Math.abs(group.startSeconds - check.time) < 0.001 &&
              Math.abs(group.durationSeconds - check.duration) < 0.001,
          )
        )
          failures.push(check);
        continue;
      }
      if (!['mapped', 'playhead'].includes(check.kind))
        throw new Error(`${entry.id}: unsupported check ${check.kind}`);
      const start = source.indexOf(check.needle);
      if (start < 0 || source.indexOf(check.needle, start + 1) !== -1)
        throw new Error(`${entry.id}: needle must be unique`);
      const sourceByte = Buffer.byteLength(source.slice(0, start + (check.offset ?? 0)));
      const matching = elements.filter(
        (element) =>
          ['NoteHead', 'TabNoteHead'].includes(element.region.elementKind) &&
          element.region.spans.some(
            (span) => span.byteStart <= sourceByte && sourceByte < span.byteEnd,
          ),
      );
      const ids = new Set(
        matching
          .filter((element) => element.region.target.kind === 'element')
          .map((element) => element.region.target.elementId),
      );
      const note = playback.notes.some(
        (item) =>
          item.pitch === check.pitch &&
          Math.abs(item.startSeconds - check.time) < 0.001 &&
          (check.duration === undefined || Math.abs(item.durationSeconds - check.duration) < 0.001),
      );
      const group = playback.groups.find(
        (item) =>
          item.pitches.includes(check.pitch) &&
          item.startSeconds !== null &&
          Math.abs(item.startSeconds - check.time) < 0.001 &&
          item.elementIds.some((id) => ids.has(id)),
      );
      let valid = note && !!group && (!check.tied || group.elementIds.length >= 2);
      if (check.continuation) {
        const offset = source.indexOf(check.continuation);
        if (offset < 0 || source.indexOf(check.continuation, offset + 1) !== -1)
          throw new Error(`${entry.id}: continuation must be unique`);
        const byte = Buffer.byteLength(source.slice(0, offset + (check.continuationOffset ?? 0)));
        valid &&= elements.some(
          (element) =>
            element.region.target.kind === 'element' &&
            ['NoteHead', 'TabNoteHead'].includes(element.region.elementKind) &&
            group?.elementIds.includes(element.region.target.elementId) &&
            element.region.spans.some((span) => span.byteStart <= byte && byte < span.byteEnd),
        );
      }
      if (check.kind === 'playhead') {
        const trace = report.trace.find((item) => Math.abs(item.time - check.time) < 0.001);
        valid &&=
          !!trace?.position &&
          matching.some(
            (element) =>
              element.page === trace.position.page &&
              (element.systemIndex === trace.position.system ||
                (element.systemIndex === null &&
                  playback.systems.some(
                    (system) =>
                      system.page === element.page &&
                      system.systemIndex === trace.position.system &&
                      (element.bounds.y0 + element.bounds.y1) / 2 >= system.y0 &&
                      (element.bounds.y0 + element.bounds.y1) / 2 <= system.y1,
                  ))) &&
              trace.position.x >= element.bounds.x0 &&
              trace.position.x <= element.bounds.x1,
          );
      }
      if (!valid) failures.push({ ...check, sounding: note, mapped: !!group });
    }
    console.log(
      JSON.stringify({
        id: entry.id,
        origin: entry.origin,
        midiNoteCount,
        soundingNotes: playback.notes.length,
        checks: entry.checks.length,
        failures,
        untimedGroups: report.untimedGroups,
        issues: report.issues,
      }),
    );
    failed ||= failures.length > 0 || report.issues.length > 0 || report.untimedGroups > 0;
  }
  return Number(failed);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href)
  process.exitCode = runPlaybackCases(process.argv.slice(2));
