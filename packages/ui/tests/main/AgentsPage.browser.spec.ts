import { expect, test } from '@playwright/test';

import { mockManagement, releaseAgentChange } from './management.ts';

test('Agent changes can be queued independently', async ({ page }) => {
  await mockManagement(page);
  await page.goto('/main.html');
  const codex = page.getByRole('button', { name: '移除 Codex 插件' });
  const dsh = page.getByRole('button', { name: '安装 DeepSeek Harness 插件' });
  await codex.click();
  await expect(codex).toBeDisabled();
  await expect(dsh).toBeEnabled();
  await dsh.click();
  await expect(dsh).toBeDisabled();
  await expect(dsh).toHaveAttribute('aria-busy', 'true');
  await releaseAgentChange(page);
  await expect(page.getByRole('button', { name: '安装 Codex 插件' })).toBeEnabled();
  await expect(dsh).toBeDisabled();
  await releaseAgentChange(page);
  await expect(page.getByRole('button', { name: '移除 DeepSeek Harness 插件' })).toBeEnabled();
  expect(
    await page.evaluate(
      () =>
        (window as Window & { managementAgentChanges?: unknown[] }).managementAgentChanges?.length,
    ),
  ).toBe(2);
});

test('Pi icon keeps its shape in light and dark themes', async ({ page }) => {
  await mockManagement(page, { pi: true });
  await page.goto('/main.html');
  const icon = page.getByRole('button', { name: '移除 Pi' }).locator('.agent-chip-icon');
  const mask = icon.locator('.agent-monochrome-icon');
  await expect(mask).toHaveCSS('mask-image', /pi.*\.svg/);
  await expect(mask).not.toHaveCSS('mask-image', /data:/);
  for (const theme of ['light', 'dark']) {
    await page.evaluate(
      (value) => document.documentElement.setAttribute('data-theme', value),
      theme,
    );
    const screenshot = await icon.screenshot();
    const colors = await page.evaluate(async (base64) => {
      const image = new Image();
      image.src = `data:image/png;base64,${base64}`;
      await image.decode();
      const canvas = document.createElement('canvas');
      canvas.width = image.width;
      canvas.height = image.height;
      const context = canvas.getContext('2d')!;
      context.drawImage(image, 0, 0);
      const data = context.getImageData(0, 0, canvas.width, canvas.height).data;
      return new Set(
        Array.from(
          { length: data.length / 4 },
          (_, i) => `${data[i * 4]},${data[i * 4 + 1]},${data[i * 4 + 2]}`,
        ),
      ).size;
    }, screenshot.toString('base64'));
    expect(colors).toBeGreaterThan(2);
  }
});

test('Agent notices follow the selected language immediately', async ({ page }, testInfo) => {
  await mockManagement(page);
  await page.goto('/main.html');
  await page.getByRole('button', { name: '安装 DeepSeek Harness 插件' }).click();
  await releaseAgentChange(page);
  await expect(page.locator('.agent-detail')).toHaveCount(2);
  await page.getByRole('button', { name: '常规', exact: true }).click();
  await page.getByLabel('语言').selectOption('en-US');
  await page.getByRole('button', { name: 'Agent', exact: true }).click();
  await expect(page.locator('.agent-detail > .hint')).toHaveText([
    'This integration does not apply to existing Codex sessions. Start a new session or restart Codex.',
    'This integration does not apply to existing DSH sessions. Start a new session or restart DSH.',
  ]);
  await expect(page.locator('.agent-details')).not.toContainText(/[\u3400-\u9fff]/);
  await page.screenshot({ path: testInfo.outputPath('agents-english.png') });
  await page.getByRole('button', { name: 'General', exact: true }).click();
  await page.getByLabel('Language').selectOption('zh-CN');
  await page.getByRole('button', { name: 'Agent', exact: true }).click();
  await expect(page.locator('.agent-detail > .hint')).toHaveText([
    'Codex 已创建的会话不会生效，请新建会话或重启 Codex。',
    'DSH 已创建的会话不会生效，请新建会话或重启 DSH。',
  ]);
});

test('management Agent empty and initializing states', async ({ page }, testInfo) => {
  await mockManagement(page, { empty: true, initializing: true });
  await page.setViewportSize({ width: 1080, height: 720 });
  await page.goto('/main.html');
  await expect(page.locator('.agent-list')).toHaveText('正在检测可接入的 Agent…');
  await expect(page.locator('.agent-list')).toHaveAttribute('aria-busy', 'true');
  await expect(page.locator('.agent-list h2')).toHaveCount(0);
  await page.screenshot({ path: testInfo.outputPath('agents-initializing.png') });
  await page.evaluate(() =>
    (window as Window & { finishAgentDetection?: () => void }).finishAgentDetection?.(),
  );
  await expect(page.locator('.agent-list')).toHaveText(
    '未检测到可接入的 Agent，推荐安装 Claude、DeepSeek Harness、Codex 或 WorkBuddy',
  );
  await expect(page.locator('.agent-list')).toHaveAttribute('aria-busy', 'false');
  await expect(page.locator('.agent-list h2')).toHaveCount(0);
  await expect(page.locator('.agent-chip')).toHaveCount(0);
  await expect(page.locator('.agent-details')).toHaveCount(0);
  await page.screenshot({ path: testInfo.outputPath('agents-empty.png') });
});

