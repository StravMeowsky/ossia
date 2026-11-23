import assert from 'node:assert/strict';

import { test } from 'vite-plus/test';

import { waitForCheck } from '../../../scripts/release/check-gate.mjs';

const sha = 'a'.repeat(40);
const repository = 'owner/repository';
const run = (id, status, conclusion = null) => ({
  id,
  status,
  conclusion,
  event: 'push',
  head_branch: 'main',
  head_sha: sha,
  head_repository: { full_name: repository },
});
const response = (runs) => ({ ok: true, json: async () => ({ workflow_runs: runs }) });

test('waits for the latest exact main Check to succeed', async () => {
  const batches = [
    [],
    [run(1, 'completed', 'success'), run(2, 'in_progress')],
    [run(2, 'completed', 'success')],
  ];
  let requests = 0;
  const id = await waitForCheck({
    repository,
    sha,
    token: 'test',
    fetch: async (url) => {
      const query = new URL(url).searchParams;
      assert.equal(query.get('head_sha'), sha);
      assert.equal(query.get('branch'), 'main');
      requests += 1;
      return response(batches.shift());
    },
    sleep: async () => {},
  });
  assert.equal(id, 2);
  assert.equal(requests, 3);
});

test('rejects the latest failed Check even if an older run passed', async () => {
  await assert.rejects(
    waitForCheck({
      repository,
      sha,
      token: 'test',
      fetch: async () => response([run(1, 'completed', 'success'), run(2, 'completed', 'failure')]),
    }),
    /finished with failure/,
  );
});

test('does not accept checks for another repository or branch', async () => {
  let clock = 0;
  await assert.rejects(
    waitForCheck({
      repository,
      sha,
      token: 'test',
      fetch: async () =>
        response([
          { ...run(1, 'completed', 'success'), head_branch: 'other' },
          { ...run(2, 'completed', 'success'), head_repository: { full_name: 'other/repository' } },
        ]),
      now: () => clock,
      sleep: async () => {
        clock += 1;
      },
      timeoutMs: 1,
    }),
    /timed out/,
  );
});
