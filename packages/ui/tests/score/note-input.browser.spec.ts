import { expect, test } from '@playwright/test';

import type { NoteInputArgs } from '../../src/generated/protocol.ts';
import { firstId, hash, mockScoreRuntime } from './score-runtime.ts';

const SOURCE = `\\version "2.26.0"\n\\score { \\new Staff { c'4 d' e' f' | g'1 | } \\layout { } }\n`;
const D = SOURCE.indexOf("d'");

function mei(second: { pname: string; accid?: string }) {
  const accid = second.accid ? ` accid="${second.accid}"` : '';
  return [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<mei xmlns="http://www.music-encoding.org/ns/mei" meiversion="5.0">',
    '<meiHead><fileDesc><titleStmt><title></title></titleStmt><pubStmt/></fileDesc></meiHead>',
    '<music><body><mdiv><score>',
    '<scoreDef keysig="0" meter.count="4" meter.unit="4"><staffGrp>',
    '<staffDef n="1" lines="5"><clef shape="G" line="2"/></staffDef></staffGrp></scoreDef>',
    '<section><measure n="1"><staff n="1"><layer n="1">',
    '<note xml:id="o40" pname="c" oct="4" dur="4"/>',
    `<note xml:id="o${D}" pname="${second.pname}" oct="4"${accid} dur="4"/>`,
    '<note xml:id="o47" pname="e" oct="4" dur="4"/>',
    '<note xml:id="o50" pname="f" oct="4" dur="4"/>',
    '</layer></staff></measure><measure n="2"><staff n="1"><layer n="1">',
    '<note xml:id="o55" pname="g" oct="4" dur="1"/>',
    '</layer></staff></measure></section></score></mdiv></body></music></mei>',
  ].join('');
}

test('note input edits through the quick preview and commits once with LilyPond', async ({
  context,
  page,
}) => {
  await mockScoreRuntime(context, { languageEvent: 'en-US' });
  const notes: NoteInputArgs[] = [];
  const calls: string[] = [];
  await context.route('**/api/v1/documents/*/notes/begin', (route) =>
    route.fulfill({
      contentType: 'application/json',
      body: JSON.stringify({
        editId: '55555555-5555-4555-8555-555555555555',
        documentId: firstId,
        baseVersionId: null,
        status: 'active',
        startedAt: 'now',
        author: 'user',
      }),
    }),
  );
  await context.route('**/api/v1/documents/*/edits', async (route) => {
    const body = route.request().postDataJSON();
    calls.push(`edit:${body.action}`);
    await route.fulfill({
      contentType: 'application/json',
      body: JSON.stringify(
        body.action === 'begin'
          ? {
              editId: '55555555-5555-4555-8555-555555555555',
              documentId: firstId,
              baseVersionId: null,
              status: 'active',
              startedAt: 'now',
              author: 'user',
            }
          : null,
      ),
    });
  });
  await context.route('**/api/v1/documents/*/notes', async (route) => {
    const args = route.request().postDataJSON() as NoteInputArgs;
    notes.push(args);
    const raised =
      args.action?.kind === 'transpose' ||
      args.action?.kind === 'step' ||
      (args.action?.kind === 'restore' && args.action.source.includes("dis'"));
    await route.fulfill({
      contentType: 'application/json',
      body: JSON.stringify({
        sourceHash: raised ? 'b'.repeat(64) : hash,
        source: raised ? SOURCE.replace("d'", "dis'") : SOURCE,
        cursor: args.target,
        mei: raised ? mei({ pname: 'd', accid: 's' }) : mei({ pname: 'd' }),
      }),
    });
  });
  for (const step of ['check', 'commit'] as const) {
    await context.route(`**/api/v1/documents/*/${step}`, async (route) => {
      calls.push(`${step}:${route.request().postDataJSON().expectedSourceHash.slice(0, 1)}`);
      await route.fulfill({
        contentType: 'application/json',
        body: JSON.stringify(
          step === 'check'
            ? {
                jobId: 'check-job',
                documentId: firstId,
                kind: 'check',
                state: 'done',
                sourceHash: 'b'.repeat(64),
                phase: 'finished',
                result: null,
                error: null,
              }
            : { versionId: 'v', seq: 3 },
        ),
      });
    });
  }

  await page.goto(`/scores?documentId=${firstId}`);
  await page.getByRole('button', { name: 'Enter notes' }).click();
  await expect(page.locator('.note-editor-page > svg')).toHaveCount(1, { timeout: 20_000 });
  expect(calls).toEqual([]);
  // The first note is selected; a click selects another.
  await expect(page.locator('.is-note-cursor')).toHaveAttribute('id', 'o40');
  // Clicking on the note, even where a staff line crosses it, selects that note.
  const box = (await page.locator(`[id="o${D}"]`).boundingBox())!;
  await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
  await expect(page.locator('.is-note-cursor')).toHaveAttribute('id', `o${D}`);

  // Up moves within the key; Alt+Up moves by a semitone.
  await page.keyboard.press('ArrowUp');
  await expect.poll(() => notes.at(-1)?.action).toEqual({ kind: 'step', steps: 1 });
  expect(notes.at(-1)?.target).toBe(D);
  expect(notes.at(-1)?.expectedSourceHash).toBe(hash);
  await page.keyboard.press('Alt+ArrowUp');
  await expect.poll(() => notes.at(-1)?.action).toEqual({ kind: 'transpose', semitones: 1 });
  await expect(page.locator(`[id="o${D}"] .accid`)).toHaveCount(1);

  // Undo puts the first source back.
  await page.keyboard.press('Control+z');
  await page.keyboard.press('Control+z');
  await expect.poll(() => notes.at(-1)?.action).toEqual({ kind: 'restore', source: SOURCE });
  await page.keyboard.press('Control+Shift+z');
  await expect.poll(() => notes.length).toBe(6);

  await page.getByRole('button', { name: 'Done' }).click();
  await expect(page.locator('.note-editor')).toHaveCount(0);
  expect(calls).toEqual(['check:b', 'commit:b']);
});

test('number keys pick the value new notes are written with', async ({ context, page }) => {
  await mockScoreRuntime(context, { languageEvent: 'en-US' });
  await context.route('**/api/v1/documents/*/notes/begin', (route) =>
    route.fulfill({
      contentType: 'application/json',
      body: JSON.stringify({
        editId: '55555555-5555-4555-8555-555555555555',
        documentId: firstId,
        baseVersionId: null,
        status: 'active',
        startedAt: 'now',
        author: 'user',
      }),
    }),
  );
  await context.route('**/api/v1/documents/*/notes', (route) =>
    route.fulfill({
      contentType: 'application/json',
      body: JSON.stringify({
        sourceHash: hash,
        source: SOURCE,
        cursor: 0,
        mei: mei({ pname: 'd' }),
      }),
    }),
  );
  await page.goto(`/scores?documentId=${firstId}`);
  await expect(page.getByRole('button', { name: 'Enter notes' })).toBeVisible();
  await page.keyboard.press('n');
  await expect(page.locator('.note-editor-page > svg')).toHaveCount(1, { timeout: 20_000 });
  await page.keyboard.press('4');
  await expect(page.getByRole('button', { name: 'Eighth note' })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  await page.keyboard.press('.');
  await expect(page.getByRole('button', { name: 'Dot' })).toHaveAttribute('aria-pressed', 'true');
});
