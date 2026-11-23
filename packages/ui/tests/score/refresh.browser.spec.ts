import { expect, test } from '@playwright/test';

import type { RenderGeometry } from '../../src/generated/protocol.ts';
import { firstId, mockScoreRuntime, newer, secondId } from './score-runtime.ts';

function deferred() {
  let release!: () => void;
  const promise = new Promise<void>((resolve) => {
    release = resolve;
  });
  return { promise, release };
}

function updatedGeometry(key = newer): RenderGeometry {
  return {
    renderKey: key,
    sourceHash: key,
    pages: [1, 2].map((page) => ({
      page,
      viewBox: { x0: 0, y0: 0, x1: 200, y1: 140 },
      capability: 'measure',
      reason: null,
      elements: [],
      regions: [],
    })),
  };
}

const updatedSvg =
  '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 140"><text x="10" y="20">Updated score</text></svg>';

test('loading indicator appears only after 200 milliseconds', async ({ context, page }) => {
  await mockScoreRuntime(context);
  await page.clock.install({ time: new Date('2026-11-17T08:00:00Z') });
  await page.clock.pauseAt(new Date('2026-11-17T08:00:00Z'));
  const gate = deferred();
  await context.route('**/api/v1/documents/*/context', async (route) => {
    await gate.promise;
    await route.fallback();
  });
  try {
    const requested = page.waitForRequest('**/api/v1/documents/*/context');
    await page.goto(`/scores?documentId=${firstId}`);
    await requested;
    await page.clock.runFor(199);
    await expect(page.locator('.score-loading-state')).toHaveCount(0);
    await page.clock.runFor(1);
    await expect(page.getByRole('progressbar')).toBeVisible();
  } finally {
    gate.release();
  }
});

test('fast initial loading does not insert a loading indicator', async ({ context, page }) => {
  await mockScoreRuntime(context);
  await page.addInitScript(() => {
    Reflect.set(window, 'loadingShown', false);
    new MutationObserver((records) => {
      if (
        records.some((record) =>
          [...record.addedNodes].some(
            (node) =>
              node instanceof Element &&
              (node.matches('.score-loading-state') || node.querySelector('.score-loading-state')),
          ),
        )
      )
        Reflect.set(window, 'loadingShown', true);
    }).observe(document, { subtree: true, childList: true });
  });
  await page.goto(`/scores?documentId=${firstId}`);
  await expect(page.locator('.score-image')).toHaveCount(2);
  expect(await page.evaluate(() => Reflect.get(window, 'loadingShown'))).toBe(false);
});

test('agent compilation keeps the current drawing when the new render is pending', async ({
  context,
  page,
}) => {
  const runtime = await mockScoreRuntime(context);
  await page.goto(`/scores?documentId=${firstId}`);
  await expect(page.locator('.score-image')).toHaveCount(2);
  await page
    .locator('.score-image')
    .first()
    .evaluate((node) => Reflect.set(window, 'originalSvg', node));
  runtime.states[0]!.lastSuccessfulRender = null;
  runtime.states[0]!.editState = 'active';
  runtime.change();
  await expect(page.locator('.score-view')).toHaveClass(/is-agent-editing/);
  await expect(page.locator('.score-image')).toHaveCount(2);
  expect(
    await page
      .locator('.score-image')
      .first()
      .evaluate((node) => node === Reflect.get(window, 'originalSvg')),
  ).toBe(true);
  await expect(page.locator('.score-loading-state')).toHaveCount(0);
  await expect(page.locator('.overlay.interactive')).toHaveCount(0);
});

test('a score without any previous render shows the empty state while editing', async ({
  context,
  page,
}) => {
  const runtime = await mockScoreRuntime(context);
  runtime.states[0]!.lastSuccessfulRender = null;
  runtime.states[0]!.editState = 'active';
  await page.goto(`/scores?documentId=${firstId}`);
  await expect(page.getByRole('heading', { name: '尚无可用谱面' })).toBeVisible();
  await expect(page.locator('.score-image')).toHaveCount(0);
  await expect(page.locator('.score-loading-state')).toHaveCount(0);
});

test('new geometry and SVG replace the current drawing in place without loading', async ({
  context,
  page,
}) => {
  const runtime = await mockScoreRuntime(context);
  const geometry = deferred(),
    drawing = deferred();
  await context.route(`**/api/v1/renders/${newer}/geometry`, async (route) => {
    await geometry.promise;
    await route.fulfill({ json: updatedGeometry() });
  });
  await context.route(`**/api/v1/renders/${newer}/pages/*`, async (route) => {
    await drawing.promise;
    await route.fulfill({ contentType: 'image/svg+xml', body: updatedSvg });
  });
  await page.goto(`/scores?documentId=${firstId}`);
  await expect(page.locator('.score-image')).toHaveCount(2);
  const host = page.locator('.page-host').first();
  const height = (await host.boundingBox())!.height;
  await page.locator('.score-scroll').dispatchEvent('wheel', { deltaY: 100 });
  const top = (await host.boundingBox())!.y;
  await page.locator('.score-canvas').evaluate((node) => {
    Reflect.set(window, 'originalCanvas', node);
    const gaps: string[] = [];
    Reflect.set(window, 'refreshGaps', gaps);
    Reflect.set(window, 'watchRefresh', true);
    const sample = () => {
      if (!Reflect.get(window, 'watchRefresh')) return;
      if (!document.querySelector('[data-page="1"] .score-image')) gaps.push('blank');
      if (document.querySelector('.score-loading-state')) gaps.push('loading');
      requestAnimationFrame(sample);
    };
    requestAnimationFrame(sample);
  });
  try {
    const requestedGeometry = page.waitForRequest(`**/api/v1/renders/${newer}/geometry`);
    runtime.states[0]!.lastSuccessfulRender = {
      ...runtime.states[0]!.lastSuccessfulRender!,
      sourceHash: newer,
    };
    runtime.change();
    await requestedGeometry;
    await expect(page.locator('.score-image').first()).toHaveAttribute('viewBox', '0 0 100 140');
    await expect(page.locator('.score-loading-state')).toHaveCount(0);
    const requestedSvg = page.waitForRequest(`**/api/v1/renders/${newer}/pages/1`);
    geometry.release();
    await requestedSvg;
    expect((await host.boundingBox())!.height).toBe(height);
    await expect(page.locator('.overlay.interactive')).toHaveCount(0);
    drawing.release();
    await expect(page.locator('.score-image').first()).toHaveAttribute('viewBox', '0 0 200 140');
    await expect.poll(async () => (await host.boundingBox())!.height).toBe(height / 2);
    expect((await host.boundingBox())!.y).toBe(top);
    expect(
      await page
        .locator('.score-canvas')
        .evaluate((node) => node === Reflect.get(window, 'originalCanvas')),
    ).toBe(true);
    expect(await page.evaluate(() => Reflect.get(window, 'refreshGaps'))).toEqual([]);
  } finally {
    geometry.release();
    drawing.release();
    await page.evaluate(() => Reflect.set(window, 'watchRefresh', false));
  }
});

