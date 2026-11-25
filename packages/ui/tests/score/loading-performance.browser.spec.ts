import { expect, test } from '@playwright/test';
import type { Request } from '@playwright/test';

import type { GeometryElement } from '../../src/generated/protocol.ts';
import { firstId, hash, mockScoreRuntime, olderVersionId, secondId } from './score-runtime.ts';

test('score displays while status, history and another page remain pending', async ({
  context,
  page,
}) => {
  await mockScoreRuntime(context);
  let release!: () => void;
  const pending = new Promise<void>((resolve) => {
    release = resolve;
  });
  await context.route(
    /\/api\/v1\/(status|documents\/[^/]+\/versions\?|renders\/[^/]+\/pages\/2)/,
    async (route) => {
      await pending;
      await route.fallback();
    },
  );
  try {
    await page.goto(`/scores?documentId=${firstId}`);
    await expect(page.locator('.score-image').first()).toBeVisible({ timeout: 3000 });
  } finally {
    release();
  }
  await expect(page.locator('.score-image')).toHaveCount(2);
});

test('remembered version loads before the history response', async ({ context, page }) => {
  const runtime = await mockScoreRuntime(context);
  await page.addInitScript(
    ({ firstId, olderVersionId }) => {
      localStorage.setItem(`ossia:score-version:v1:${firstId}`, olderVersionId);
    },
    { firstId, olderVersionId },
  );
  let release!: () => void;
  const pending = new Promise<void>((resolve) => {
    release = resolve;
  });
  await context.route('**/api/v1/documents/*/versions?*', async (route) => {
    await pending;
    await route.fallback();
  });
  try {
    await page.goto(`/scores?documentId=${firstId}`);
    await expect(page.locator('.score-image').first()).toBeVisible({ timeout: 3000 });
    expect(runtime.versionRenders).toEqual([olderVersionId]);
  } finally {
    release();
  }
});

test('long scores mount and request only visible pages and neighbours', async ({
  context,
  page,
}) => {
  const runtime = await mockScoreRuntime(context, { pageCount: 30 });
  await page.goto(`/scores?documentId=${firstId}`);
  await expect(page.locator('.page-host')).toHaveCount(30);
  await expect(page.locator('.score-image').first()).toBeVisible();
  expect(await page.locator('.score-image').count()).toBeLessThanOrEqual(4);
  expect(runtime.pageRequests.length).toBeLessThanOrEqual(4);
  const initial = await page.locator('.page-host').nth(14).boundingBox();
  await page.locator('.score-scroll').dispatchEvent('wheel', { deltaY: 14 * 1284 });
  await expect(page.locator('.page-host').nth(14).locator('.score-image')).toBeVisible();
  await expect(page.locator('.page-host').first().locator('.score-image')).toHaveCount(0);
  expect(await page.locator('.score-image').count()).toBeLessThanOrEqual(4);
  const current = await page.locator('.page-host').nth(14).boundingBox();
  expect(current!.height).toBe(initial!.height);
  await page.locator('.score-scroll').dispatchEvent('wheel', { deltaY: -14 * 1284 });
  await expect(page.locator('.page-host').first().locator('.score-image')).toBeVisible();
  expect(runtime.pageRequests.filter((path) => path === `/renders/${hash}/pages/1`)).toHaveLength(
    1,
  );
});

test('one failed page does not hide successfully loaded pages', async ({ context, page }) => {
  await mockScoreRuntime(context);
  await context.route('**/api/v1/renders/*/pages/2', (route) =>
    route.fulfill({ status: 500, body: '{}' }),
  );
  await page.goto(`/scores?documentId=${firstId}`);
  await expect(page.locator('.score-image').first()).toBeVisible();
  await expect(page.getByRole('alert').first()).toBeVisible();
});

test('status and history failures leave the current score usable', async ({ context, page }) => {
  await mockScoreRuntime(context);
  await context.route(/\/api\/v1\/(status|documents\/[^/]+\/versions\?)/, (route) =>
    route.fulfill({ status: 500, contentType: 'application/json', body: '{}' }),
  );
  await page.goto(`/scores?documentId=${firstId}`);
  await expect(page.locator('.score-image').first()).toBeVisible();
  await expect(page.getByRole('button', { name: '播放', exact: true })).toBeEnabled();
  await expect(page.getByRole('button', { name: '打开标注' })).toBeEnabled();
});

