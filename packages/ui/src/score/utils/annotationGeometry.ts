import type {
  AnnotationBox,
  GeometryBounds,
  GeometryPage,
  SelectionAnnotation,
  SelectionTarget,
} from '../../generated/protocol.ts';
import { selectionKey } from './api.ts';
import { BASE_PAGE_WIDTH } from './scoreLayout.ts';

export const ANNOTATION_PIN_SIZE = 20;

interface TargetOccurrence {
  bounds: GeometryBounds;
  pin: { x: number; y: number };
  target: SelectionTarget;
}

export interface AnnotationOccurrence {
  annotation: SelectionAnnotation;
  bounds: GeometryBounds;
  index: number;
  pin: { x: number; y: number };
  target?: SelectionTarget;
}

export interface DraftOccurrence {
  bounds: GeometryBounds;
  index: number;
  pin: { x: number; y: number };
  target?: SelectionTarget;
}

/** Returns every printed occurrence of one primary annotation target. */
export function targetOccurrences(page: GeometryPage, target: SelectionTarget): TargetOccurrence[] {
  const key = selectionKey(target);
  if (target.kind === 'measure') {
    return page.regions
      .filter((region) => selectionKey({ kind: 'measure', locator: region.locator }) === key)
      .map((region) => {
        const bounds = region.bounds;
        return { bounds, pin: resolveMeasurePin(page, bounds), target };
      });
  }
  return page.elements
    .filter((element) => selectionKey(element.region.target) === key)
    .map((element) => ({
      bounds: element.bounds,
      pin: resolvePin(
        page,
        element.bounds,
        ['NoteHead', 'TabNoteHead'].includes(element.region.elementKind ?? ''),
      ),
      target,
    }));
}

/** Resolves the visible pins of all annotation groups on one page. */
export function annotationOccurrences(
  page: GeometryPage,
  annotations: readonly SelectionAnnotation[],
): AnnotationOccurrence[] {
  return annotations.flatMap((annotation, index) => {
    if (annotation.boxes.length > 0) {
      return annotation.boxes
        .filter((box) => box.page === page.page && box.pin)
        .map((box) => ({
          annotation,
          bounds: box.bounds,
          index,
          pin: resolveMeasurePin(page, box.bounds),
        }));
    }
    return annotation.targets.flatMap((target) =>
      targetOccurrences(page, target).map(({ bounds, pin, target: occurrenceTarget }) => ({
        annotation,
        bounds,
        index,
        pin,
        target: occurrenceTarget,
      })),
    );
  });
}

/** Resolves uncommitted pins for a draft annotation group. */
export function draftOccurrences(
  page: GeometryPage,
  targets: readonly SelectionTarget[],
  index: number,
): DraftOccurrence[] {
  return targets.flatMap((target) =>
    targetOccurrences(page, target).map(({ bounds, pin, target: occurrenceTarget }) => ({
      bounds,
      index,
      pin,
      target: occurrenceTarget,
    })),
  );
}

/** Resolves uncommitted box pins for a marquee draft group. */
export function draftBoxOccurrences(
  page: GeometryPage,
  boxes: readonly AnnotationBox[],
  index: number,
): DraftOccurrence[] {
  return boxes
    .filter((box) => box.page === page.page && box.pin)
    .map((box) => ({
      bounds: box.bounds,
      index,
      pin: resolveMeasurePin(page, box.bounds),
    }));
}

/** Resolves the first printed anchor for an annotation editor. */
export function firstTargetBounds(page: GeometryPage, target: SelectionTarget) {
  return targetOccurrences(page, target)[0]?.bounds;
}

