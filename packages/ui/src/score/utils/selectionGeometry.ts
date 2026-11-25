import type { GeometryPage, SelectionTarget } from '../../generated/protocol.ts';
import { selectionKey } from './api.ts';
export { hitElement, hitSelection } from './hitElement.ts';

/** Paint every occurrence of a selected source object. */
export function selectedBounds(page: GeometryPage, keys: Set<string>) {
  return page.regions
    .filter((region) => keys.has(selectionKey({ kind: 'measure', locator: region.locator })))
    .map(({ bounds }) => ({ bounds, element: false }));
}

/** Outline the actual hit region of selected non-note elements. */
export function selectedElementBounds(page: GeometryPage, keys: Set<string>) {
  return page.elements
    .filter(
      (element) =>
        !['NoteHead', 'TabNoteHead'].includes(element.region.elementKind ?? '') &&
        keys.has(selectionKey(element.region.target)),
    )
    .flatMap((element) =>
      (element.hitBounds.length ? element.hitBounds : [element.bounds]).map((bounds) => ({
        bounds,
        element: true,
      })),
    );
}

/** Color actual SVG groups; common stems and beams are the union of their owners. */
export function selectedGraphics(page: GeometryPage, keys: Set<string>) {
  return new Set(
    page.elements
      .filter(({ region }) => keys.has(selectionKey(region.target)))
      .flatMap((element) => element.graphicIds),
  );
}

/** Removing an attached mark also removes any selected note group that requires it. */
export function selectionGroup(pages: GeometryPage[], target: SelectionTarget, keys: Set<string>) {
  const key = selectionKey(target);
  const groups = pages
    .flatMap((page) => page.elements)
    .filter(
      (element) =>
        selectionKey(element.region.target) === key ||
        (keys.has(selectionKey(element.region.target)) &&
          element.linkedTargets.some((linked) => selectionKey(linked) === key)),
    );
  const result = new Map([[key, target]]);
  for (const group of groups)
    for (const linked of group.linkedTargets) result.set(selectionKey(linked), linked);
  return [...result.values()];
}
