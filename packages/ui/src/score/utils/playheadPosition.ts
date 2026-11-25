import type { GeometryPage, PlaybackSystemSpan } from '../../generated/protocol.ts';

/** Locate a playback group only when every referenced element has one position. */
export function locatePlayhead(
  pages: GeometryPage[],
  systems: PlaybackSystemSpan[],
  elementIds: Set<string>,
) {
  const positions: { span: PlaybackSystemSpan; x: number }[] = [];
  for (const elementId of elementIds) {
    const candidates: { span: PlaybackSystemSpan; x: number }[] = [];
    for (const page of pages) {
      for (const element of page.elements) {
        if (
          element.region.target.kind !== 'element' ||
          element.region.target.elementId !== elementId
        )
          continue;
        const x = (element.bounds.x0 + element.bounds.x1) / 2;
        let span = systems.find(
          (system) => system.page === page.page && system.systemIndex === element.systemIndex,
        );
        if (!span && element.systemIndex === null) {
          const y = (element.bounds.y0 + element.bounds.y1) / 2;
          const containing = systems.filter(
            (system) => system.page === page.page && y >= system.y0 && y <= system.y1,
          );
          const indices = new Set(containing.map((system) => system.systemIndex));
          if (indices.size === 1) span = containing[0];
        }
        if (span) candidates.push({ span, x });
      }
    }
    if (candidates.length !== 1) return null;
    positions.push(candidates[0]!);
  }
  positions.sort(
    (a, b) => a.span.page - b.span.page || a.span.systemIndex - b.span.systemIndex || a.x - b.x,
  );
  return positions[0] ?? null;
}
