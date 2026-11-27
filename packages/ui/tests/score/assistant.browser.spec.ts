import { expect, test } from '@playwright/test';
import type { BrowserContext, Page } from '@playwright/test';

import type { AssistantEvent, AssistantSendArgs } from '../../src/generated/protocol.ts';
import { firstId, mockScoreRuntime } from './score-runtime.ts';

type Call = { command: string; args?: { args?: AssistantSendArgs } };

/** Stand in for the desktop app's IPC: record commands and let the test emit assistant events. */
async function mockDesktop(context: BrowserContext) {
  await context.addInitScript(() => {
    const callbacks = new Map<number, (event: unknown) => void>();
    const listeners = new Map<string, number>();
    let next = 0;
    const own = window as Window & {
      __TAURI_INTERNALS__?: unknown;
      assistantCalls?: { command: string; args?: unknown }[];
      emitAssistant?: (payload: unknown) => void;
    };
    own.assistantCalls = [];
    own.emitAssistant = (payload) => {
      const handler = listeners.get('assistant-event');
      if (handler !== undefined)
        callbacks.get(handler)?.({ event: 'assistant-event', id: 1, payload });
    };
    own.__TAURI_INTERNALS__ = {
      transformCallback: (callback: (event: unknown) => void) => {
        next += 1;
        callbacks.set(next, callback);
        return next;
      },
      invoke: async (command: string, args?: { event?: string; handler?: number }) => {
        if (command === 'plugin:event|listen') {
          listeners.set(args!.event!, args!.handler!);
          return 1;
        }
        if (command === 'plugin:event|unlisten') return null;
        own.assistantCalls!.push({ command, args });
        return null;
      },
    };
  });
}

const calls = (page: Page) =>
  page.evaluate(() => (window as Window & { assistantCalls?: Call[] }).assistantCalls ?? []);
const sendCalls = async (page: Page) =>
  (await calls(page)).filter((call) => call.command === 'assistant_send');
async function emit(page: Page, event: AssistantEvent, runId?: string) {
  const id = runId ?? (await sendCalls(page)).at(-1)?.args?.args?.runId;
  await page.evaluate(
    (update) =>
      (window as Window & { emitAssistant?: (payload: unknown) => void }).emitAssistant?.(update),
    { runId: id, event },
  );
}

test('the score window assistant sends messages to Claude Code and continues the conversation', async ({
  context,
  page,
}) => {
  await mockScoreRuntime(context);
  await mockDesktop(context);
  await page.goto(`/scores?documentId=${firstId}&window=1`);
  const toggle = page.getByRole('button', { name: '打开助手' });
  await toggle.click();
  const panel = page.getByRole('complementary', { name: '助手' });
  await expect(panel).toBeVisible();
  const input = panel.getByRole('textbox', { name: '消息' });
  await expect(input).toBeFocused();

  await input.fill('移调到 D 大调');
  await input.press('Enter');
  await expect(panel.locator('.score-assistant-message.is-user')).toHaveText('移调到 D 大调');
  await expect(panel.getByText('思考中…')).toBeVisible();
  await expect(input).toHaveValue('');
  expect((await calls(page)).at(-1)).toEqual({
    command: 'assistant_send',
    args: {
      args: {
        provider: 'claude-code',
        runId: expect.any(String),
        documentId: firstId,
        title: '合成钢琴练习',
        message: '移调到 D 大调',
        session: null,
      },
    },
  });

  await panel.getByRole('button', { name: '停止' }).click();
  expect((await calls(page)).at(-1)?.command).toBe('assistant_stop');

  await emit(page, { kind: 'session', session: 's1' });
  await emit(page, { kind: 'tool', name: 'ToolSearch' });
  await expect(panel.getByText('思考中…')).toBeVisible();
  await emit(page, { kind: 'tool', name: 'ossia_read_source' });
  await expect(panel.getByText('正在读取乐谱…')).toBeVisible();
  await emit(page, { kind: 'tool', name: 'ossia_write_source' });
  await expect(panel.getByText('正在修改乐谱…')).toBeVisible();
  await expect(panel.getByText('正在读取乐谱…')).toHaveCount(0);
  await emit(page, { kind: 'text', text: '已移调到 D 大调。' });
  await emit(page, { kind: 'tool', name: 'ossia_commit' });
  await expect(panel.getByText('正在保存新版本…')).toBeVisible();
  await emit(page, { kind: 'done', session: 's1', error: null });
  const reply = panel.locator('.score-assistant-message.is-assistant');
  await expect(reply.locator('.score-assistant-text')).toHaveText('已移调到 D 大调。');
  await expect(reply.locator('.score-assistant-pending')).toHaveCount(0);

  await input.fill('Transpose it back');
  await input.press('Enter');
  await emit(page, { kind: 'tool', name: 'ossia_read_source' });
  await expect(panel.getByText('Reading the score…')).toBeVisible();
  await emit(page, { kind: 'done', session: 's1', error: null });
  await expect(panel.getByRole('button', { name: '发送' })).toBeVisible();

  await input.fill('再加一个结尾');
  await panel.getByRole('button', { name: '发送' }).click();
  expect((await calls(page)).at(-1)?.args?.args).toMatchObject({ session: 's1' });
  await emit(page, { kind: 'done', session: null, error: 'stopped' });
  await expect(panel.getByText('已停止。')).toBeVisible();
  await input.fill('再试一次');
  await input.press('Enter');
  await emit(page, { kind: 'text', text: 'Not logged in · Please run /login' });
  await emit(page, { kind: 'done', session: null, error: 'Not logged in · Please run /login' });
  await expect(panel.getByText('Not logged in · Please run /login')).toHaveCount(1);
  await expect(panel.locator('.score-assistant-error').last()).toHaveText(
    'Not logged in · Please run /login',
  );

  await page.reload();
  await expect(page.getByRole('complementary', { name: '助手' })).toBeVisible();
  await expect(page.locator('.score-assistant-message')).toHaveCount(8);
  await page.getByRole('button', { name: '新对话' }).click();
  await expect(page.locator('.score-assistant-message')).toHaveCount(0);
  await page.getByRole('button', { name: '关闭助手' }).first().click();
  await expect(page.getByRole('complementary', { name: '助手' })).toHaveCount(0);
});

