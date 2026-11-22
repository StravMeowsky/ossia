import { expect, test } from '@playwright/test';

import { firstId, mockScoreRuntime } from './score-runtime.ts';

test('layout and pan position persist per score and version', async ({ context, page }) => {
  await mockScoreRuntime(context);
  await page.goto(`/scores?documentId=${firstId}`);
  await expect(page.locator('.score-image')).toHaveCount(2);

  const scroll = page.locator('.score-scroll');
  const host = page.locator('.page-host').first();
  await page.locator('.score-view').hover();
  await page.getByRole('button', { name: '横向' }).click();
  await expect(page.locator('.score-pages.horizontal')).toHaveCount(1);
  await scroll.dispatchEvent('wheel', { deltaX: 240, deltaY: 0 });
  await page.waitForTimeout(250);
  const draftLeft = (await host.boundingBox())!.x;

  await page.locator('.score-version-control button').click();
  await page.getByRole('menuitemradio', { name: /版本 1/ }).click();
  await expect(page.locator('.score-pages.vertical')).toHaveCount(1);
  await scroll.dispatchEvent('wheel', { deltaX: 0, deltaY: 320 });
  await page.waitForTimeout(250);
  const versionTop = (await host.boundingBox())!.y;

  await page.reload();
  await expect(page.locator('.score-image')).toHaveCount(2);
  await expect(page.locator('.score-version-control')).toHaveAttribute(
    'data-viewing-version',
    'true',
  );
  await expect(page.locator('.score-version-control').getByRole('button')).toHaveText('版本 1');
  await expect(page.locator('.score-pages.vertical')).toHaveCount(1);
  await expect.poll(async () => (await host.boundingBox())!.y).toBeCloseTo(versionTop, 0);

  await page.locator('.score-version-control button').click();
  await page.getByRole('menuitemradio', { name: /版本 2/ }).click();
  await expect(page.locator('.score-pages.horizontal')).toHaveCount(1);
  await expect.poll(async () => (await host.boundingBox())!.x).toBeCloseTo(draftLeft, 0);

  await page.locator('.score-version-control button').click();
  await page.getByRole('menuitemradio', { name: /版本 1/ }).click();
  await expect(page.locator('.score-pages.vertical')).toHaveCount(1);
  await expect.poll(async () => (await host.boundingBox())!.y).toBeCloseTo(versionTop, 0);

  await page.locator('.score-version-control button').click();
  await page.getByRole('menuitemradio', { name: /版本 2/ }).click();
  await page.reload();
  await expect(page.locator('.score-image')).toHaveCount(2);
  await expect(page.locator('.score-version-control')).not.toHaveAttribute('data-viewing-version');
  await expect(page.locator('.score-version-control').getByRole('button')).toHaveText(
    '版本 2（当前）',
  );
});

test('zoom persists per version across reloads', async ({ context, page }) => {
  await mockScoreRuntime(context);
  await page.goto(`/scores?documentId=${firstId}`);
  await expect(page.locator('.score-image')).toHaveCount(2);

  const zoom = page.getByLabel('谱面缩放');
  await page.locator('.score-view').hover();
  await zoom.click();
  await page.getByRole('menuitemradio', { name: '150%', exact: true }).click();
  await expect(zoom).toHaveText('150%');
  await page.waitForTimeout(250);

  await page.reload();
  await expect(page.locator('.score-image')).toHaveCount(2);
  await expect(zoom).toHaveText('150%');

  await page.locator('.score-version-control button').click();
  await page.getByRole('menuitemradio', { name: /版本 1/ }).click();
  await expect(zoom).toHaveText('100%');
  await page.locator('.score-view').hover();
  await zoom.click();
  await page.getByRole('menuitemradio', { name: '75%', exact: true }).click();
  await expect(zoom).toHaveText('75%');
  await page.waitForTimeout(250);

  await page.reload();
  await expect(page.locator('.score-image')).toHaveCount(2);
  await expect(page.locator('.score-version-control').getByRole('button')).toHaveText('版本 1');
  await expect(zoom).toHaveText('75%');

  await page.locator('.score-version-control button').click();
  await page.getByRole('menuitemradio', { name: /版本 2/ }).click();
  await expect(zoom).toHaveText('150%');
});