for (const layout of ['vertical', 'horizontal']) {
  test(`restoring a distant ${layout} view loads that page before the first page`, async ({
    context,
    page,
  }) => {
    const runtime = await mockScoreRuntime(context, { pageCount: 30 });
    await page.addInitScript(
      ({ firstId, layout }) => {
        localStorage.setItem(
          `ossia:score-view:v2:${firstId}:draft`,
          JSON.stringify({
            layout,
            x: layout === 'horizontal' ? -14 * 924 : 0,
            y: layout === 'vertical' ? -14 * 1284 : 0,
          }),
        );
      },
      { firstId, layout },
    );
    await page.goto(`/scores?documentId=${firstId}`);
    await expect(page.locator('[data-page="15"] .score-image')).toBeVisible();
    expect(runtime.pageRequests).not.toContain(`/renders/${hash}/pages/1`);
    const expectedPages = await page.locator('.score-scroll').evaluate((root) => {
      const viewport = root.getBoundingClientRect();
      const hosts = [...root.querySelectorAll<HTMLElement>('.page-host')];
      const expected = new Set<number>();
      hosts.forEach((host, index) => {
        const bounds = host.getBoundingClientRect();
        if (
          bounds.left >= viewport.right ||
          bounds.right <= viewport.left ||
          bounds.top >= viewport.bottom ||
          bounds.bottom <= viewport.top
        )
          return;
        for (
          let neighbour = Math.max(0, index - 1);
          neighbour <= Math.min(hosts.length - 1, index + 1);
          neighbour++
        )
          expected.add(Number(hosts[neighbour]!.dataset.page));
      });
      return [...expected].sort((a, b) => a - b);
    });
    await expect
      .poll(() =>
        page
          .locator('.page-host:has(.score-image)')
          .evaluateAll((hosts) => hosts.map((host) => Number((host as HTMLElement).dataset.page))),
      )
      .toEqual(expectedPages);
  });
}

test('an old document response cannot overwrite a newer navigation', async ({ context, page }) => {
  await mockScoreRuntime(context);
  let release!: () => void;
  const pending = new Promise<void>((resolve) => {
    release = resolve;
  });
  await context.route(`**/api/v1/documents/${firstId}/context`, async (route) => {
    await pending;
    await route.fallback();
  });
  try {
    const requested = page.waitForRequest(`**/api/v1/documents/${firstId}/context`);
    await page.goto(`/scores?documentId=${firstId}`);
    await requested;
    await page.evaluate((id) => {
      history.pushState(null, '', `/scores?documentId=${id}`);
      window.dispatchEvent(new PopStateEvent('popstate'));
    }, secondId);
    await expect(page.locator('.topbar-title')).toHaveText('另一份乐谱');
    const oldResponse = page.waitForResponse(`**/api/v1/documents/${firstId}/context`);
    release();
    await oldResponse;
    await expect(page.locator('.score-image').first()).toBeVisible();
    await expect(page.locator('.topbar-title')).toHaveText('另一份乐谱');
  } finally {
    release();
  }
});

test('page requests stay bounded and discard queued pages after a distant pan', async ({
  context,
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 7000 });
  await mockScoreRuntime(context, { pageCount: 30 });
  let release!: () => void;
  const pending = new Promise<void>((resolve) => {
    release = resolve;
  });
  const requested: number[] = [];
  const active = new Set<Request>();
  let maximum = 0;
  page.on('requestfailed', (request) => active.delete(request));
  page.on('requestfinished', (request) => active.delete(request));
  await context.route('**/api/v1/renders/*/pages/*', async (route) => {
    const request = route.request();
    requested.push(Number(request.url().split('/').pop()));
    active.add(request);
    maximum = Math.max(maximum, active.size);
    try {
      await pending;
      await route.fallback();
    } finally {
      active.delete(request);
    }
  });
  try {
    await page.goto(`/scores?documentId=${firstId}`);
    await expect.poll(() => requested.length).toBe(4);
    await page.locator('.score-scroll').dispatchEvent('wheel', { deltaY: 19 * 1284 });
    await page.evaluate(
      () =>
        new Promise<void>((resolve) =>
          requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
        ),
    );
    await expect.poll(() => requested.some((page) => page >= 19)).toBe(true);
    release();
    await expect(page.locator('[data-page="20"] .score-image')).toBeVisible();
    expect(maximum).toBeLessThanOrEqual(4);
    expect(requested).not.toContain(5);
    expect(requested).not.toContain(6);
    expect(requested).not.toContain(7);
  } finally {
    release();
  }
});

