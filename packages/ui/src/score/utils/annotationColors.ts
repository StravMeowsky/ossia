import type {
  GeometryPage,
  SelectionAnnotation,
  SelectionTarget,
} from '../../generated/protocol.ts';
import { selectionKey } from './api.ts';
import { selectionGroup } from './selectionGeometry.ts';

const ANNOTATION_COLORS = [
  'var(--annotation-color-1)',
  'var(--annotation-color-2)',
  'var(--annotation-color-3)',
  'var(--annotation-color-4)',
  'var(--annotation-color-5)',
  'var(--annotation-color-6)',
] as const;

/** Resolves the stable display color for one zero-based annotation group ordinal. */
export function annotationColor(index: number) {
  return ANNOTATION_COLORS[index % ANNOTATION_COLORS.length]!;
}

/** Map every drawn SVG group to the first annotation group that affects it. */
export function annotationGraphicColors(
  pages: readonly GeometryPage[],
  annotations: readonly SelectionAnnotation[],
  preview?: { index: number; targets: readonly SelectionTarget[] },
): ReadonlyMap<string, string> {
  const targetColors = new Map<string, number>();
  const pageList = [...pages];
  const assign = (targets: readonly SelectionTarget[], index: number) => {
    const keys = new Set(targets.map(selectionKey));
    for (const target of targets) {
      for (const linked of selectionGroup(pageList, target, keys)) {
        const key = selectionKey(linked);
        targetColors.set(key, index);
      }
    }
  };
  annotations.forEach((annotation, index) => assign(annotation.targets, index));
  if (preview) assign(preview.targets, preview.index);

  const colors = new Map<string, string>();
  for (const page of pages) {
    for (const element of page.elements) {
      const index = targetColors.get(selectionKey(element.region.target));
      if (index === undefined) continue;
      const color = annotationColor(index);
      for (const graphicId of element.graphicIds) {
        if (!colors.has(graphicId)) colors.set(graphicId, color);
      }
    }
  }
  return colors;
}
