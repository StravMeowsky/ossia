import { expect, test } from '@playwright/test';

import { firstId, mockScoreRuntime } from './score-runtime.ts';

test('a score opened in its own app window leaves its title to the window', async ({
  context,
  page,
}) => {
  await mockScoreRuntime(context);
  await page.goto(`/scores?documentId=${firstId}&window=1`);
  const controls = page.locator('.score-document-controls');
  await expect(controls.getByRole('button', { name: /^版本/ })).toBeVisible();
  await expect(page).toHaveTitle('合成钢琴练习');
  await expect(controls.locator('.topbar-title')).toHaveCount(0);
  await expect(page.getByRole('button', { name: /选择乐谱/ })).toHaveCount(0);
});
