import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { createServer } from 'node:http';
import { promisify } from 'node:util';

import { test } from 'vite-plus/test';

import { auditPlayback, compareExpected } from '../../scripts/verify/playback-audit.mjs';

function fixture() {
  const span = { page: 1, systemIndex: 0 };
  const element = (id, x, systemIndex = 0) => ({
    region: { target: { kind: 'element', elementId: id } },
    bounds: { x0: x, x1: x },
    systemIndex,
  });
  const group = (time, pitch, ...elementIds) => ({
    startSeconds: time,
    pitches: [pitch],
    elementIds,
    voice: 'upper',
  });
  return {
    geometry: {
      renderKey: 'r',
      sourceHash: 's',
      pages: [{ page: 1, elements: [element('a', 10), element('b', 20), element('c', 30)] }],
    },
    playback: {
      renderKey: 'r',
      sourceHash: 's',
      systems: [span],
      notes: [],
      groups: [group(0, 60, 'a'), group(1, 62, 'b'), group(2, 64, 'c')],
    },
  };
}

test('reports an unlocatable onset and a sounding note without a matching group', () => {
  const { geometry, playback } = fixture();
  playback.groups[1].elementIds = ['missing'];
  playback.notes = [{ startSeconds: 1, pitch: 67, partId: 'p' }];
  const report = auditPlayback(playback, geometry);
  assert.equal(report.located, 2);
  assert.deepEqual(
    report.issues.map((issue) => issue.kind),
    ['unlocated', 'unmapped-note'],
  );
  assert.equal(report.trace[1].position, null);
  assert.equal(report.trace[1].held.x, 10);
});

test('locates a note in one system but holds when system bounds overlap', () => {
  const { geometry, playback } = fixture();
  const note = geometry.pages[0].elements[1];
  note.systemIndex = null;
  note.bounds.y0 = 15;
  note.bounds.y1 = 17;
  playback.systems = [
    { page: 1, systemIndex: 0, y0: 0, y1: 20 },
    { page: 1, systemIndex: 0, y0: 0, y1: 20 },
  ];
  assert.equal(auditPlayback(playback, geometry).trace[1].position.x, 20);
  playback.systems.push({ page: 1, systemIndex: 1, y0: 10, y1: 30 });
  const step = auditPlayback(playback, geometry).trace[1];
  assert.equal(step.position, null);
  assert.equal(step.held.x, 10);
});

test('flags backwards motion but does not infer whether it is a repeat', () => {
  const { geometry, playback } = fixture();
  playback.groups[1].elementIds = ['c'];
  playback.groups[2].elementIds = ['b'];
  const report = auditPlayback(playback, geometry);
  assert.deepEqual(
    report.issues.map((issue) => issue.kind),
    ['backward'],
  );
  assert.equal(report.issues[0].from.time, 1);
  assert.equal(report.issues[0].to.time, 2);
});

test('uses the earliest tied position even when element identifiers are reversed', () => {
  const { geometry, playback } = fixture();
  playback.groups[0].elementIds = ['c', 'a'];
  const report = auditPlayback(playback, geometry);
  assert.deepEqual(
    report.issues.map((issue) => issue.kind),
    ['multiple-positions'],
  );
  assert.deepEqual(
    report.trace.map((step) => step.position.x),
    [10, 20, 30],
  );
});

test('refuses to compare different render snapshots', () => {
  const { geometry, playback } = fixture();
  geometry.sourceHash = 'different';
  assert.throws(() => auditPlayback(playback, geometry), /different renders/);
});

test('reports a visible notehead absent from every playback group', () => {
  const { geometry, playback } = fixture();
  geometry.pages[0].elements[2].region.elementKind = 'NoteHead';
  playback.groups.pop();
  assert.deepEqual(
    auditPlayback(playback, geometry).issues.map((issue) => issue.kind),
    ['unmapped-head'],
  );
});

test('compares the observed trace against independently checked source positions', () => {
  const { geometry, playback } = fixture();
  geometry.pages[0].elements[0].region.spans = [{ byteStart: 42 }];
  const { trace } = auditPlayback(playback, geometry);
  assert.deepEqual(
    compareExpected(trace, [{ time: 0, page: 1, system: 0, sourceByteStart: 42 }]),
    [],
  );
  assert.equal(
    compareExpected(trace, [{ time: 0, page: 1, system: 0, sourceByteStart: 99 }])[0].kind,
    'expected-mismatch',
  );
});

