import { readFileSync } from 'node:fs';

import { expect, test } from '@playwright/test';

import type {
  GeometryElement,
  RenderGeometry,
  ScorePlayback,
} from '../../src/generated/protocol.ts';
import { hitElement } from '../../src/score/utils/hitElement.ts';
import { sameVoice } from '../../src/score/utils/playbackTimeline.ts';
import { firstId, hash, mockScoreRuntime } from './score-runtime.ts';

interface Fixture {
  id: string;
  geometry: RenderGeometry;
  playback: ScorePlayback;
  pages: [number, string][];
}
const input = process.env.OSSIA_AUDITION_FIXTURES;
const fixtures: Fixture[] = input ? JSON.parse(readFileSync(input, 'utf8')) : [];
if (!input) test.skip('engraved audition fixtures require pnpm verify:audition', () => {});

for (const fixture of fixtures) {
  test(`${fixture.id}: real noteheads click individually, arrows visit each group, Alt combines attacks`, async ({
    context,
    page,
  }) => {
    test.setTimeout(180_000);
    const timeline = { ...fixture.playback, renderKey: hash, sourceHash: hash };
    const geometry = { ...fixture.geometry, renderKey: hash, sourceHash: hash };
    await mockScoreRuntime(context, { playback: timeline, pageCount: fixture.pages.length });
    await context.route('**/api/v1/renders/*/geometry', (route) =>
      route.fulfill({ json: geometry }),
    );
    await context.route('**/api/v1/renders/*/pages/*', (route) => {
      const number = Number(new URL(route.request().url()).pathname.split('/').at(-1));
      const svg = fixture.pages.find(([page]) => page === number)?.[1];
      return route.fulfill({
        status: svg ? 200 : 404,
        contentType: 'image/svg+xml',
        body: svg ?? '',
      });
    });
    await page.goto(`/scores?documentId=${firstId}`);
    await expect(page.locator('[data-page="1"] .score-image')).toBeVisible();
    await expect(page.getByRole('button', { name: '播放', exact: true })).toBeEnabled();
    await page.evaluate(async () => {
      const modulePath = '/src/score/scoreAudio.ts';
      const { scoreAudio } = await import(modulePath);
      const play = scoreAudio.play.bind(scoreAudio);
      const calls: number[][] = [];
      Reflect.set(window, 'auditionCalls', calls);
      scoreAudio.play = async (
        pitches: number[],
        start: number,
        duration: number,
        instrument: string,
      ) => {
        calls.push([...pitches]);
        await play(pitches, start, duration, instrument);
      };
    });
    const firstPage = geometry.pages[0]!;
    const heads = firstPage.elements.filter(
      (element) =>
        element.systemIndex === 0 &&
        ['NoteHead', 'TabNoteHead'].includes(element.region.elementKind ?? ''),
    );
    expect(heads.length).toBeGreaterThan(0);
    const groupFor = (element: GeometryElement) =>
      timeline.groups.find(
        (group) =>
          element.region.target.kind === 'element' &&
          group.elementIds.includes(element.region.target.elementId),
      )!;
    const reset = () => page.evaluate(() => Reflect.get(window, 'auditionCalls').splice(0));
    const sounds = () =>
      page.evaluate(() =>
        (Reflect.get(window, 'auditionCalls') as number[][]).flat().sort((a, b) => a - b),
      );
    const click = async (element: GeometryElement, alt = false) => {
      const bounds = element.bounds;
      const points = [0.25, 0.5, 0.75].flatMap((x) =>
        [0.25, 0.5, 0.75].map((y) => ({
          x: bounds.x0 + (bounds.x1 - bounds.x0) * x,
          y: bounds.y0 + (bounds.y1 - bounds.y0) * y,
        })),
      );
      const point =
        points.find((p) => {
          const target = hitElement(firstPage, p.x, p.y)?.region.target;
          return (
            target?.kind === 'element' &&
            element.region.target.kind === 'element' &&
            target.elementId === element.region.target.elementId
          );
        }) ?? points[4]!;
      const overlay = page.locator('[data-page="1"] .overlay');
      const box = (await overlay.boundingBox())!;
      const view = firstPage.viewBox!;
      await overlay.click({
        position: {
          x: ((point.x - view.x0) / (view.x1 - view.x0)) * box.width,
          y: ((point.y - view.y0) / (view.y1 - view.y0)) * box.height,
        },
        modifiers: alt ? ['Alt'] : [],
      });
    };
    for (const head of heads) {
      await reset();
      await click(head);
      await expect.poll(sounds, { message: head.region.label }).toEqual(groupFor(head).pitches);
    }
    const visited = new Set<string>();
    for (const head of heads) {
      const anchor = groupFor(head);
      const key = `${anchor.staffIndex}:${anchor.voice}`;
      if (visited.has(key)) continue;
      visited.add(key);
      const sequence = timeline.groups.filter((group) => sameVoice(group, anchor));
      const visible = new Set(heads.map(groupFor));
      const first = sequence.findIndex((group) => visible.has(group));
      const startHead = heads.find((element) => groupFor(element) === sequence[first])!;
      await click(startHead);
      for (let i = first + 1; i < sequence.length && visible.has(sequence[i]!); i++) {
        await reset();
        await page.keyboard.press('ArrowRight');
        await expect.poll(sounds).toEqual(sequence[i]!.pitches);
        await reset();
        await page.keyboard.press('ArrowLeft');
        await expect.poll(sounds).toEqual(sequence[i - 1]!.pitches);
        await page.keyboard.press('ArrowRight');
      }
    }
    const onsets = new Set<number>();
    for (const head of heads) {
      const group = groupFor(head);
      if (group.startSeconds === null || onsets.has(group.startSeconds)) continue;
      onsets.add(group.startSeconds);
      const expected = timeline.notes
        .filter((note) => Math.abs(note.startSeconds - group.startSeconds!) < 0.001)
        .map((note) => note.pitch)
        .sort((a, b) => a - b);
      await reset();
      await click(head, true);
      await expect.poll(sounds, { message: `Alt at ${group.startSeconds}` }).toEqual(expected);
    }
  });
}