test('switching providers preserves separate conversations and resumes the selected one', async ({
  context,
  page,
}) => {
  await mockScoreRuntime(context);
  await mockDesktop(context);
  await page.goto(`/scores?documentId=${firstId}&window=1`);
  await page.getByRole('button', { name: '打开助手' }).click();
  const panel = page.getByRole('complementary', { name: '助手' });
  const provider = panel.getByRole('group', { name: '选择助手' });
  const claude = provider.getByRole('button', { name: 'Claude Code' });
  const codex = provider.getByRole('button', { name: 'Codex' });
  const input = panel.getByRole('textbox', { name: '消息' });
  const ask = async (message: string) => {
    await input.fill(message);
    await input.press('Enter');
  };

  await expect(claude).toHaveAttribute('aria-pressed', 'true');
  await expect(codex).toHaveAttribute('aria-pressed', 'false');
  await expect(claude.locator('img')).toBeVisible();
  await expect(codex.locator('img')).toBeVisible();
  const codexBox = await codex.boundingBox();
  const claudeBox = await claude.boundingBox();
  expect(codexBox!.x + codexBox!.width).toBeLessThanOrEqual(claudeBox!.x);
  expect(codexBox!.y).toBe(claudeBox!.y);
  await ask('Claude message');
  await expect(claude).toBeDisabled();
  await expect(codex).toBeDisabled();
  const claudeRun = (await sendCalls(page)).at(-1)?.args?.args?.runId;
  await emit(page, { kind: 'session', session: 'claude-thread' });
  await emit(page, { kind: 'text', text: 'Claude reply' });
  await emit(page, { kind: 'done', session: null, error: null });

  await expect(codex).toBeEnabled();
  await codex.press('Space');
  await expect(codex).toHaveAttribute('aria-pressed', 'true');
  await expect(claude).toHaveAttribute('aria-pressed', 'false');
  await expect(panel.getByText('Claude reply')).toHaveCount(0);
  await ask('Codex message');
  expect((await sendCalls(page)).at(-1)?.args?.args).toMatchObject({
    provider: 'codex',
    session: null,
  });
  expect(claudeRun).toBeTruthy();
  await emit(page, { kind: 'text', text: 'Late Claude reply' }, claudeRun);
  await emit(page, { kind: 'session', session: 'codex-thread' });
  await emit(page, { kind: 'text', text: 'Codex reply' });
  await emit(page, { kind: 'done', session: null, error: null });
  await expect(panel.getByText('Codex reply')).toBeVisible();
  await expect(panel.getByText('Late Claude reply')).toHaveCount(0);

  await page.reload();
  await expect(codex).toHaveAttribute('aria-pressed', 'true');
  await expect(panel.getByText('Codex reply')).toBeVisible();
  await ask('Continue with Codex');
  expect((await sendCalls(page)).at(-1)?.args?.args).toMatchObject({
    provider: 'codex',
    session: 'codex-thread',
  });
  await emit(page, { kind: 'done', session: null, error: null });

  await claude.click();
  await expect(claude).toHaveAttribute('aria-pressed', 'true');
  await expect(panel.getByText('Claude reply')).toBeVisible();
  await expect(panel.getByText('Codex reply')).toHaveCount(0);
  await ask('Continue with Claude');
  expect((await sendCalls(page)).at(-1)?.args?.args).toMatchObject({
    provider: 'claude-code',
    session: 'claude-thread',
  });
  await emit(page, { kind: 'done', session: null, error: null });
});

test('the assistant is only offered in desktop score windows', async ({ context, page }) => {
  await mockScoreRuntime(context);
  await page.goto(`/scores?documentId=${firstId}&window=1`);
  await expect(page.getByRole('button', { name: /版本/ }).first()).toBeVisible();
  await expect(page.getByRole('button', { name: '打开助手' })).toHaveCount(0);
});

test('a new score window hands its composition brief to the assistant once', async ({
  context,
  page,
}) => {
  await mockScoreRuntime(context);
  await mockDesktop(context);
  const brief = '请在这份新建的空白乐谱里创作：一段温柔的旋律';
  await page.goto(`/scores?documentId=${firstId}&window=1&compose=${encodeURIComponent(brief)}`);
  const panel = page.getByRole('complementary', { name: '助手' });
  await expect(panel).toBeVisible();
  await expect(panel.locator('.score-assistant-message.is-user')).toHaveText(brief);
  expect((await sendCalls(page)).map((call) => call.args?.args?.message)).toEqual([brief]);
  expect(new URL(page.url()).searchParams.has('compose')).toBe(false);

  // Reloading the window does not send the brief again.
  await page.reload();
  await expect(panel).toBeVisible();
  await expect(panel.locator('.score-assistant-message.is-user')).toHaveText(brief);
  expect(await sendCalls(page)).toEqual([]);
});
