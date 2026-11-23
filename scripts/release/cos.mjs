import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { stat } from 'node:fs/promises';

import COS from 'cos-nodejs-sdk-v5';

export function objectKey(prefix, ...parts) {
  const key = [prefix, ...parts]
    .filter((part) => part !== undefined && part !== null && part !== '')
    .join('/')
    .replaceAll(/\/+/g, '/')
    .replace(/^\/+|\/+$/g, '');
  if (!key) throw new Error('object key is empty');
  return key;
}

export async function fileDigest(file) {
  const sha256 = createHash('sha256');
  for await (const chunk of createReadStream(file)) {
    sha256.update(chunk);
  }
  return {
    sha256: sha256.digest('hex'),
    size: (await stat(file)).size,
  };
}

export function cosClient({ secretId, secretKey, securityToken }) {
  return new COS({
    SecretId: secretId,
    SecretKey: secretKey,
    SecurityToken: securityToken,
    UseAccelerate: true,
  });
}

export async function putObject({
  client,
  bucket,
  region,
  key,
  file,
  body,
  contentType,
  cacheControl,
  acl,
}) {
  if (Boolean(file) === Boolean(body)) {
    throw new Error('putObject requires exactly one of file or body');
  }
  const common = {
    Bucket: bucket,
    Region: region,
    Key: key,
    ContentType: contentType,
    CacheControl: cacheControl,
    ACL: acl,
  };
  return file
    ? client.uploadFile({
        ...common,
        FilePath: file,
        SliceSize: 5 * 1024 * 1024,
      })
    : client.putObject({
        ...common,
        Body: body,
      });
}