test('reports a group spanning different notehead positions for review', () => {
  const { geometry, playback } = fixture();
  playback.groups[1].elementIds = ['b', 'c'];
  const report = auditPlayback(playback, geometry);
  assert.equal(
    report.issues.some((issue) => issue.kind === 'multiple-positions'),
    true,
  );
});

test('does not warn when different noteheads form a verified tie chain', () => {
  const { geometry, playback } = fixture();
  playback.groups[1].elementIds = ['b', 'c'];
  const target = (elementId) => ({ kind: 'element', elementId });
  geometry.pages[0].elements[1].linkedTargets = [target('c'), target('tie')];
  geometry.pages[0].elements[2].linkedTargets = [target('b'), target('tie')];
  geometry.pages[0].elements.push({ region: { target: target('tie'), elementKind: 'Tie' } });
  const report = auditPlayback(playback, geometry);
  assert.equal(
    report.issues.some((issue) => issue.kind === 'multiple-positions'),
    false,
  );
});

test('does not warn when the same source note is shown in notation and tablature', () => {
  const { geometry, playback } = fixture();
  playback.groups[1].elementIds = ['b', 'c'];
  geometry.pages[0].elements[1].region.elementKind = 'NoteHead';
  geometry.pages[0].elements[2].region.elementKind = 'TabNoteHead';
  geometry.pages[0].elements[1].region.spans = [{ byteStart: 42, byteEnd: 45 }];
  geometry.pages[0].elements[2].region.spans = [{ byteStart: 42, byteEnd: 45 }];
  assert.equal(
    auditPlayback(playback, geometry).issues.some((issue) => issue.kind === 'multiple-positions'),
    false,
  );
});

test('warns when the same source note is engraved twice at different positions', () => {
  const { geometry, playback } = fixture();
  playback.groups[1].elementIds = ['b', 'c'];
  geometry.pages[0].elements[1].region.elementKind = 'NoteHead';
  geometry.pages[0].elements[2].region.elementKind = 'NoteHead';
  geometry.pages[0].elements[1].region.spans = [{ byteStart: 42, byteEnd: 45 }];
  geometry.pages[0].elements[2].region.spans = [{ byteStart: 42, byteEnd: 45 }];
  assert.equal(
    auditPlayback(playback, geometry).issues.some((issue) => issue.kind === 'multiple-positions'),
    true,
  );
});

test('CLI fails on audit findings and untimed groups without an opt-in flag', async () => {
  const { geometry, playback } = fixture();
  const server = createServer((request, response) => {
    response.setHeader('Content-Type', 'application/json');
    response.end(
      JSON.stringify(
        request.url.endsWith('/context')
          ? { document: { title: 'Fixture' }, render: { renderKey: 'r' } }
          : request.url.endsWith('/geometry')
            ? geometry
            : playback,
      ),
    );
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  try {
    const base = `http://127.0.0.1:${server.address().port}/`;
    const run = () =>
      promisify(execFile)(process.execPath, [
        'scripts/verify/playback-audit.mjs',
        'fixture',
        `--base=${base}`,
      ]);
    await run();
    playback.groups[1].elementIds = ['c'];
    playback.groups[2].elementIds = ['b'];
    await assert.rejects(run(), (error) => error.code === 1 && error.stdout.includes('backward'));
    playback.groups = [];
    playback.groups.push({ startSeconds: null, pitches: [], elementIds: [] });
    await assert.rejects(
      run(),
      (error) => error.code === 1 && error.stdout.includes('"untimedGroups": 1'),
    );
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
});

test('rejects timed groups extending past the playback duration', () => {
  const { geometry, playback } = fixture();
  playback.durationSeconds = 2.5;
  playback.groups[2].durationSeconds = 1;
  const report = auditPlayback(playback, geometry);
  assert.equal(report.issues[0].kind, 'out-of-range-group');
  assert.equal(report.issues[0].end, 3);
});
