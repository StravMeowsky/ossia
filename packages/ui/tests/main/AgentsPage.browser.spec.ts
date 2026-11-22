import { expect, test } from '@playwright/test';

import { mockManagement, releaseAgentChange } from './management.ts';

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
    '未检测到可接入的 Agent，推荐安装 DeepSeek Harness、Codex 或 WorkBuddy',
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