for (const failure of ['geometry', 'parse']) {
  test(`failed ${failure} keeps the last drawing and reports the error`, async ({
    context,
    page,
  }) => {
    const runtime = await mockScoreRuntime(context);
    await context.route(`**/api/v1/renders/${newer}/geometry`, (route) =>
      failure === 'geometry'
        ? route.fulfill({
            status: 500,
            json: { error: { code: 'test-error', message: 'Refresh failed' } },
          })
        : route.fulfill({ json: updatedGeometry() }),
    );
    await context.route(`**/api/v1/renders/${newer}/pages/*`, (route) =>
      route.fulfill({ contentType: 'image/svg+xml', body: '<broken' }),
    );
    await page.goto(`/scores?documentId=${firstId}`);
    await expect(page.locator('.score-image')).toHaveCount(2);
    runtime.states[0]!.lastSuccessfulRender = {
      ...runtime.states[0]!.lastSuccessfulRender!,
      sourceHash: newer,
    };
    runtime.change();
    await expect(page.locator('.score-view').getByRole('alert').first()).toBeVisible();
    await expect(page.locator('.score-image')).toHaveCount(2);
    await expect(page.locator('.score-image').first()).toHaveAttribute('viewBox', '0 0 100 140');
    await expect(page.locator('.score-loading-state')).toHaveCount(0);
  });
}

test('a newer render wins when an earlier SVG finishes late', async ({ context, page }) => {
  const runtime = await mockScoreRuntime(context);
  const late = deferred(),
    latest = 'c'.repeat(64);
  await context.route(`**/api/v1/renders/${newer}/geometry`, (route) =>
    route.fulfill({ json: updatedGeometry() }),
  );
  await context.route(`**/api/v1/renders/${latest}/geometry`, (route) =>
    route.fulfill({ json: updatedGeometry(latest) }),
  );
  await context.route(`**/api/v1/renders/${newer}/pages/*`, async (route) => {
    await late.promise;
    await route.fulfill({ contentType: 'image/svg+xml', body: updatedSvg });
  });
  await context.route(`**/api/v1/renders/${latest}/pages/*`, (route) =>
    route.fulfill({
      contentType: 'image/svg+xml',
      body: updatedSvg.replace('Updated score', 'Latest score'),
    }),
  );
  await page.goto(`/scores?documentId=${firstId}`);
  await expect(page.locator('.score-image')).toHaveCount(2);
  try {
    const requested = page.waitForRequest(`**/api/v1/renders/${newer}/pages/1`);
    runtime.states[0]!.lastSuccessfulRender = {
      ...runtime.states[0]!.lastSuccessfulRender!,
      sourceHash: newer,
    };
    runtime.change();
    await requested;
    runtime.states[0]!.sourceHash = latest;
    runtime.states[0]!.lastSuccessfulRender = {
      ...runtime.states[0]!.lastSuccessfulRender!,
      sourceHash: latest,
    };
    runtime.emit('document-changed');
    await expect(page.locator('.score-image text')).toHaveText(['Latest score', 'Latest score']);
    late.release();
    await page.evaluate(
      () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve())),
    );
    await expect(page.locator('.score-image text')).toHaveText(['Latest score', 'Latest score']);
    await expect(page.locator('.score-loading-state')).toHaveCount(0);
  } finally {
    late.release();
  }
});

test('switching documents never retains another score as its preview', async ({
  context,
  page,
}) => {
  const runtime = await mockScoreRuntime(context);
  runtime.states[1]!.lastSuccessfulRender = null;
  runtime.states[1]!.editState = 'active';
  await page.goto(`/scores?documentId=${firstId}`);
  await expect(page.locator('.score-image')).toHaveCount(2);
  await page.evaluate((id) => {
    history.pushState(null, '', `/scores?documentId=${id}`);
    window.dispatchEvent(new PopStateEvent('popstate'));
  }, secondId);
  await expect(page.locator('.topbar-title')).toHaveText('另一份乐谱');
  await expect(page.locator('.score-image')).toHaveCount(0);
  await expect(page.getByRole('heading', { name: '尚无可用谱面' })).toBeVisible();
});