test('missing DSH bundle can be reinstalled from its chip', async ({ page }) => {
  await mockManagement(page, { dshState: 'broken' });
  await page.goto('/main.html');
  const chip = page.getByRole('button', { name: '安装 DeepSeek Harness 插件' });
  await expect(chip.locator('.agent-chip-state')).toHaveText('需修复');
  await chip.click();
  await releaseAgentChange(page);
  await expect(page.getByRole('button', { name: '移除 DeepSeek Harness 插件' })).toBeEnabled();
  await expect(page.locator('.agent-detail')).toHaveCount(2);
  expect(
    await page.evaluate(
      () =>
        (window as Window & { managementCommands?: string[] }).managementCommands?.filter(
          (command) => command === 'management_agents_change',
        ).length,
    ),
  ).toBe(1);
});

test('conflicting DSH integration is overwritten when its chip is clicked', async ({ page }) => {
  await mockManagement(page, { dshState: 'conflict' });
  await page.goto('/main.html');
  const chip = page.getByRole('button', { name: '覆盖接入 DeepSeek Harness' });
  await expect(chip.locator('.agent-chip-state')).toHaveText('冲突');
  await chip.click();
  await releaseAgentChange(page);
  await expect(page.getByRole('button', { name: '移除 DeepSeek Harness 插件' })).toBeEnabled();
  expect(
    await page.evaluate(
      () =>
        (
          window as Window & {
            managementAgentChanges?: Array<{
              agentIds: string[];
              enabled: boolean;
              overwrite: boolean;
            }>;
          }
        ).managementAgentChanges?.[0],
    ),
  ).toMatchObject({ agentIds: ['dsh'], enabled: true, overwrite: true });
});

test('Agent paths hide Windows verbatim prefixes', async ({ page }) => {
  await mockManagement(page, {
    agentPaths: {
      codex:
        '\\\\?\\E:\\Projects\\ossia\\.ossia\\integrations\\hosts\\codex\\marketplace\\plugins\\ossia',
    },
  });
  await page.goto('/main.html');

  const detail = page.locator('.agent-detail').filter({ hasText: 'Codex' });
  await expect(detail.locator('code').nth(1)).toHaveText(
    'E:\\Projects\\ossia\\.ossia\\integrations\\hosts\\codex\\marketplace\\plugins\\ossia',
  );
  await expect(detail).not.toContainText('\\\\?\\');
});

test('multiple WorkBuddy installs render and toggle independently', async ({ page }) => {
  await mockManagement(page, {
    workbuddies: [
      { id: 'workbuddy:one', configRoot: '/Users/example/.workbuddy' },
      { id: 'workbuddy:two', configRoot: '/Users/example/.workbuddy-ai' },
    ],
  });
  await page.goto('/main.html');
  const chips = page.getByRole('button', { name: '安装 WorkBuddy 插件' });
  await expect(chips).toHaveCount(2);
  await expect(chips.nth(0)).toHaveAttribute('title', /\.workbuddy/);
  await expect(chips.nth(1)).toHaveAttribute('title', /\.workbuddy-ai/);

  await chips.nth(0).click();
  await releaseAgentChange(page);
  await expect(page.locator('.agent-detail').filter({ hasText: 'WorkBuddy' })).toHaveCount(1);
  await expect(
    page.locator('.agent-detail').filter({ hasText: 'WorkBuddy' }).locator('code').first(),
  ).toHaveText('/Users/example/.workbuddy');
  await expect(page.getByRole('button', { name: '移除 WorkBuddy 插件' })).toHaveCount(1);
  await expect(page.locator('.agent-detail').filter({ hasText: 'WorkBuddy' })).toContainText(
    '首次使用前请在连接器中信任 Ossia MCP',
  );

  await page.getByRole('button', { name: '安装 WorkBuddy 插件' }).click();
  await releaseAgentChange(page);
  await expect(page.getByRole('button', { name: '移除 WorkBuddy 插件' })).toHaveCount(2);
  await page.getByRole('button', { name: '移除 WorkBuddy 插件' }).nth(0).click();
  await releaseAgentChange(page);
  await expect(page.getByRole('button', { name: '移除 WorkBuddy 插件' })).toHaveCount(1);
});

test('Claude Code shows its own integration details and can be toggled', async ({ page }) => {
  await mockManagement(page, { claude: true });
  await page.goto('/main.html');
  const remove = page.getByRole('button', { name: '移除 Claude Code 插件' });
  const detail = page.locator('.agent-detail').filter({ hasText: 'Claude Code' });
  await expect(remove).toHaveAttribute('aria-pressed', 'true');
  await expect(detail).toContainText('/claude/plugins/installed_plugins.json');
  await expect(detail).toContainText('2.1.123');
  await expect(page.getByText('Claude Desktop', { exact: true })).toHaveCount(0);

  await remove.click();
  await releaseAgentChange(page);
  const install = page.getByRole('button', { name: '安装 Claude Code 插件' });
  await expect(install).toHaveAttribute('aria-pressed', 'false');
  await install.click();
  await releaseAgentChange(page);
  await expect(remove).toHaveAttribute('aria-pressed', 'true');
});