test('an annotation editor keeps its page mounted and preserves unsaved text', async ({
  context,
  page,
}) => {
  const runtime = await mockScoreRuntime(context, { pageCount: 30 });
  await page.goto(`/scores?documentId=${firstId}`);
  const first = page.locator('[data-page="1"]');
  await expect(first.locator('.overlay.interactive')).toBeVisible();
  await page.keyboard.press('e');
  await first.locator('.overlay').click({ position: { x: 360, y: 225 } });
  const input = page.getByRole('textbox', { name: '标注', exact: true });
  await input.fill('保留这段草稿');
  await page.locator('.score-scroll').dispatchEvent('wheel', { deltaY: 14 * 1284 });
  await expect(page.locator('[data-page="15"] .score-image')).toBeVisible();
  await expect(first.locator('.score-image')).toHaveCount(1);
  await expect(input).toHaveValue('保留这段草稿');
  await expect(input).toBeFocused();
  await page.getByRole('button', { name: '提交标注' }).click();
  await expect
    .poll(() => runtime.selections.get(firstId)?.annotations[0]?.text)
    .toBe('保留这段草稿');
  await expect(first.locator('.score-image')).toHaveCount(0);
  await page.locator('.score-scroll').dispatchEvent('wheel', { deltaY: -14 * 1284 });
  await expect(first.locator('.score-annotation-pin')).toBeVisible();
});

test('large page text is evicted by its memory budget', async ({ context, page }) => {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 140"><!--${'x'.repeat(1024 * 1024)}--><path d="M10 20h70v10z"/></svg>`;
  const runtime = await mockScoreRuntime(context, { pageCount: 30, svg });
  await page.goto(`/scores?documentId=${firstId}`);
  await expect(page.locator('[data-page="1"] .score-image')).toBeVisible();
  for (let index = 1; index <= 18; index++) {
    await page.locator('.score-scroll').dispatchEvent('wheel', { deltaY: 1284 });
    await expect(page.locator(`[data-page="${index + 1}"] .score-image`)).toBeVisible();
  }
  await page.locator('.score-scroll').dispatchEvent('wheel', { deltaY: -18 * 1284 });
  await expect(page.locator('[data-page="1"] .score-image')).toBeVisible();
  expect(runtime.pageRequests.filter((path) => path.endsWith('/pages/1'))).toHaveLength(2);
});

test('playback continues offscreen without repainting the adjacent drawing', async ({
  context,
  page,
}) => {
  const elements: GeometryElement[] = [20, 60].map((x, index) => ({
    systemIndex: 0,
    bounds: { x0: x, x1: x + 3, y0: 24, y1: 26 },
    priority: 0,
    hitBounds: [],
    graphicIds: [],
    linkedTargets: [],
    region: {
      target: { kind: 'element', elementId: `note-${index}` },
      label: 'NoteHead',
      elementKind: 'NoteHead',
      spans: [],
      occurrences: [],
      insertionAnchor: null,
      staffIndex: 0,
      voice: null,
      reason: null,
    },
  }));
  await mockScoreRuntime(context, {
    pageCount: 30,
    eventsDelay: 60_000,
    elementsByPage: { 1: elements },
    playback: {
      renderKey: hash,
      sourceHash: hash,
      durationSeconds: 20,
      parts: [{ id: 'part', name: 'Piano', instrument: 'piano' }],
      notes: elements.map((_, index) => ({
        partId: 'part',
        instrument: 'piano' as const,
        pitch: 60,
        startSeconds: index,
        durationSeconds: index === 0 ? 1 : 19,
      })),
      groups: elements.map((_, index) => ({
        elementIds: [`note-${index}`],
        pitches: [60],
        staffIndex: 0,
        voice: null,
        startSeconds: index,
        durationSeconds: index === 0 ? 1 : 19,
        partId: 'part',
      })),
      systems: [
        {
          page: 1,
          systemIndex: 0,
          measureStart: 1,
          measureEnd: 1,
          startSeconds: 0,
          endSeconds: 20,
          x0: 10,
          x1: 80,
          y0: 18,
          y1: 32,
        },
      ],
    },
  });
  await page.goto(`/scores?documentId=${firstId}`);
  await expect(page.locator('[data-page="2"] .score-image')).toHaveCount(1);
  await page.evaluate(() => {
    const node = document.querySelector('[data-page="2"] .score-image')!;
    const mutations: MutationRecord[] = [];
    const observer = new MutationObserver((records) => mutations.push(...records));
    observer.observe(node, { subtree: true, childList: true, attributes: true });
    Object.assign(window, { neighbourMutations: mutations });
  });
  await page.getByRole('button', { name: '播放', exact: true }).click();
  await expect(page.getByRole('button', { name: '暂停', exact: true })).toBeVisible();
  const line = page.locator('[data-page="1"] .score-playhead');
  await expect(line).toHaveCount(1);
  const started = Number(await line.getAttribute('x1'));
  await expect
    .poll(async () => Number(await line.getAttribute('x1')))
    .toBeGreaterThan(started + 0.1);
  expect(await page.evaluate(() => Reflect.get(window, 'neighbourMutations').length)).toBe(0);
  await page.locator('.score-scroll').dispatchEvent('wheel', { deltaY: 14 * 1284 });
  await expect(page.locator('[data-page="1"] .score-image')).toHaveCount(0);
  await expect(page.getByRole('button', { name: '暂停', exact: true })).toBeVisible();
  await page.locator('.score-scroll').dispatchEvent('wheel', { deltaY: -14 * 1284 });
  await expect(page.locator('[data-page="1"] .score-image')).toBeVisible();
  await expect
    .poll(async () => Number(await line.getAttribute('x1')))
    .toBeGreaterThan(started + 0.1);
});

