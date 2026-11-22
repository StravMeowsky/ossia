import type { CSSProperties } from 'react';

import type {
  AnnotationBox,
  GeometryBounds,
  GeometryPage,
  SelectionAnnotation,
  SelectionTarget,
} from '../../generated/protocol.ts';
import { annotationColor } from '../utils/annotationColors.ts';
import {
  ANNOTATION_PIN_SIZE,
  annotationOccurrences,
  draftBoxOccurrences,
  draftOccurrences,
} from '../utils/annotationGeometry.ts';

function percentPosition(point: { x: number; y: number }, box: GeometryBounds) {
  return {
    left: `${((point.x - box.x0) / Math.max(1e-9, box.x1 - box.x0)) * 100}%`,
    top: `${((point.y - box.y0) / Math.max(1e-9, box.y1 - box.y0)) * 100}%`,
  };
}

function percentBounds(bounds: GeometryBounds, box: GeometryBounds) {
  return {
    left: `${((bounds.x0 - box.x0) / Math.max(1e-9, box.x1 - box.x0)) * 100}%`,
    top: `${((bounds.y0 - box.y0) / Math.max(1e-9, box.y1 - box.y0)) * 100}%`,
    width: `${((bounds.x1 - bounds.x0) / Math.max(1e-9, box.x1 - box.x0)) * 100}%`,
    height: `${((bounds.y1 - bounds.y0) / Math.max(1e-9, box.y1 - box.y0)) * 100}%`,
  };
}

export function ScoreAnnotationLayer({
  annotations,
  draftBoxes = [],
  draftIndex,
  draftTargets = [],
  layout,
  onOpen,
  scale,
}: {
  annotations: SelectionAnnotation[];
  draftBoxes?: AnnotationBox[];
  draftIndex?: number;
  draftTargets?: SelectionTarget[];
  layout: GeometryPage;
  onOpen: (annotation: SelectionAnnotation, bounds: GeometryBounds) => void;
  scale: number;
}) {
  const box = layout.viewBox;
  if (!box) {
    return null;
  }
  const style = {
    '--annotation-pin-size': `${ANNOTATION_PIN_SIZE * scale}px`,
    '--annotation-pin-border': `${1.5 * scale}px`,
    '--annotation-pin-font-size': `${10 * scale}px`,
    '--annotation-pin-shadow': `0 ${3 * scale}px ${8 * scale}px rgb(0 0 0 / 28%)`,
  } as CSSProperties;
  const occurrences = annotationOccurrences(layout, annotations);
  const boxes = annotations.flatMap((annotation, index) =>
    annotation.boxes
      .filter((annotationBox) => annotationBox.page === layout.page)
      .map((annotationBox) => ({ annotation, annotationBox, index })),
  );
  const draftBoxLayer =
    draftIndex === undefined
      ? []
      : draftBoxes
          .filter((annotationBox) => annotationBox.page === layout.page)
          .map((annotationBox) => ({ annotationBox, index: draftIndex }));
  const drafts =
    draftIndex === undefined
      ? []
      : draftBoxes.length > 0
        ? draftBoxOccurrences(layout, draftBoxes, draftIndex)
        : draftOccurrences(layout, draftTargets, draftIndex);
  const pinSlots = new Map<string, number>();
  const stackedPosition = (pin: { x: number; y: number }) => {
    const key = `${pin.x}:${pin.y}`;
    const slot = pinSlots.get(key) ?? 0;
    pinSlots.set(key, slot + 1);
    const offset = slot * ANNOTATION_PIN_SIZE * scale * 0.6;
    return {
      ...percentPosition(pin, box),
      '--annotation-pin-offset-x': `${offset}px`,
      '--annotation-pin-offset-y': `${offset}px`,
    };
  };
  return (
    <div className="score-annotation-layer" style={style}>
      {boxes.map(({ annotation, annotationBox, index }, boxIndex) => (
        <span
          key={`${annotation.id}:box:${boxIndex}`}
          aria-hidden="true"
          className="score-annotation-box"
          style={
            {
              ...percentBounds(annotationBox.bounds, box),
              '--annotation-color': annotationColor(index),
            } as CSSProperties
          }
        />
      ))}
      {draftBoxLayer.map(({ annotationBox, index }, boxIndex) => (
        <span
          key={`draft-box:${boxIndex}`}
          aria-hidden="true"
          className="score-annotation-box is-draft"
          style={
            {
              ...percentBounds(annotationBox.bounds, box),
              '--annotation-color': annotationColor(index),
            } as CSSProperties
          }
        />
      ))}
      {occurrences.map(({ annotation, bounds, index, pin }, occurrence) => (
        <button
          key={`${annotation.id}:pin:${occurrence}`}
          type="button"
          className="score-annotation-pin"
          aria-label={`${index + 1}. ${annotation.text}`}
          style={
            {
              ...stackedPosition(pin),
              '--annotation-color': annotationColor(index),
            } as CSSProperties
          }
          onClick={(event) => {
            event.stopPropagation();
            onOpen(annotation, bounds);
          }}
        >
          {index + 1}
          <span role="tooltip" className="score-annotation-pin-tooltip">
            {annotation.text}
          </span>
        </button>
      ))}
      {drafts.map(({ index, pin }, occurrence) => (
        <span
          key={`draft-pin:${occurrence}`}
          aria-hidden="true"
          className="score-annotation-pin is-draft"
          style={
            {
              ...stackedPosition(pin),
              '--annotation-color': annotationColor(index),
            } as CSSProperties
          }
        >
          {index + 1}
        </span>
      ))}
    </div>
  );
}
