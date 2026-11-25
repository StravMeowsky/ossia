import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import { afterEach, beforeEach, test, vi } from 'vite-plus/test';

import { auditPlayback } from '../../../scripts/verify/playback-audit.mjs';
import { runPlaybackCases } from '../../../scripts/verify/playback-cases.mjs';
import { fixtureRenderer } from '../../../scripts/verify/render-fixture.mjs';

vi.mock('node:fs', () => ({ readFileSync: vi.fn() }));
vi.mock('../../../scripts/verify/playback-audit.mjs', () => ({ auditPlayback: vi.fn() }));
vi.mock('../../../scripts/verify/render-fixture.mjs', () => ({
  fixtures: '/fixtures',
  fixtureRenderer: vi.fn(),
}));

function fixture() {
  const element = (elementId, elementKind, byteStart, x) => ({
    systemIndex: 0,
    bounds: { x0: x, x1: x + 2, y0: 10, y1: 12 },
    region: {
      target: { kind: 'element', elementId },
      elementKind,
      spans: [{ byteStart, byteEnd: byteStart + 2 }],
    },
  });
  return {
    source: '音 c4 ~ | c4 | r4',
    midiNoteCount: 1,
    playback: {
      notes: [{ pitch: 60, startSeconds: 0, durationSeconds: 2 }],
      groups: [
        { pitches: [60], startSeconds: 0, durationSeconds: 2, elementIds: ['attack', 'tied'] },
        { pitches: [], startSeconds: 2, durationSeconds: 1, elementIds: ['rest'] },
      ],
      systems: [{ page: 1, systemIndex: 0, y0: 9, y1: 13 }],
    },
    geometry: {
      pages: [
        {
          page: 1,
          elements: [
            element('attack', 'NoteHead', 4, 10),
            element('tied', 'TabNoteHead', 11, 20),
            element('rest', 'MultiMeasureRest', 16, 30),
          ],
        },
      ],
    },
  };
}

const mapped = {
  kind: 'mapped',
  needle: '音 c4 ~',
  offset: 2,
  time: 0,
  pitch: 60,
  tied: true,
  duration: 2,
  continuation: '| c4 |',
  continuationOffset: 2,
};
const playhead = { ...mapped, kind: 'playhead' };
const rest = { kind: 'rest', needle: 'r4', time: 2, duration: 1 };
const forward = { kind: 'forward', from: 0, to: 2 };

let bundle;
let observation;
let log;

function cases(...entries) {
  readFileSync.mockReturnValue(JSON.stringify(entries));
}

beforeEach(() => {
  vi.resetAllMocks();
  bundle = fixture();
  observation = {
    trace: [
      { time: 0, position: { page: 1, system: 0, x: 11 } },
      { time: 2, position: { page: 1, system: 0, x: 31 } },
    ],
    issues: [],
    untimedGroups: 0,
  };
  log = vi.spyOn(console, 'log').mockImplementation(() => {});
  fixtureRenderer.mockReturnValue(vi.fn(() => bundle));
  auditPlayback.mockReturnValue(observation);
});
afterEach(() => vi.restoreAllMocks());

test('checks UTF-8 source offsets, a tie continuation, a rest, and playhead movement', () => {
  cases(
    { id: 'other', checks: [] },
    { id: 'chosen', origin: 'fixture', checks: [mapped, playhead, rest, forward] },
  );
  bundle.geometry.pages[0].elements[0].systemIndex = null;
  assert.equal(runPlaybackCases(['chosen']), 0);
  assert.deepEqual(
    fixtureRenderer.mock.results[0].value.mock.calls.map(([entry]) => entry.id),
    ['chosen'],
  );
  assert.deepEqual(JSON.parse(log.mock.calls[0][0]), {
    id: 'chosen',
    origin: 'fixture',
    midiNoteCount: 1,
    soundingNotes: 1,
    checks: 4,
    failures: [],
    untimedGroups: 0,
    issues: [],
  });
});

test('fails individual checkpoints on an incorrect note, held cursor, rest, or backward movement', () => {
  cases({ id: 'changed', checks: [mapped, playhead, rest, forward] });
  bundle.playback.notes[0].durationSeconds = 1;
  bundle.playback.groups[1].durationSeconds = 0.5;
  observation.trace[0].position = null;
  observation.issues.push({ kind: 'backward', to: { time: 2 } });
  assert.equal(runPlaybackCases([]), 1);
  const report = JSON.parse(log.mock.calls[0][0]);
  assert.deepEqual(
    report.failures.map(({ kind }) => kind),
    ['mapped', 'playhead', 'rest', 'forward'],
  );
  assert.equal(report.failures[0].sounding, false);
  assert.equal(report.failures[0].mapped, true);
});

test('requires the tied continuation and the correct visual position independently', () => {
  cases({ id: 'wrong', checks: [mapped, playhead, forward] });
  bundle.playback.groups[0].elementIds = ['attack'];
  observation.trace[0].position = { page: 1, system: 0, x: 30 };
  assert.equal(runPlaybackCases([]), 1);
  assert.equal(JSON.parse(log.mock.calls[0][0]).failures.length, 2);
  bundle.playback.groups[0].elementIds.push('tied');
  assert.equal(runPlaybackCases([]), 1);
  assert.deepEqual(
    JSON.parse(log.mock.calls[1][0]).failures.map(({ kind }) => kind),
    ['playhead'],
  );
});

test('rejects ambiguous source markers and unsupported checks', () => {
  cases({ id: 'invalid', checks: [{ ...rest, needle: 'c4' }] });
  assert.throws(() => runPlaybackCases([]), /invalid: needle must be unique/);
  cases({ id: 'invalid', checks: [{ ...mapped, continuation: 'c4' }] });
  assert.throws(() => runPlaybackCases([]), /invalid: continuation must be unique/);
  cases({ id: 'invalid', checks: [{ kind: 'custom' }] });
  assert.throws(() => runPlaybackCases([]), /invalid: unsupported check custom/);
});

test('fails an audit without declared checkpoints and rejects unknown or empty case selections', () => {
  cases();
  assert.throws(() => runPlaybackCases([]), /no playback cases selected/);
  cases({ id: 'known', checks: [] });
  assert.throws(() => runPlaybackCases(['unknown']), /unknown case/);
  assert.equal(fixtureRenderer.mock.calls.length, 0);
  observation.untimedGroups = 1;
  assert.equal(runPlaybackCases([]), 1);
  observation.untimedGroups = 0;
  observation.issues = [{ kind: 'unlocated' }];
  assert.equal(runPlaybackCases([]), 1);
});
