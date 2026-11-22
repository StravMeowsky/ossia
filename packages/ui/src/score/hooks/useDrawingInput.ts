import { useCallback, useRef, useState } from 'react';
import type { PointerEvent as ReactPointerEvent } from 'react';

import type { DrawingColor, DrawingPoint, DrawingStroke } from '../../generated/protocol.ts';
import { eraseStrokes } from '../utils/drawing.ts';
import type { DrawingController } from './useDrawing.ts';

export type DrawingMode = 'pen' | 'eraser' | null;

type Session = {
  pointerId: number;
  pointerType: string;
  page: number;
  bounds: DOMRect;
  aspect: number;
  width: number;
  erase: boolean;
  color: DrawingColor;
  points: DrawingPoint[];
};

const TOUCH_DRAW_THRESHOLD = 4;

/** Coordinates pointer drawing with the viewport's pan and pinch recognizer. */
export function useDrawingInput(
  controller: DrawingController | undefined,
  mode: DrawingMode,
  penSize: number,
  eraserSize: number,
  color: DrawingColor,
) {
  const active = useRef<Session | null>(null);
  const touches = useRef(new Set<number>());
  const [preview, setPreview] = useState<Session | null>(null);
  const pointAt = (session: Session, x: number, y: number): DrawingPoint => ({
    x: Math.max(0, Math.min(1, (x - session.bounds.left) / session.bounds.width)),
    y: Math.max(0, Math.min(1, (y - session.bounds.top) / session.bounds.height)),
  });
  const appendPoint = (session: Session, x: number, y: number) => {
    const next = pointAt(session, x, y);
    const last = session.points.at(-1)!;
    const distance = Math.hypot(next.x - last.x, (next.y - last.y) * session.aspect);
    if (!distance) return;
    const steps = session.erase ? Math.max(1, Math.ceil(distance / (session.width / 4))) : 1;
    for (let i = 1; i <= steps; i++)
      session.points.push({
        x: last.x + ((next.x - last.x) * i) / steps,
        y: last.y + ((next.y - last.y) * i) / steps,
      });
  };
  const finish = useCallback(() => {
    const session = active.current;
    if (!session || !controller) return;
    active.current = null;
    setPreview(null);
    const { page, width, points, aspect, erase, color } = session;
    if (erase) {
      controller.edit((strokes) =>
        points.reduce((next, point) => eraseStrokes(next, page, point, width / 2, aspect), strokes),
      );
    } else {
      const stroke = { page, width, points, color };
      controller.edit((strokes) => [...strokes, stroke]);
    }
  }, [controller]);
  const down = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (!mode || !controller?.ready || event.button !== 0) return 'normal';
    const session = active.current;
    if (session?.pointerType === 'pen' && event.pointerType === 'touch') return 'ignore';
    let resetGesture = false;
    if (event.pointerType === 'pen' && (session?.pointerType === 'touch' || touches.current.size)) {
      active.current = null;
      setPreview(null);
      touches.current.clear();
      resetGesture = true;
    }
    if (event.pointerType === 'touch') {
      touches.current.add(event.pointerId);
      if (touches.current.size > 1) {
        if (session?.pointerType === 'touch') {
          const first = session.points[0]!;
          const moved = session.points.some(
            (point) =>
              Math.hypot(
                (point.x - first.x) * session.bounds.width,
                (point.y - first.y) * session.bounds.height,
              ) >= TOUCH_DRAW_THRESHOLD,
          );
          if (moved) finish();
          else {
            active.current = null;
            setPreview(null);
          }
        }
        return 'normal';
      }
    }
    if ((session && !resetGesture) || touches.current.size > 1) return 'normal';
    const host =
      event.target instanceof Element ? event.target.closest<HTMLElement>('.page-host') : null;
    const page = Number(host?.dataset.page);
    if (!host || !page) return resetGesture ? 'reset' : 'normal';
    const bounds = host.getBoundingClientRect();
    if (!bounds.width || !bounds.height) return resetGesture ? 'reset' : 'normal';
    const erase = mode === 'eraser' || event.metaKey || event.ctrlKey;
    const next: Session = {
      pointerId: event.pointerId,
      pointerType: event.pointerType,
      page,
      bounds,
      aspect: bounds.height / bounds.width,
      width: (erase ? eraserSize : penSize) / bounds.width,
      erase,
      color,
      points: [],
    };
    next.points.push(pointAt(next, event.clientX, event.clientY));
    active.current = next;
    setPreview({ ...next, points: [...next.points] });
    event.currentTarget.setPointerCapture(event.pointerId);
    event.preventDefault();
    return resetGesture ? 'reset' : 'normal';
  };
  const move = (event: ReactPointerEvent<HTMLDivElement>) => {
    let session = active.current;
    if (session?.pointerType === 'pen' && event.pointerType === 'touch') return true;
    if (!session || session.pointerId !== event.pointerId) return false;
    const eraseNow = mode === 'eraser' || event.metaKey || event.ctrlKey;
    if (eraseNow !== session.erase) {
      const previous = session;
      const point = previous.points.at(-1)!;
      finish();
      session = {
        ...previous,
        erase: eraseNow,
        width: (eraseNow ? eraserSize : penSize) / previous.bounds.width,
        points: [point],
      };
      active.current = session;
    }
    const events = event.nativeEvent.getCoalescedEvents?.() ?? [event.nativeEvent];
    for (const sample of events.length ? events : [event.nativeEvent]) {
      appendPoint(session, sample.clientX, sample.clientY);
    }
    appendPoint(session, event.clientX, event.clientY);
    setPreview({ ...session, points: [...session.points] });
    event.preventDefault();
    return false;
  };
  const up = (event: ReactPointerEvent<HTMLDivElement>) => {
    touches.current.delete(event.pointerId);
    if (active.current?.pointerType === 'pen' && event.pointerType === 'touch') return true;
    if (active.current?.pointerId === event.pointerId) {
      if (event.type === 'pointerup') appendPoint(active.current, event.clientX, event.clientY);
      finish();
    }
    return false;
  };
  const strokes = controller?.drawing.strokes ?? [];
  let displayed = strokes;
  if (preview?.erase) {
    displayed = preview.points.reduce(
      (next, point) => eraseStrokes(next, preview.page, point, preview.width / 2, preview.aspect),
      strokes,
    );
  } else if (preview) {
    displayed = [
      ...strokes,
      { page: preview.page, width: preview.width, points: preview.points, color: preview.color },
    ];
  }
  return { active, down, move, up, strokes: displayed as DrawingStroke[] };
}
