import assert from 'node:assert/strict';

import { test } from 'vite-plus/test';

import { auditAudition } from '../../scripts/verify/audition-audit.mjs';

function fixture() {
  const elements = ['a', 'b', 'c'].map((id, i) => ({
    region: {
      target: { kind: 'element', elementId: id },
      elementKind: 'NoteHead',
      label: id,
      staffIndex: 0,
      spans: [],
    },
    bounds: { x0: i * 10, x1: i * 10 + 2, y0: 10, y1: 12 },
    hitBounds: [],
    priority: 0,
  }));
  return {
    geometry: { renderKey: 'r', sourceHash: 's', pages: [{ page: 1, elements }] },
    playback: {
      renderKey: 'r',
      sourceHash: 's',
      groups: elements.map((e, i) => ({
        elementIds: [e.region.target.elementId],
        pitches: [60 + i],
        startSeconds: i,
        voice: 'upper',
        staffIndex: 0,
      })),
      notes: elements.map((_, i) => ({ pitch: 60 + i, startSeconds: i })),
    },
  };
}

test('audits every head, including missing click groups and multi-pitch audition', () => {
  const { geometry, playback } = fixture();
  playback.groups.shift();
  playback.groups[1].pitches.push(67);
  const report = auditAudition(playback, geometry);
  assert.equal(report.noteheads, 3);
  assert.equal(report.independent, 1);
  assert.deepEqual(
    report.issues.map((i) => i.kind),
    ['unauditionable-head', 'non-single-note-audition'],
  );
});

test('detects keyboard steps that are skipped because two groups share an identity', () => {
  const { geometry, playback } = fixture();
  playback.groups.splice(1, 0, { ...playback.groups[0], startSeconds: 0.5 });
  assert(
    auditAudition(playback, geometry).issues.some((i) => i.kind === 'ambiguous-keyboard-step'),
  );
});

test('coincident unisons have equivalent audio, but different pitches cannot mask an unclickable head', () => {
  const { geometry, playback } = fixture();
  geometry.pages[0].elements[1].bounds = { ...geometry.pages[0].elements[0].bounds };
  assert(auditAudition(playback, geometry).issues.some((i) => i.kind === 'unhittable-head'));
  playback.groups[1].pitches = [60];
  playback.groups[1].startSeconds = 0;
  const report = auditAudition(playback, geometry);
  assert.equal(report.coincidentUnisons, 1);
  assert.equal(report.clickable, 3);
  assert.equal(report.issues.length, 0);
});

test('rejects mixed renders and accepts independently reachable single-note groups', () => {
  const { geometry, playback } = fixture();
  assert.equal(auditAudition(playback, geometry).issues.length, 0);
  playback.sourceHash = 'other';
  assert.throws(() => auditAudition(playback, geometry), /different renders/);
});

test('simultaneous audition deduplicates notation copies but preserves independent performers', async () => {
  const { auditionSounds } = await import('../../packages/ui/src/score/utils/playbackTimeline.ts');
  const group = {
    partId: 'guitar',
    pitches: [60],
    startSeconds: 0,
    durationSeconds: 1,
    elementIds: ['staff'],
  };
  const tab = { ...group, elementIds: ['tab'] };
  const violin = { ...group, partId: 'violin', elementIds: ['violin'] };
  assert.deepEqual(
    auditionSounds([group, tab, violin]).map((g) => g.partId),
    ['guitar', 'violin'],
  );
  assert.equal(
    auditionSounds([
      { ...group, partId: null },
      { ...tab, partId: null },
    ]).length,
    2,
  );
});

test('intentional silent notation remains clickable and keyboard reachable without a sounding attack', () => {
  const { geometry, playback } = fixture();
  playback.groups[1].pitches = [];
  playback.notes.splice(1, 1);
  const report = auditAudition(playback, geometry);
  assert.equal(report.silent, 1);
  assert.equal(report.independent, 2);
  assert.equal(report.clickable, 3);
  assert.equal(report.issues.length, 0);
});
