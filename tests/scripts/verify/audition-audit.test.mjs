import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { promisify } from 'node:util';

import { test } from 'vite-plus/test';

import { auditAudition } from '../../../scripts/verify/audition-audit.mjs';

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
    ['unauditionable-head', 'non-single-note-audition', 'simultaneous-audition-mismatch'],
  );
});

test('detects keyboard steps that are skipped because two groups share an identity', () => {
  const { geometry, playback } = fixture();
  playback.groups.splice(1, 0, { ...playback.groups[0], startSeconds: 0.5 });
  const { issues } = auditAudition(playback, geometry);
  assert.ok(issues.some((i) => i.kind === 'ambiguous-keyboard-step' && i.elementId === 'a'));
  assert.ok(issues.some((i) => i.kind === 'ambiguous-audition' && i.elementId === 'a'));
  assert.ok(
    issues.some(
      (i) =>
        i.kind === 'keyboard-skipped-head' && i.elementId === 'b' && i.directions.includes('next'),
    ),
  );
});

test('coincident unisons have equivalent audio, but different pitches cannot mask an unclickable head', () => {
  const { geometry, playback } = fixture();
  geometry.pages[0].elements[1].bounds = { ...geometry.pages[0].elements[0].bounds };
  assert(auditAudition(playback, geometry).issues.some((i) => i.kind === 'unhittable-head'));
  playback.groups[1].pitches = [60];
  playback.groups[1].startSeconds = 0;
  playback.notes[1] = { pitch: 60, startSeconds: 0 };
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
  const { auditionSounds } =
    await import('../../../packages/ui/src/score/utils/playbackTimeline.ts');
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

test('compares simultaneous audition with the MIDI attack multiset', () => {
  const { geometry, playback } = fixture();
  playback.notes.push({ pitch: 60, startSeconds: 0 });
  const report = auditAudition(playback, geometry);
  assert.equal(report.simultaneousOnsets, 3);
  assert.deepEqual(report.issues, [
    {
      kind: 'simultaneous-audition-mismatch',
      time: 0,
      expected: [60, 60],
      actual: [60],
    },
  ]);
});

test('requires an attack at the mapped pitch and time for audible heads', () => {
  const { geometry, playback } = fixture();
  playback.notes[2].startSeconds = 2.1;
  const report = auditAudition(playback, geometry);
  assert.deepEqual(
    report.issues.map(({ kind }) => kind),
    ['missing-audition-attack', 'simultaneous-audition-mismatch'],
  );
  assert.equal(report.issues[0].elementId, 'c');
});

const run = (...args) =>
  promisify(execFile)(process.execPath, ['scripts/verify/audition-audit.mjs', ...args]);

test('CLI audits a captured render and fails with its report when an audition group is missing', async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), 'ossia-audition-input-'));
  try {
    const input = path.join(directory, 'render.json');
    const bundle = fixture();
    await writeFile(input, JSON.stringify(bundle));
    const { stdout } = await run(`--input=${input}`);
    assert.equal(JSON.parse(stdout).clickable, 3);
    bundle.playback.groups.pop();
    await writeFile(input, JSON.stringify(bundle));
    await assert.rejects(
      run(`--input=${input}`),
      (error) =>
        error.code === 1 && JSON.parse(error.stdout).issues[0].kind === 'unauditionable-head',
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('CLI paginates the library and returns per-score errors alongside healthy audits', async () => {
  const { geometry, playback } = fixture();
  const responses = new Map([
    ['/api/documents?limit=200&offset=0', { items: [{ documentId: 'one' }], nextOffset: 200 }],
    ['/api/documents?limit=200&offset=200', { items: [{ documentId: 'two' }], nextOffset: null }],
    ['/api/documents/one/context', { document: { title: 'First' }, render: { renderKey: 'r' } }],
    ['/api/documents/two/context', { document: { title: 'Second' }, render: null }],
    ['/api/renders/r/geometry', geometry],
    ['/api/renders/r/playback', playback],
  ]);
  const server = createServer((request, response) => {
    const data = responses.get(request.url);
    response.statusCode = data ? 200 : 404;
    response.end(JSON.stringify(data ?? {}));
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  try {
    await assert.rejects(run(`--base=http://127.0.0.1:${server.address().port}/api`), (error) => {
      assert.equal(error.code, 1);
      const [first, second] = JSON.parse(error.stdout);
      assert.equal(first.documentId, 'one');
      assert.deepEqual(first.issues, []);
      assert.deepEqual(second, { documentId: 'two', error: 'Error: no render' });
      return true;
    });
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
});
