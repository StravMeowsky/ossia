import assert from 'node:assert/strict';
import { mkdtemp, mkdir, rm, symlink, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';

import { test } from 'vite-plus/test';

import { uploadWebsite } from '../../../scripts/release/upload-website.mjs';

test('uploads assets before the index without touching release objects', async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), 'ossia-website-'));
  try {
    await mkdir(path.join(directory, 'assets', 'images'), { recursive: true });
    await writeFile(path.join(directory, 'index.html'), '<html></html>');
    await writeFile(path.join(directory, 'assets', 'images', 'score.png'), 'image');
    const calls = [];
    await uploadWebsite({
      directory,
      prefix: 'releases',
      upload: async (item) => calls.push(item),
    });
    assert.deepEqual(
      calls.map(({ key, contentType }) => ({ key, contentType })),
      [
        { key: 'releases/assets/images/score.png', contentType: 'image/png' },
        { key: 'releases/index.html', contentType: 'text/html; charset=utf-8' },
      ],
    );

    await writeFile(path.join(directory, 'index.json'), '{}');
    await assert.rejects(uploadWebsite({ directory, upload: async () => assert.fail() }), /only/);
    await rm(path.join(directory, 'index.json'));
    await symlink(path.join(directory, 'index.html'), path.join(directory, 'assets', 'link.html'));
    await assert.rejects(
      uploadWebsite({ directory, upload: async () => assert.fail() }),
      /unsupported website entry/,
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
