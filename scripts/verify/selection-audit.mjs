import { hitSelection } from '../../packages/ui/src/score/utils/hitElement.ts';

const rhythmicKinds = new Set(['NoteHead', 'TabNoteHead', 'Rest', 'MultiMeasureRest']);
const center = (a, b) => (a + b) / 2;
const fractions = [0.5, 0.33, 0.67, 0.2, 0.8, 0.1, 0.9, 0.05, 0.95];
const sameLocator = (a, b) =>
  a.staffIndex === b.staffIndex &&
  a.measureStart === b.measureStart &&
  a.measureEnd === b.measureEnd &&
  a.voice === b.voice;

/** Audit engraved staff/measure intervals independently of the selectable regions. */
export function auditSelection({ layout, geometry, selection }) {
  if (layout.sourceHash !== geometry.sourceHash) throw new Error('mismatched selection render');
  const issues = [];
  const clicks = [];
  let cells = 0;
  if (!selection.reliable)
    issues.push({ kind: 'unreliable-musical-identity', reason: selection.measureCountError });
  const staffVoices = selection.staves.map(({ voices }) =>
    selection.voices.filter(({ name }) => voices.includes(name)).map(({ musicSpan }) => musicSpan),
  );
  for (const [pageIndex, page] of layout.pages.entries()) {
    const hitPage = geometry.pages.find((candidate) => candidate.page === page.page);
    if (!hitPage || !page.systems.length) {
      issues.push({ kind: 'missing-page-grid', page: page.page, reason: page.reason });
      continue;
    }
    const rhythmic = page.objects.filter(
      (object) => !object.hitOnly && object.span && rhythmicKinds.has(object.kind),
    );
    const originsByStaff = staffVoices.map((spans) =>
      rhythmic.filter((object) =>
        spans.some(
          (span) => span.byteStart <= object.span.byteStart && span.byteEnd > object.span.byteStart,
        ),
      ),
    );
    for (const [systemIndex, system] of page.systems.entries()) {
      if (system.barXs.length < 2) {
        issues.push({ kind: 'missing-barlines', page: page.page, systemIndex });
        continue;
      }
      for (let column = 0; column + 1 < system.barXs.length; column++) {
        for (const [staffIndex, staff] of system.staves.entries()) {
          const x0 = Math.max(staff.x0, system.barXs[column]);
          const x1 = Math.min(staff.x1, system.barXs[column + 1]);
          if (x1 - x0 <= page.staffSpace * 0.5) continue;
          cells++;
          const detail = { page: page.page, systemIndex, column, staffIndex };
          let click;
          for (const fx of fractions) {
            for (const fy of fractions) {
              const x = x0 + fx * (x1 - x0);
              const y = staff.top + fy * (staff.bot - staff.top);
              const target = hitSelection(hitPage, x, y);
              if (target?.kind === 'measure' && target.locator.staffIndex === staffIndex) {
                click = { ...detail, target };
                break;
              }
            }
            if (click) break;
          }
          if (!click) {
            issues.push({ kind: 'unselectable-measure', ...detail });
            continue;
          }
          clicks.push(click);
          const { locator } = click.target;
          const region = hitPage.regions.find(
            (region) => region.systemIndex === systemIndex && sameLocator(region.locator, locator),
          );
          const resolved = selection.pages[pageIndex].find((region) =>
            sameLocator(region.target.locator, locator),
          );
          if (!region || !resolved?.spans.length) {
            issues.push({ kind: 'unresolved-measure-target', ...detail, locator });
            continue;
          }
          // Cross-staff notes keep the source voice of their original staff.
          const origins =
            originsByStaff[staffIndex]?.filter(({ bbox, staffIndex: owner }) => {
              const x = center(bbox.x0, bbox.x1);
              const y = center(bbox.y0, bbox.y1);
              return owner === staffIndex && x >= x0 && x < x1 && y >= staff.top && y <= staff.bot;
            }) ?? [];
          const missing = origins.filter(
            ({ span }) =>
              !resolved.spans.some(
                (source) => source.byteStart <= span.byteStart && source.byteEnd > span.byteStart,
              ),
          );
          if (missing.length)
            issues.push({
              kind: 'wrong-measure-source',
              ...detail,
              locator,
              lines: [...new Set(missing.map(({ span }) => span.start.line))],
            });
        }
      }
    }
  }
  if (!cells) issues.push({ kind: 'empty-measure-grid' });
  return { cells, clicks, issues };
}
