import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';

import { afterEach, beforeEach, test, vi } from 'vite-plus/test';

import { fixtureRenderer, root } from '../../../scripts/verify/render-fixture.mjs';
import { auditSelection } from '../../../scripts/verify/selection-audit.mjs';
import { runSelectionCases } from '../../../scripts/verify/selection-cases.mjs';

vi.mock('node:fs', () => ({ readFileSync: vi.fn() }));
vi.mock('../../../scripts/verify/selection-audit.mjs', () => ({ auditSelection: vi.fn() }));
vi.mock('../../../scripts/verify/render-fixture.mjs', () => ({
  root: process.cwd(),
  fixtures: '/fixtures',
  fixtureRenderer: vi.fn(),
}));

const click = (staffIndex, measureStart, measureEnd = measureStart) => ({
  staffIndex,
  target: { locator: { measureStart, measureEnd } },
});
const clicks = [click(0, 1), click(1, 1), click(0, 2), click(1, 2)];
const ids = ['a0000000-0000-0000-0000-000000000001', 'b0000000-0000-0000-0000-000000000002'];
const library = path.join(root, '.ossia');
const source = (id) => path.join(library, 'documents', id, 'score.ly');
const response = (data) => ({ ok: true, json: async () => data });

let log;
let error;
let render;
let request;
beforeEach(() => {
  vi.resetAllMocks();
  log = vi.spyOn(console, 'log').mockImplementation(() => {});
  error = vi.spyOn(console, 'error').mockImplementation(() => {});
  request = vi.fn();
  vi.stubGlobal('fetch', request);
  readFileSync.mockReturnValue(
    '[{"id":"one","selection":{"measures":2,"staves":2}},{"id":"other"}]',
  );
  render = vi.fn((entry) => entry);
  fixtureRenderer.mockReturnValue(render);
  auditSelection.mockImplementation(() => ({ cells: 4, clicks: [...clicks], issues: [] }));
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

test('verifies only the selected fixture and checks all staff positions independently', async () => {
  assert.equal(await runSelectionCases(['one']), 0);
  assert.deepEqual(fixtureRenderer.mock.calls, [[{ selection: true }]]);
  assert.deepEqual(
    render.mock.calls.map(([entry]) => entry.id),
    ['one'],
  );
  assert.deepEqual(JSON.parse(log.mock.calls[0][0]), {
    id: 'one',
    cells: 4,
    clickable: 4,
    issues: [],
  });
});

test('fails on an unexpected count or a wrong locator range despite successful hits', async () => {
  auditSelection.mockReturnValueOnce({ cells: 3, clicks: clicks.slice(0, 3), issues: [] });
  assert.equal(await runSelectionCases(['one']), 1);
  assert.ok(
    JSON.parse(log.mock.calls[0][0]).issues.some(({ kind }) => kind === 'unexpected-measure-count'),
  );

  const wrongRange = [...clicks.slice(0, 3), click(1, 2, 3)];
  auditSelection.mockReturnValueOnce({ cells: 4, clicks: wrongRange, issues: [] });
  assert.equal(await runSelectionCases(['one']), 1);
  assert.deepEqual(JSON.parse(log.mock.calls[1][0]).issues, [
    {
      kind: 'wrong-engraved-measure-order',
      staffIndex: 1,
    },
  ]);
});

test('returns failure for both audit and render errors while checking the remaining cases', async () => {
  readFileSync.mockReturnValueOnce('[{"id":"broken"},{"id":"issue"},{"id":"healthy"}]');
  render.mockImplementationOnce(() => {
    throw new Error('could not engrave');
  });
  auditSelection.mockReturnValueOnce({
    cells: 0,
    clicks: [],
    issues: [{ kind: 'empty-measure-grid' }],
  });
  assert.equal(await runSelectionCases([]), 1);
  assert.deepEqual(JSON.parse(error.mock.calls[0][0]), {
    id: 'broken',
    error: 'Error: could not engrave',
  });
  assert.deepEqual(
    log.mock.calls.map(([line]) => JSON.parse(line).id),
    ['issue', 'healthy'],
  );
});

test('checks explicitly requested library scores directly and normalizes UUID filenames', async () => {
  assert.equal(await runSelectionCases(['--library', ids[0].toUpperCase()]), 0);
  assert.deepEqual(render.mock.calls, [
    [
      {
        id: ids[0].toUpperCase(),
        title: undefined,
        source: source(ids[0]),
      },
    ],
  ]);
  assert.equal(request.mock.calls.length, 0);
  assert.equal(readFileSync.mock.calls.length, 0);
});

test('discovers every page of active library scores and retains their titles', async () => {
  request
    .mockResolvedValueOnce(response({ runtime: { home: library } }))
    .mockResolvedValueOnce(
      response({ items: [{ documentId: ids[0], title: 'First' }], nextOffset: 200 }),
    )
    .mockResolvedValueOnce(
      response({ items: [{ documentId: ids[1], title: 'Second' }], nextOffset: null }),
    );
  assert.equal(await runSelectionCases(['--library', '--base=http://localhost:4321/custom']), 0);
  assert.deepEqual(
    request.mock.calls.map(([url]) => url.href),
    [
      'http://localhost:4321/custom/status',
      'http://localhost:4321/custom/documents?limit=200&offset=0',
      'http://localhost:4321/custom/documents?limit=200&offset=200',
    ],
  );
  assert.deepEqual(
    render.mock.calls.map(([entry]) => entry.source),
    ids.map(source),
  );
  assert.deepEqual(
    log.mock.calls.map(([line]) => JSON.parse(line).title),
    ['First', 'Second'],
  );
});

test('rejects unknown fixtures, unsafe document IDs, and a mismatched or failed library before building', async () => {
  await assert.rejects(runSelectionCases(['unknown']), /select known fixture IDs/);
  await assert.rejects(runSelectionCases(['--library', '../outside']), /invalid document ID/);
  request.mockResolvedValueOnce(response({ runtime: { home: root } }));
  await assert.rejects(runSelectionCases(['--library']), /the server must be using/);
  request.mockResolvedValueOnce({ ok: false, status: 503 });
  await assert.rejects(runSelectionCases(['--library']), /HTTP 503/);
  request
    .mockResolvedValueOnce(response({ runtime: { home: library } }))
    .mockResolvedValueOnce(response({ items: [], nextOffset: null }));
  await assert.rejects(runSelectionCases(['--library']), /the project library is empty/);
  assert.equal(fixtureRenderer.mock.calls.length, 0);
});
