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

test('horizontal panning stops at the page edges and a narrow score stays centered', async ({
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
  const fitted = (await pages.boundingBox())!;
  expect(fitted.width).toBeLessThan(viewport.width);
  await scroll.dispatchEvent('wheel', { deltaX: 600 });
  await page.waitForTimeout(50);
  expect((await pages.boundingBox())!.x).toBeCloseTo(fitted.x, 0);

  await scroll.hover();
  await page.keyboard.down('Control');
  for (let index = 0; index < 4; index++) {
    await page.mouse.wheel(0, -100);
    await page.waitForTimeout(10);
  }
  await page.keyboard.up('Control');
  await expect.poll(async () => (await pages.boundingBox())!.width).toBeGreaterThan(viewport.width);

  await scroll.dispatchEvent('wheel', { deltaX: 100_000 });
  await expect
    .poll(async () => {
      const box = (await pages.boundingBox())!;
      return Math.round(box.x + box.width - (viewport.x + viewport.width));
    })
    .toBe(-32);
  await scroll.dispatchEvent('wheel', { deltaX: -100_000 });
  await expect.poll(async () => Math.round((await pages.boundingBox())!.x - viewport.x)).toBe(32);
});