function resolvePin(
  page: GeometryPage,
  bounds: GeometryBounds,
  note: boolean,
): { x: number; y: number } {
  const box = page.viewBox;
  if (!box) {
    return { x: bounds.x1, y: bounds.y0 };
  }
  const radius = (ANNOTATION_PIN_SIZE / 2) * ((box.x1 - box.x0) / BASE_PAGE_WIDTH);
  const gap = radius * 0.4;
  const centerX = (bounds.x0 + bounds.x1) / 2;
  const centerY = (bounds.y0 + bounds.y1) / 2;
  const offset = radius + gap;
  const candidates = [
    { x: bounds.x1 + offset, y: bounds.y0 - offset },
    { x: bounds.x0 - offset, y: bounds.y0 - offset },
    { x: bounds.x1 + offset, y: bounds.y1 + offset },
    { x: bounds.x0 - offset, y: bounds.y1 + offset },
    { x: bounds.x1 + offset, y: centerY },
    { x: bounds.x0 - offset, y: centerY },
    { x: centerX, y: bounds.y0 - offset },
    { x: centerX, y: bounds.y1 + offset },
  ].map((point) => ({
    x: clamp(point.x, box.x0 + radius, box.x1 - radius),
    y: clamp(point.y, box.y0 + radius, box.y1 - radius),
  }));
  let best = candidates[0]!;
  let bestScore = pinScore(page, contactBounds(best, radius), note);
  for (const [index, candidate] of candidates.entries()) {
    const score = pinScore(page, contactBounds(candidate, radius), note) + index * 1e-6;
    if (score < bestScore) {
      best = candidate;
      bestScore = score;
    }
  }
  return best;
}

function resolveMeasurePin(page: GeometryPage, bounds: GeometryBounds): { x: number; y: number } {
  const box = page.viewBox;
  if (!box) {
    return { x: bounds.x1, y: bounds.y0 };
  }
  const radius = (ANNOTATION_PIN_SIZE / 2) * ((box.x1 - box.x0) / BASE_PAGE_WIDTH);
  const inset = radius * 1.1;
  const centerX = (bounds.x0 + bounds.x1) / 2;
  const candidates = [
    { x: bounds.x1 - inset, y: bounds.y0 + inset },
    { x: bounds.x0 + inset, y: bounds.y0 + inset },
    { x: bounds.x1 - inset, y: bounds.y1 - inset },
    { x: bounds.x0 + inset, y: bounds.y1 - inset },
    { x: centerX, y: bounds.y0 + inset },
    { x: centerX, y: bounds.y1 - inset },
  ].map((point) => ({
    x: clamp(point.x, bounds.x0 + radius, bounds.x1 - radius),
    y: clamp(point.y, bounds.y0 + radius, bounds.y1 - radius),
  }));
  let best = candidates[0]!;
  let bestScore = pinScore(page, contactBounds(best, radius), false);
  for (const [index, candidate] of candidates.entries()) {
    const score = pinScore(page, contactBounds(candidate, radius), false) + index * 1e-6;
    if (score < bestScore) {
      best = candidate;
      bestScore = score;
    }
  }
  return best;
}

function pinScore(page: GeometryPage, bounds: GeometryBounds, note: boolean) {
  return page.elements.reduce((score, element) => {
    const overlap = overlapArea(bounds, element.bounds);
    if (overlap === 0) {
      return score;
    }
    const primary = ['NoteHead', 'TabNoteHead'].includes(element.region.elementKind ?? '');
    return score + overlap * (note && primary ? 8 : 1);
  }, 0);
}

function contactBounds(point: { x: number; y: number }, radius: number): GeometryBounds {
  return {
    x0: point.x - radius,
    x1: point.x + radius,
    y0: point.y - radius,
    y1: point.y + radius,
  };
}

function overlapArea(left: GeometryBounds, right: GeometryBounds) {
  const width = Math.max(0, Math.min(left.x1, right.x1) - Math.max(left.x0, right.x0));
  const height = Math.max(0, Math.min(left.y1, right.y1) - Math.max(left.y0, right.y0));
  return width * height;
}

function clamp(value: number, minimum: number, maximum: number) {
  return Math.min(maximum, Math.max(minimum, value));
}
