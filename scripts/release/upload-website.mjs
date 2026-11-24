import { readdir } from 'node:fs/promises';
import { dirname, extname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { cosClient, objectKey, putObject } from './cos.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const SHORT_CACHE = 'public, max-age=60';
const LONG_CACHE = 'public, max-age=31536000, immutable';
const CONTENT_TYPES = {
  '.css': 'text/css; charset=utf-8',
  '.gif': 'image/gif',
  '.html': 'text/html; charset=utf-8',
  '.ico': 'image/x-icon',
  '.jpeg': 'image/jpeg',
  '.jpg': 'image/jpeg',
  '.js': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.png': 'image/png',
  '.svg': 'image/svg+xml',
  '.webp': 'image/webp',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
};

/** Publish website assets before the index without touching release objects. */
export async function uploadWebsite({ directory, prefix = '', upload }) {
  const root = resolve(directory);
  const entries = await readdir(root, { withFileTypes: true });
  if (entries.some((entry) => !['index.html', 'assets'].includes(entry.name))) {
    throw new Error('website may contain only index.html and assets/ at its root');
  }
  if (!entries.find((entry) => entry.name === 'index.html')?.isFile()) {
    throw new Error('missing website index');
  }
  if (!entries.find((entry) => entry.name === 'assets')?.isDirectory()) {
    throw new Error('missing website assets directory');
  }
  const uploads = [];
  async function collect(dir, relative) {
    for (const entry of await readdir(dir, { withFileTypes: true })) {
      const name = relative ? `${relative}/${entry.name}` : entry.name;
      const file = join(dir, entry.name);
      if (entry.isDirectory()) await collect(file, name);
      else if (entry.isFile()) {
        const contentType = CONTENT_TYPES[extname(entry.name).toLowerCase()];
        if (!contentType) throw new Error(`unsupported website file type: ${name}`);
        const hashed = /^[^/]+-[\w-]{16}\.[a-z0-9]+$/i.test(name);
        uploads.push({
          file,
          key: objectKey(prefix, 'assets', name),
          contentType,
          cacheControl: hashed ? LONG_CACHE : SHORT_CACHE,
        });
      } else throw new Error(`unsupported website entry: ${name}`);
    }
  }
  await collect(join(root, 'assets'), '');
  uploads.push({
    file: join(root, 'index.html'),
    key: objectKey(prefix, 'index.html'),
    contentType: CONTENT_TYPES['.html'],
    cacheControl: SHORT_CACHE,
  });
  for (const item of uploads) await upload(item);
}

async function main() {
  const required = (name) => {
    const value = process.env[name]?.trim();
    if (!value) throw new Error(`${name} is required`);
    return value;
  };
  const bucket = required('OSSIA_RELEASES_BUCKET');
  const region = required('OSSIA_RELEASES_REGION');
  const client = cosClient({
    secretId: required('OSSIA_RELEASES_SECRET_ID'),
    secretKey: required('OSSIA_RELEASES_SECRET_KEY'),
    securityToken: process.env.OSSIA_RELEASES_SECURITY_TOKEN?.trim() || undefined,
  });
  await uploadWebsite({
    directory: join(ROOT, 'dist/website'),
    prefix: process.env.OSSIA_RELEASES_PREFIX?.trim() || '',
    upload: async (item) => {
      await putObject({
        client,
        bucket,
        region,
        ...item,
        acl: process.env.OSSIA_RELEASES_ACL?.trim() || 'public-read',
      });
      console.log(`Uploaded ${item.key}`);
    },
  });
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main().catch((error) => {
    console.error(`website upload: ${error.message}`);
    process.exitCode = 1;
  });
}
