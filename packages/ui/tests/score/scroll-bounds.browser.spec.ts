import { expect, test } from '@playwright/test';

import { firstId, mockScoreRuntime } from './score-runtime.ts';

test('wheel scrolling stops at the top of the first page and the bottom of the last', async ({
  context,
  page,
}) => {
  await mockScoreRuntime(context);
  await page.setViewportSize({ width: 1200, height: 700 });
  await page.goto(`/scores?documentId=${firstId}`);
  const scroll = page.locator('.score-scroll');
  const pages = page.locator('.score-pages');
  await expect(page.locator('.page-host').first()).toBeVisible();
  const viewport = (await scroll.boundingBox())!;
  const start = (await pages.boundingBox())!;
  expect(start.height).toBeGreaterThan(viewport.height);
  expect(start.y - viewport.y).toBeCloseTo(32, 0);

  await scroll.dispatchEvent('wheel', { deltaY: -2000 });
  await page.waitForTimeout(50);
  expect((await pages.boundingBox())!.y).toBeCloseTo(start.y, 0);

  await scroll.dispatchEvent('wheel', { deltaY: 100_000 });
  await expect
    .poll(async () => {
      const end = (await pages.boundingBox())!;
      return Math.round(end.y + end.height - (viewport.y + viewport.height));
    })
    .toBe(-32);
  await scroll.dispatchEvent('wheel', { deltaY: 2000 });
  await page.waitForTimeout(50);
  const end = (await pages.boundingBox())!;
  expect(Math.round(end.y + end.height - (viewport.y + viewport.height))).toBe(-32);

  await scroll.dispatchEvent('wheel', { deltaY: -100_000 });
  await expect
    .poll(async () => Math.round((await pages.boundingBox())!.y))
    .toBe(Math.round(start.y));
});
