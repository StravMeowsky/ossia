import type { GeometryBounds, GeometryElement, GeometryPage } from '../../generated/protocol.ts';

/** Test a point against inclusive SVG bounds. */
export const contains = (bounds: GeometryBounds, x: number, y: number) =>
  x >= bounds.x0 && x <= bounds.x1 && y >= bounds.y0 && y <= bounds.y1;
const area = (bounds: GeometryBounds) => (bounds.x1 - bounds.x0) * (bounds.y1 - bounds.y0);

/** Match the backend's local-symbol, spanning-mark, then measure ordering. */
export function hitElement(page: GeometryPage, x: number, y: number): GeometryElement | undefined {
  return page.elements
    .filter((candidate) =>
      candidate.hitBounds.length
        ? candidate.hitBounds.some((bounds) => contains(bounds, x, y))
        : contains(candidate.bounds, x, y),
    )
    .sort((a, b) => a.priority - b.priority || area(a.bounds) - area(b.bounds))[0];
}
