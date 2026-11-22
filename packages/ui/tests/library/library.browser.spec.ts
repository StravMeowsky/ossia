import { expect, test } from '@playwright/test';

const first = '00000000-0000-4000-8000-000000000001';
const second = '00000000-0000-4000-8000-000000000002';
const scores = [
  {
    documentId: first,
    title: 'Alpha',
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-02T00:00:00Z',
    archivedAt: null,
  },
  {
    documentId: second,
    title: 'Beta',
    createdAt: '2026-01-02T00:00:00Z',
    updatedAt: '2026-01-03T00:00:00Z',
    archivedAt: null,
  },
];

test('library browses paginated scores with previews, search, sorting and layout', async ({
  page,
}) => {
  await page.route('**/api/v1/library/scores?*', async (route) => {
    const offset = Number(new URL(route.request().url()).searchParams.get('offset'));
    await route.fulfill({
      json: { items: [scores[offset]], total: 2, offset, nextOffset: offset === 0 ? 1 : null },
    });
  });
  await page.route('**/api/v1/library/scores/*/preview', async (route) => {
    await route.fulfill({
      json: {
        state: 'ready',
        dataUrl: 'data:image/svg+xml,%3Csvg xmlns="http://www.w3.org/2000/svg"/%3E',
      },
    });
  });
  await page.goto('/');
  await expect(page.getByRole('heading', { name: '我的乐谱' })).toBeVisible();
  await expect(page.locator('.score-card-title')).toHaveText(['Beta', 'Alpha']);
  await expect(page.locator('.score-preview-image')).toHaveCount(2);
  await expect(page.getByRole('button', { name: '已归档' })).toHaveCount(0);
  await page.getByRole('button', { name: '列表' }).click();
  await page.getByRole('button', { name: '按创建时间排序' }).click();
  await page.getByRole('button', { name: '创建时间：从新到旧' }).click();
  await expect(page.locator('.score-card-title')).toHaveText(['Alpha', 'Beta']);
  await page.reload();
  await expect(page.locator('.score-collection')).toHaveClass(/is-list/);
  await expect(page.locator('.score-card-title')).toHaveText(['Alpha', 'Beta']);
  await page.getByPlaceholder('搜索乐谱').fill('beta');
  await expect(page.locator('.score-card-title')).toHaveText(['Beta']);
  await page.route('**/scores?*', (route) => route.fulfill({ body: 'score destination' }));
  await page.getByRole('button', { name: '打开 Beta' }).first().click();
  await expect(page).toHaveURL(new RegExp(`/scores\\?documentId=${second}`));
});

test('library shows errors and retries, including empty results', async ({ page }) => {
  let attempts = 0;
  await page.route('**/api/v1/library/scores?*', async (route) => {
    attempts += 1;
    if (attempts === 1) await route.fulfill({ status: 500 });
    else await route.fulfill({ json: { items: [], total: 0, offset: 0, nextOffset: null } });
  });
  await page.goto('/');
  await expect(page.getByText('无法读取乐谱列表')).toBeVisible();
  await page.getByRole('button', { name: '重试' }).click();
  await expect(page.getByText('还没有乐谱')).toBeVisible();
});

test('mobile library keeps controls and preview retry usable', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.route('**/api/v1/library/scores?*', (route) =>
    route.fulfill({ json: { items: scores, total: 2, offset: 0, nextOffset: null } }),
  );
  let requests = 0;
  await page.route('**/api/v1/library/scores/*/preview', async (route) => {
    requests += 1;
    await route.fulfill({
      json:
        requests <= 2
          ? { state: 'failed', error: { code: 'failed', message: 'failed' } }
          : { state: 'missing' },
    });
  });
  await page.goto('/');
  await expect(page.getByRole('heading', { name: '我的乐谱' })).toBeVisible();
  await expect(page.locator('.score-card')).toHaveCount(2);
  await expect(page.getByPlaceholder('搜索乐谱')).toBeVisible();
  await page
    .getByRole('button', { name: /预览生成失败.*重试/ })
    .first()
    .click();
  await expect.poll(() => requests).toBeGreaterThan(2);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});
