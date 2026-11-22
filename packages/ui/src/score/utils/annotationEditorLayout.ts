export interface EditorRect {
  x0: number;
  x1: number;
  y0: number;
  y1: number;
}

export interface AnnotationEditorPlacement {
  anchor: { x: number; y: number };
  obstacles: readonly EditorRect[];
  selection: EditorRect;
  viewport: EditorRect;
}

export interface PlacedEditor {
  height: number;
  left: number;
  top: number;
  width: number;
}

/** Places the annotation editor in the closest viewport area that covers no selected elements. */
export function placeAnnotationEditor({
  placement,
  preferredWidth,
  height,
  margin = 8,
  gap = 12,
}: {
  placement: AnnotationEditorPlacement;
  preferredWidth: number;
  height: number;
  margin?: number;
  gap?: number;
}): PlacedEditor {
  const { anchor, obstacles, selection, viewport } = placement;
  const minLeft = viewport.x0 + margin;
  const maxRight = Math.max(minLeft, viewport.x1 - margin);
  const minTop = viewport.y0 + margin;
  const maxBottom = Math.max(minTop, viewport.y1 - margin);
  const width = Math.min(preferredWidth, maxRight - minLeft);
  const selectionCenterX = (selection.x0 + selection.x1) / 2;
  const selectionCenterY = (selection.y0 + selection.y1) / 2;
  const candidates = [
    {
      left: selectionCenterX - width / 2,
      top: selection.y0 - height - gap,
    },
    {
      left: selectionCenterX - width / 2,
      top: selection.y1 + gap,
    },
    {
      left: selection.x0 - width - gap,
      top: selectionCenterY - height / 2,
    },
    {
      left: selection.x1 + gap,
      top: selectionCenterY - height / 2,
    },
    {
      left: anchor.x - width / 2,
      top: anchor.y - height - gap,
    },
    {
      left: anchor.x - width / 2,
      top: anchor.y + gap,
    },
  ];

  let best: PlacedEditor | undefined;
  let bestScore = Number.POSITIVE_INFINITY;
  for (const candidate of candidates) {
    const raw = {
      height,
      left: candidate.left,
      top: candidate.top,
      width,
    };
    const placed = {
      ...raw,
      left: clamp(raw.left, minLeft, Math.max(minLeft, maxRight - width)),
      top: clamp(raw.top, minTop, Math.max(minTop, maxBottom - height)),
    };
    const editorBounds = {
      x0: placed.left,
      x1: placed.left + width,
      y0: placed.top,
      y1: placed.top + height,
    };
    const overlap = obstacles.reduce(
      (total, bounds) => total + overlapArea(editorBounds, bounds),
      0,
    );
    const overflow = overflowArea(raw, {
      x0: minLeft,
      x1: maxRight,
      y0: minTop,
      y1: maxBottom,
    });
    const distance =
      (editorBounds.x0 + width / 2 - anchor.x) ** 2 +
      (editorBounds.y0 + height / 2 - anchor.y) ** 2;
    const score = overlap * 1_000_000 + overflow * 1_000 + distance;
    if (score < bestScore) {
      best = placed;
      bestScore = score;
    }
  }
  return best!;
}

function overlapArea(left: EditorRect, right: EditorRect) {
  const width = Math.max(0, Math.min(left.x1, right.x1) - Math.max(left.x0, right.x0));
  const height = Math.max(0, Math.min(left.y1, right.y1) - Math.max(left.y0, right.y0));
  return width * height;
}

function overflowArea(rect: PlacedEditor, viewport: EditorRect) {
  const width = rect.width;
  const height = rect.height;
  const insideWidth = Math.max(
    0,
    Math.min(rect.left + width, viewport.x1) - Math.max(rect.left, viewport.x0),
  );
  const insideHeight = Math.max(
    0,
    Math.min(rect.top + height, viewport.y1) - Math.max(rect.top, viewport.y0),
  );
  return width * height - insideWidth * insideHeight;
}

function clamp(value: number, minimum: number, maximum: number) {
  return Math.min(maximum, Math.max(minimum, value));
}
