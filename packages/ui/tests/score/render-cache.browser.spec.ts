import { expect, test } from '@playwright/test';

import { firstId, mockScoreRuntime } from './score-runtime.ts';

test('version renders and immutable render assets are reused when switching back', async ({
  context,
  page,
}) => {
  const runtime = await mockScoreRuntime(context);
  await page.goto(`/scores?documentId=${firstId}`);
  await expect(page.locator('.score-image')).toHaveCount(2);
  await expect(page.getByRole('button', { name: '播放', exact: true })).toBeEnabled();
  const initialGeometry = runtime.geometryRequests.length;
  const initialPages = runtime.pageRequests.length;
  const initialPlayback = runtime.playbackRequests.length;
  const versionButton = page.locator('.score-version-control').getByRole('button');

  await versionButton.click();
  await page.getByRole('menuitemradio', { name: /版本 1/ }).click();
  await expect(versionButton).toHaveText('版本 1');
  await expect(page.locator('.score-image')).toHaveCount(2);
  expect(runtime.versionRenders).toHaveLength(1);
  const firstVersionGeometry = runtime.geometryRequests.length;
  const firstVersionPages = runtime.pageRequests.length;
  const firstVersionPlayback = runtime.playbackRequests.length;
  expect(firstVersionGeometry).toBe(initialGeometry);
  expect(firstVersionPages).toBe(initialPages);
  expect(firstVersionPlayback).toBe(initialPlayback);

  await versionButton.click();
  await page.getByRole('menuitemradio', { name: /版本 2/ }).click();
  await expect(versionButton).toHaveText('版本 2（当前）');
  await versionButton.click();
  await page.getByRole('menuitemradio', { name: /版本 1/ }).click();
  await expect(versionButton).toHaveText('版本 1');
  await expect(page.locator('.score-image')).toHaveCount(2);
  expect(runtime.versionRenders).toHaveLength(1);
  expect(runtime.geometryRequests).toHaveLength(firstVersionGeometry);
  expect(runtime.pageRequests).toHaveLength(firstVersionPages);
  expect(runtime.playbackRequests).toHaveLength(firstVersionPlayback);
});