test('large SVG sanitization yields without the Scheduler API', async ({ context, page }) => {
  await page.addInitScript(() => Object.defineProperty(window, 'scheduler', { value: undefined }));
  await mockScoreRuntime(context, {
    pageCount: 1,
    svg: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 140">${'<g onclick="alert(1)"><path d="M10 20h1v1z"/></g>'.repeat(5000)}</svg>`,
  });
  await page.goto(`/scores?documentId=${firstId}`);
  await expect(page.locator('.score-image')).toBeVisible();
  await expect(page.locator('.score-image path')).toHaveCount(5000);
  await expect(page.locator('.score-image [onclick]')).toHaveCount(0);
});

test('a delayed status response cannot undo a newer language event', async ({ context, page }) => {
  await mockScoreRuntime(context, { language: 'zh-CN' });
  let releaseStatus!: () => void, releaseEvent!: () => void;
  const statusGate = new Promise<void>((resolve) => {
    releaseStatus = resolve;
  });
  const eventGate = new Promise<void>((resolve) => {
    releaseEvent = resolve;
  });
  await context.route('**/api/v1/status', async (route) => {
    await statusGate;
    await route.fallback();
  });
  await context.route('**/api/v1/events?*', async (route) => {
    await eventGate;
    await route.fulfill({
      contentType: 'text/event-stream',
      body: `event: runtime\ndata: ${JSON.stringify({
        instanceId: 'instance',
        seq: 2,
        type: 'ui-language-changed',
        payload: { language: 'en-US' },
      })}\n\n`,
    });
  });
  try {
    const status = page.waitForResponse('**/api/v1/status');
    await page.goto(`/scores?documentId=${firstId}`);
    await expect(page.locator('.score-image').first()).toBeVisible();
    releaseEvent();
    await expect(page.locator('html')).toHaveAttribute('lang', 'en-US');
    releaseStatus();
    await status;
    await page.evaluate(
      () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve())),
    );
    await expect(page.locator('html')).toHaveAttribute('lang', 'en-US');
  } finally {
    releaseStatus();
    releaseEvent();
  }
});

test('keyboard navigation reaches annotations in a buffered page', async ({ context, page }) => {
  const runtime = await mockScoreRuntime(context, { pageCount: 30 });
  runtime.selections.set(firstId, {
    documentId: firstId,
    sourceHash: hash,
    targets: [],
    annotations: [
      {
        id: 'keyboard-pin',
        text: '邻页标注',
        boxes: [],
        targets: [
          {
            kind: 'measure',
            locator: { staffIndex: 0, measureStart: 2, measureEnd: 2, voice: null },
          },
        ],
      },
    ],
  });
  await page.goto(`/scores?documentId=${firstId}`);
  const pin = page.locator('[data-page="2"] .score-annotation-pin');
  await expect(pin).toHaveCount(1);
  await page.getByRole('button', { name: '清除标注', exact: true }).focus();
  await page.keyboard.press('Tab');
  await expect(pin).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('textbox', { name: '标注', exact: true })).toHaveValue('邻页标注');
});
