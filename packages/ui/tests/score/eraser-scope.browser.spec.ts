import { expect, test } from '@playwright/test';

import { firstId, mockScoreRuntime } from './score-runtime.ts';

test('the eraser removes whole lines by default and its button switches to a sized partial eraser', async ({
  context,
  page,
}) => {
  const runtime = await mockScoreRuntime(context);
  await page.goto(`/scores?documentId=${firstId}`);
  await page.getByRole('button', { name: '画笔', exact: true }).click();
  const host = page.locator('.page-host').first();
  const box = (await host.boundingBox())!;
  for (const y of [0.25, 0.4]) {
    await page.mouse.move(box.x + box.width * 0.2, box.y + box.height * y);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width * 0.8, box.y + box.height * y, { steps: 20 });
    await page.mouse.up();
  }
  await expect.poll(() => runtime.drawings.get(firstId)?.strokes.length).toBe(2);

  const eraser = page.getByRole('button', { name: '橡皮擦', exact: true });
  await eraser.click();
  await expect(eraser).toHaveAttribute('aria-pressed', 'true');
  await expect(eraser).toHaveAttribute('data-eraser-scope', 'stroke');
  await expect(page.locator('.score-drawing-options')).toHaveCount(0);

  await page.mouse.move(box.x + box.width * 0.5, box.y + box.height * 0.22);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width * 0.5, box.y + box.height * 0.28, { steps: 5 });
  await page.mouse.up();
  await expect.poll(() => runtime.drawings.get(firstId)?.strokes.length).toBe(1);
  const [remaining] = runtime.drawings.get(firstId)!.strokes;
  expect(remaining!.points[0]!.x).toBeCloseTo(0.2, 1);
  expect(remaining!.points.at(-1)!.x).toBeCloseTo(0.8, 1);
  expect(remaining!.points[0]!.y).toBeCloseTo(0.4, 1);

  await eraser.click();
  await expect(eraser).toHaveAttribute('aria-pressed', 'true');
  await expect(eraser).toHaveAttribute('data-eraser-scope', 'partial');
  await expect(page.getByRole('group', { name: '橡皮擦大小' })).toBeVisible();
  await page.mouse.move(box.x + box.width * 0.5, box.y + box.height * 0.37);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width * 0.5, box.y + box.height * 0.43, { steps: 5 });
  await page.mouse.up();
  await expect.poll(() => runtime.drawings.get(firstId)?.strokes.length).toBe(2);

  await eraser.click();
  await expect(eraser).toHaveAttribute('data-eraser-scope', 'stroke');
  await expect(page.locator('.score-drawing-options')).toHaveCount(0);
  await eraser.click();
  await page.keyboard.press('Escape');
  await expect(eraser).toHaveAttribute('aria-pressed', 'false');

  await page.reload();
  await expect(eraser).toHaveAttribute('data-eraser-scope', 'partial');
  await page.keyboard.press('r');
  await expect(eraser).toHaveAttribute('aria-pressed', 'true');
  await expect(page.getByRole('group', { name: '橡皮擦大小' })).toBeVisible();
});
