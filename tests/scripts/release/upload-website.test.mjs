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
    await writeFile(path.join(directory, 'assets', 'site-AbCd1234_-567890.js'), 'js');
    await writeFile(path.join(directory, 'assets', 'site.js'), 'fixed');
    const calls = [];
    await uploadWebsite({
      directory,
      prefix: 'releases',
      upload: async (item) => calls.push(item),
    });
    assert.deepEqual(
      calls.map(({ key, contentType, cacheControl }) => ({ key, contentType, cacheControl })),
      [
        {
          key: 'releases/assets/images/score.png',
          contentType: 'image/png',
          cacheControl: 'public, max-age=60',
        },
        {
          key: ['releases', 'assets', 'site-AbCd1234_-567890.js'].join('/'),
          contentType: 'text/javascript; charset=utf-8',
          cacheControl: 'public, max-age=31536000, immutable',
        },
        {
          key: 'releases/assets/site.js',
          contentType: 'text/javascript; charset=utf-8',
          cacheControl: 'public, max-age=60',
        },
        {
          key: 'releases/index.html',
          contentType: 'text/html; charset=utf-8',
          cacheControl: 'public, max-age=60',
        },
      ],
    );

    const failed = [];
    await assert.rejects(
      uploadWebsite({
        directory,
        upload: async (item) => {
          failed.push(item.key);
          throw new Error('upload failed');
        },
      }),
      /upload failed/,
    );
    assert.ok(!failed.includes('index.html'));

    await writeFile(path.join(directory, 'assets', 'unknown.xyz'), 'unknown');
    await assert.rejects(
      uploadWebsite({ directory, upload: async () => assert.fail() }),
      /unsupported website file type/,
    );
    await rm(path.join(directory, 'assets', 'unknown.xyz'));

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
