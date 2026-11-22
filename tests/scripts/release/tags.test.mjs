import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { test } from 'vite-plus/test';

import { createNextTag, highestTag, nextPatchTag } from '../../../scripts/release/tags.mjs';

test('the highest release tag uses the same ordering as version checks', () => {
  assert.equal(highestTag(['v0.1.0', 'v0.2.0-beta.1', 'v0.1.9']), 'v0.2.0-beta.1');
  assert.equal(highestTag(['not-a-release']), null);
});

test('automatic tags always allocate the next stable patch, even after a beta', () => {
  assert.equal(nextPatchTag(['v0.1.0-beta.1']), 'v0.1.1');
  assert.equal(nextPatchTag(['v0.1.0', 'v0.4.2']), 'v0.4.3');
});

test('automatic tagging only publishes the committed version', async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ossia-tags-'));
  const repo = path.join(root, 'repo');
  const remote = path.join(root, 'remote.git');
  const git = (...args) => execFileSync('git', ['-C', repo, ...args], { encoding: 'utf8' }).trim();
  const writeVersion = (version) => {
    fs.writeFileSync(
      path.join(repo, 'Cargo.toml'),
      `[workspace.package]\nversion = "${version}"\n`,
    );
    for (const file of [
      'package.json',
      'packages/desktop/package.json',
      'packages/desktop/src-tauri/tauri.conf.json',
    ]) {
      fs.mkdirSync(path.dirname(path.join(repo, file)), { recursive: true });
      fs.writeFileSync(path.join(repo, file), JSON.stringify({ version }));
    }
  };
  try {
    execFileSync('git', ['init', '--bare', '-q', remote]);
    execFileSync('git', ['init', '-q', '-b', 'main', repo]);
    git('config', 'user.name', 'Test');
    git('config', 'user.email', 'test@example.invalid');
    git('remote', 'add', 'origin', remote);
    writeVersion('0.1.0');
    git('add', '.');
    git('commit', '-qm', 'initial');
    git('tag', '-a', 'v0.1.0', '-m', 'initial');
    git('push', '-q', 'origin', 'main', '--tags');
    assert.deepEqual(await createNextTag(git('rev-parse', 'HEAD'), repo), { status: 'tagged' });

    fs.writeFileSync(path.join(repo, 'change'), 'change');
    git('add', '.');
    git('commit', '-qm', 'change');
    git('push', '-q', 'origin', 'main');
    assert.deepEqual(await createNextTag(git('rev-parse', 'HEAD'), repo), {
      status: 'version-mismatch',
    });
    assert.equal(git('tag', '--list', 'v0.1.1'), '');

    writeVersion('0.1.1');
    git('add', '.');
    git('commit', '-qm', 'version');
    git('push', '-q', 'origin', 'main');
    assert.deepEqual(await createNextTag(git('rev-parse', 'HEAD'), repo), {
      status: 'created',
      tag: 'v0.1.1',
    });
    assert.equal(git('ls-remote', '--tags', 'origin', 'v0.1.1').includes('refs/tags/v0.1.1'), true);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
