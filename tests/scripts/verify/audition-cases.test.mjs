import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, test, vi } from 'vite-plus/test';

import { auditAudition } from '../../../scripts/verify/audition-audit.mjs';
import { runAuditionCases } from '../../../scripts/verify/audition-cases.mjs';
import { auditPlayback } from '../../../scripts/verify/playback-audit.mjs';
import { fixtureRenderer, root } from '../../../scripts/verify/render-fixture.mjs';

vi.mock('node:child_process', () => ({ execFileSync: vi.fn() }));
vi.mock('node:fs', () => ({
  mkdtempSync: vi.fn(),
  readFileSync: vi.fn(),
  rmSync: vi.fn(),
  writeFileSync: vi.fn(),
}));
vi.mock('../../../scripts/verify/audition-audit.mjs', () => ({ auditAudition: vi.fn() }));
vi.mock('../../../scripts/verify/playback-audit.mjs', () => ({ auditPlayback: vi.fn() }));
vi.mock('../../../scripts/verify/render-fixture.mjs', () => ({
  root: process.cwd(),
  fixtures: '/fixtures',
  fixtureRenderer: vi.fn(),
}));

const directory = path.join(os.tmpdir(), 'fixed-audition-directory');
const bundle = { source: 'c4', playback: {}, geometry: {} };
const trace = [
  { time: 0, position: { x: 12, page: 1, system: 0 } },
  { time: 1, position: null },
];

beforeEach(() => {
  vi.resetAllMocks();
  vi.spyOn(console, 'log').mockImplementation(() => {});
  vi.spyOn(console, 'error').mockImplementation(() => {});
  readFileSync.mockReturnValue('[{"id":"one"},{"id":"two"}]');
  mkdtempSync.mockReturnValue(directory);
  fixtureRenderer.mockReturnValue(vi.fn(() => bundle));
  auditAudition.mockReturnValue({ issues: [] });
  auditPlayback.mockReturnValue({ issues: [], untimedGroups: 0, trace });
});
afterEach(() => vi.restoreAllMocks());

test('passes only selected fixture data and the flattened trace to Playwright', () => {
  assert.equal(runAuditionCases(['two']), 0);
  const input = path.join(directory, 'fixtures.json');
  assert.deepEqual(JSON.parse(writeFileSync.mock.calls[0][1]), [
    { id: 'two', ...bundle, trace: [{ time: 0, x: 12, page: 1, system: 0 }, { time: 1 }] },
  ]);
  const [command, args, options] = execFileSync.mock.calls[0];
  assert.equal(command, process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm');
  assert.deepEqual(args, [
    '--filter',
    '@ossia/ui',
    'exec',
    'playwright',
    'test',
    '--project=browser',
    'tests/score/audition-fixture.browser.spec.ts',
  ]);
  assert.equal(options.cwd, root);
  assert.equal(options.env.OSSIA_AUDITION_FIXTURES, input);
  assert.equal(options.shell, process.platform === 'win32');
  assert.deepEqual(rmSync.mock.calls, [[directory, { recursive: true }]]);
});

test.each([
  [
    'an audition issue',
    { issues: [{ kind: 'unauditionable-head' }] },
    { issues: [], untimedGroups: 0 },
  ],
  ['a playback issue', { issues: [] }, { issues: [{ kind: 'unlocated' }], untimedGroups: 0 }],
  ['an untimed group', { issues: [] }, { issues: [], untimedGroups: 1 }],
])('runs the data checks for every case and skips the browser for %s', (_, audition, playback) => {
  auditAudition.mockReturnValueOnce(audition);
  auditPlayback.mockReturnValueOnce({ ...playback, trace });
  assert.equal(runAuditionCases([]), 1);
  assert.equal(auditPlayback.mock.calls.length, 2);
  assert.equal(auditAudition.mock.calls.length, 2);
  assert.equal(execFileSync.mock.calls.length, 0);
  assert.deepEqual(rmSync.mock.calls, [[directory, { recursive: true }]]);
});

test('cleans up after browser failure and rejects unknown or empty selections before rendering', () => {
  assert.throws(() => runAuditionCases(['unknown']), /unknown case/);
  readFileSync.mockReturnValueOnce('[]');
  assert.throws(() => runAuditionCases([]), /no audition cases selected/);
  assert.equal(fixtureRenderer.mock.calls.length, 0);
  execFileSync.mockImplementationOnce(() => {
    throw new Error('browser failed');
  });
  assert.throws(() => runAuditionCases(['one']), /browser failed/);
  assert.deepEqual(rmSync.mock.calls, [[directory, { recursive: true }]]);
});
