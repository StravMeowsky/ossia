import { setTimeout as delay } from 'node:timers/promises';
import { pathToFileURL } from 'node:url';

/** Wait for the latest main-branch Check for the exact release commit. */
export async function waitForCheck({
  repository,
  sha,
  token,
  fetch = globalThis.fetch,
  sleep = delay,
  now = Date.now,
  timeoutMs = 45 * 60_000,
  intervalMs = 30_000,
}) {
  if (!/^[\w.-]+\/[\w.-]+$/.test(repository) || !/^[a-f0-9]{40}$/.test(sha) || !token) {
    throw new Error('check gate requires a repository, full commit SHA and Actions token');
  }
  const query = new URLSearchParams({
    branch: 'main',
    event: 'push',
    head_sha: sha,
    per_page: '100',
  });
  const url = `https://api.github.com/repos/${repository}/actions/workflows/check.yml/runs?${query}`;
  const deadline = now() + timeoutMs;
  while (now() < deadline) {
    const response = await fetch(url, {
      headers: {
        Authorization: `Bearer ${token}`,
        Accept: 'application/vnd.github+json',
        'X-GitHub-Api-Version': '2022-11-28',
      },
      redirect: 'error',
      signal: AbortSignal.timeout(30_000),
    });
    if (!response.ok)
      throw new Error(`GitHub API returned ${response.status} while listing checks`);
    const { workflow_runs: runs } = await response.json();
    if (!Array.isArray(runs)) throw new Error('GitHub API returned invalid check runs');
    const latest = runs
      .filter(
        (run) =>
          run.head_sha === sha &&
          run.head_branch === 'main' &&
          run.event === 'push' &&
          run.head_repository?.full_name === repository,
      )
      .sort((left, right) => right.id - left.id)[0];
    if (latest?.status === 'completed') {
      if (latest.conclusion !== 'success') {
        throw new Error(`Check for ${sha} finished with ${latest.conclusion}`);
      }
      console.log(`Check ${latest.id} passed for ${sha}`);
      return latest.id;
    }
    console.log(latest ? `Waiting for Check ${latest.id} (${latest.status})` : 'Waiting for Check');
    await sleep(intervalMs);
  }
  throw new Error(`timed out waiting for Check for ${sha}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  waitForCheck({
    repository: process.env.GITHUB_REPOSITORY,
    sha: process.env.GITHUB_SHA,
    token: process.env.GH_TOKEN,
  }).catch((error) => {
    console.error(`release check: ${error.message}`);
    process.exitCode = 1;
  });
}
