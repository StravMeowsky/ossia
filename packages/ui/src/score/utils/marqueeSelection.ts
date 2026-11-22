import type {
  AnnotationBox,
  GeometryBounds,
  GeometryElement,
  GeometryPage,
  SelectionTarget,
} from '../../generated/protocol.ts';
import { selectionKey } from './api.ts';

/** A page's rendered bounds in the score canvas coordinate space. */
export interface MarqueePage {
  bounds: GeometryBounds;
  layout: GeometryPage;
}

/** A normalized rectangle in score canvas coordinates. */
export interface MarqueeRect {
  x0: number;
  x1: number;
  y0: number;
  y1: number;
}

export interface MarqueeSelection {
  boxes: AnnotationBox[];
  targets: SelectionTarget[];
}

export function marqueeRect(
  start: { x: number; y: number },
  end: { x: number; y: number },
): MarqueeRect {
  return {
    x0: Math.min(start.x, end.x),
    x1: Math.max(start.x, end.x),
    y0: Math.min(start.y, end.y),
    y1: Math.max(start.y, end.y),
  };
}

export function intersectsMarquee(rect: MarqueeRect, bounds: GeometryBounds) {
  return (
    bounds.x1 >= rect.x0 && bounds.x0 <= rect.x1 && bounds.y1 >= rect.y0 && bounds.y0 <= rect.y1
  );
}

function intersectMarquee(rect: MarqueeRect, bounds: GeometryBounds): GeometryBounds | undefined {
  if (!intersectsMarquee(rect, bounds)) {
    return undefined;
  }
  return {
    x0: Math.max(rect.x0, bounds.x0),
    x1: Math.min(rect.x1, bounds.x1),
    y0: Math.max(rect.y0, bounds.y0),
    y1: Math.min(rect.y1, bounds.y1),
  };
}

function elementInkBounds(element: GeometryElement) {
  return element.hitBounds.length ? element.hitBounds : [element.bounds];
}

function canAnnotateElement(element: GeometryElement) {
  return element.region.spans.length > 0 || element.region.insertionAnchor !== null;
}

/** Resolves enclosed targets and one persisted box fragment per covered page. */
export function marqueeSelection(
  pages: readonly MarqueePage[],
  rect: MarqueeRect,
): MarqueeSelection {
  const targets = new Map<string, SelectionTarget>();
  const boxes: AnnotationBox[] = [];
  let pinAssigned = false;
  for (const page of [...pages].sort((left, right) => left.layout.page - right.layout.page)) {
    const intersection = intersectMarquee(rect, page.bounds);
    if (!intersection || intersection.x1 <= intersection.x0 || intersection.y1 <= intersection.y0) {
      continue;
    }
    const box = page.layout.viewBox;
    if (!box || box.x1 <= box.x0 || box.y1 <= box.y0) {
      continue;
    }
    const scaleX = (page.bounds.x1 - page.bounds.x0) / (box.x1 - box.x0);
    const scaleY = (page.bounds.y1 - page.bounds.y0) / (box.y1 - box.y0);
    const pageTargets: SelectionTarget[] = [];
    for (const element of page.layout.elements) {
      if (!canAnnotateElement(element)) {
        continue;
      }
      const touched = elementInkBounds(element).some((bounds) => {
        const x0 = page.bounds.x0 + (bounds.x0 - box.x0) * scaleX;
        const x1 = page.bounds.x0 + (bounds.x1 - box.x0) * scaleX;
        const y0 = page.bounds.y0 + (bounds.y0 - box.y0) * scaleY;
        const y1 = page.bounds.y0 + (bounds.y1 - box.y0) * scaleY;
        return intersectsMarquee(rect, {
          x0: Math.min(x0, x1),
          x1: Math.max(x0, x1),
          y0: Math.min(y0, y1),
          y1: Math.max(y0, y1),
        });
      });
      if (touched) {
        targets.set(selectionKey(element.region.target), element.region.target);
        pageTargets.push(element.region.target);
      }
    }
    const pin = !pinAssigned && pageTargets.length > 0;
    if (pin) {
      pinAssigned = true;
    }
    boxes.push({
      page: page.layout.page,
      bounds: {
        x0: box.x0 + (intersection.x0 - page.bounds.x0) / scaleX,
        x1: box.x0 + (intersection.x1 - page.bounds.x0) / scaleX,
        y0: box.y0 + (intersection.y0 - page.bounds.y0) / scaleY,
        y1: box.y0 + (intersection.y1 - page.bounds.y0) / scaleY,
      },
      pin,
    });
  }
  return { boxes, targets: [...targets.values()] };
}
