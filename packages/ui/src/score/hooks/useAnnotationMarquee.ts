import { useEffect, useRef, useState } from 'react';
import type { PointerEvent as ReactPointerEvent, RefObject } from 'react';

import type { GeometryPage } from '../../generated/protocol.ts';
import { isAnnotationModifierPressed } from '../utils/annotationInput.ts';
import { intersectsMarquee, marqueeRect, marqueeSelection } from '../utils/marqueeSelection.ts';
import type { MarqueePage, MarqueeRect, MarqueeSelection } from '../utils/marqueeSelection.ts';

const DRAG_THRESHOLD = 4;
const AUTO_PAN_EDGE = 32;
const AUTO_PAN_MAX_STEP = 18;

interface Point {
  x: number;
  y: number;
}

interface Gesture {
  capture: Element;
  client: Point;
  current: Point;
  frame: number | null;
  pointerId: number;
  start: Point;
  startClient: Point;
  started: boolean;
}

/** Owns blank-area marquee selection, cross-page hit testing and edge auto-pan. */
export function useAnnotationMarquee({
  annotate,
  canvas,
  pages,
  pageHosts,
  viewport,
  pan,
  panBy,
  onSelect,
}: {
  annotate: boolean;
  canvas: RefObject<HTMLDivElement | null>;
  pages: GeometryPage[] | undefined;
  pageHosts: RefObject<Map<number, HTMLDivElement>>;
  viewport: RefObject<HTMLDivElement | null>;
  pan: { x: number; y: number };
  panBy: (deltaX: number, deltaY: number) => void;
  onSelect: (selection: MarqueeSelection, multiple: boolean) => void;
}) {
  const [rect, setRect] = useState<MarqueeRect | null>(null);
  const gesture = useRef<Gesture | null>(null);
  const selectRef = useRef(onSelect);
  const pagesRef = useRef(pages);
  const annotateRef = useRef(annotate);
  const panRef = useRef(pan);
  const suppressClick = useRef(false);
  selectRef.current = onSelect;
  pagesRef.current = pages;
  annotateRef.current = annotate;
  panRef.current = pan;

  function canvasPoint(clientX: number, clientY: number): Point {
    const box = canvas.current?.getBoundingClientRect();
    return {
      x: clientX - (box?.left ?? 0) - panRef.current.x,
      y: clientY - (box?.top ?? 0) - panRef.current.y,
    };
  }

  function clampedClientPoint(gestureState: Gesture): Point {
    const box = viewport.current?.getBoundingClientRect();
    if (!box) {
      return gestureState.client;
    }
    return {
      x: clamp(gestureState.client.x, box.left, box.right),
      y: clamp(gestureState.client.y, box.top, box.bottom),
    };
  }

  function updateEndpoint(gestureState: Gesture) {
    const client = clampedClientPoint(gestureState);
    gestureState.current = canvasPoint(client.x, client.y);
    setRect(marqueeRect(gestureState.start, gestureState.current));
  }

  function stopAutoPan(gestureState: Gesture) {
    if (gestureState.frame !== null) {
      cancelAnimationFrame(gestureState.frame);
      gestureState.frame = null;
    }
  }

  function panVelocity(gestureState: Gesture) {
    const node = viewport.current;
    if (!node) {
      return { x: 0, y: 0 };
    }
    const box = node.getBoundingClientRect();
    return {
      x: edgeVelocity(gestureState.client.x, box.left, box.right),
      y: edgeVelocity(gestureState.client.y, box.top, box.bottom),
    };
  }

  function syncAutoPan(gestureState: Gesture) {
    const velocity = panVelocity(gestureState);
    if (velocity.x === 0 && velocity.y === 0) {
      stopAutoPan(gestureState);
      return;
    }
    if (gestureState.frame !== null) {
      return;
    }
    const tick = () => {
      const current = gesture.current;
      if (!current || current !== gestureState) {
        return;
      }
      const node = viewport.current;
      if (!node) {
        gestureState.frame = null;
        return;
      }
      if (gestureState.capture !== node) {
        node.setPointerCapture(gestureState.pointerId);
        gestureState.capture = node;
      }
      const next = panVelocity(gestureState);
      const box = node.getBoundingClientRect();
      const pageRects = [...pageHosts.current.values()].map((page) => page.getBoundingClientRect());
      const canPanX =
        next.x !== 0 &&
        pageRects.some((rect) => {
          return next.x > 0 ? rect.right > box.right + 1 : rect.left < box.left - 1;
        });
      const canPanY =
        next.y !== 0 &&
        pageRects.some((rect) => {
          return next.y > 0 ? rect.bottom > box.bottom + 1 : rect.top < box.top - 1;
        });
      if (!canPanX && !canPanY) {
        gestureState.frame = null;
        return;
      }
      const panX = canPanX ? next.x : 0;
      const panY = canPanY ? next.y : 0;
      panBy(-panX, -panY);
      gestureState.current = {
        x: gestureState.current.x + panX,
        y: gestureState.current.y + panY,
      };
      setRect(marqueeRect(gestureState.start, gestureState.current));
      gestureState.frame = requestAnimationFrame(tick);
    };
    gestureState.frame = requestAnimationFrame(tick);
  }

  function collectSelection(gestureState: Gesture): MarqueeSelection {
    const node = canvas.current;
    const pageList = pagesRef.current;
    if (!node || !pageList) {
      return { boxes: [], targets: [] };
    }
    const canvasBox = node.getBoundingClientRect();
    const pan = panRef.current;
    const rendered: MarqueePage[] = [];
    for (const layout of pageList) {
      const host = pageHosts.current.get(layout.page);
      if (!host) {
        continue;
      }
      const box = host.getBoundingClientRect();
      const bounds = {
        x0: box.left - canvasBox.left - pan.x,
        x1: box.right - canvasBox.left - pan.x,
        y0: box.top - canvasBox.top - pan.y,
        y1: box.bottom - canvasBox.top - pan.y,
      };
      if (intersectsMarquee(marqueeRect(gestureState.start, gestureState.current), bounds)) {
        rendered.push({ bounds, layout });
      }
    }
    return marqueeSelection(rendered, marqueeRect(gestureState.start, gestureState.current));
  }

  function finish(gestureState: Gesture) {
    stopAutoPan(gestureState);
    if (gestureState.capture.hasPointerCapture(gestureState.pointerId)) {
      gestureState.capture.releasePointerCapture(gestureState.pointerId);
    }
    gesture.current = null;
    setRect(null);
  }

  function cancel() {
    const current = gesture.current;
    if (current) {
      finish(current);
    }
  }

  useEffect(() => {
    const onPointerMove = (event: PointerEvent) => {
      const current = gesture.current;
      if (!current || current.pointerId !== event.pointerId) {
        return;
      }
      current.client = { x: event.clientX, y: event.clientY };
      const dx = current.client.x - current.startClient.x;
      const dy = current.client.y - current.startClient.y;
      if (!current.started && Math.hypot(dx, dy) < DRAG_THRESHOLD) {
        return;
      }
      current.started = true;
      updateEndpoint(current);
      syncAutoPan(current);
    };
    const onPointerUp = (event: PointerEvent) => {
      const current = gesture.current;
      if (!current || current.pointerId !== event.pointerId) {
        return;
      }
      const selection = current.started ? collectSelection(current) : { boxes: [], targets: [] };
      const started = current.started;
      finish(current);
      if (!started) {
        return;
      }
      suppressClick.current = true;
      window.setTimeout(() => {
        suppressClick.current = false;
      }, 0);
      if (selection.targets.length) {
        selectRef.current(selection, isAnnotationModifierPressed(event));
      }
    };
    const onPointerCancel = () => cancel();
    const onBlur = () => cancel();
    window.addEventListener('pointermove', onPointerMove);
    window.addEventListener('pointerup', onPointerUp);
    window.addEventListener('pointercancel', onPointerCancel);
    window.addEventListener('blur', onBlur);
    return () => {
      window.removeEventListener('pointermove', onPointerMove);
      window.removeEventListener('pointerup', onPointerUp);
      window.removeEventListener('pointercancel', onPointerCancel);
      window.removeEventListener('blur', onBlur);
      cancel();
    };
  }, []);

  useEffect(() => {
    if (!annotate) {
      cancel();
    }
  }, [annotate]);

  function begin(event: ReactPointerEvent<SVGSVGElement>) {
    if (!annotateRef.current || event.button !== 0) {
      return false;
    }
    const start = canvasPoint(event.clientX, event.clientY);
    const current: Gesture = {
      capture: event.currentTarget,
      client: { x: event.clientX, y: event.clientY },
      current: start,
      frame: null,
      pointerId: event.pointerId,
      start,
      startClient: { x: event.clientX, y: event.clientY },
      started: false,
    };
    gesture.current = current;
    event.currentTarget.setPointerCapture(event.pointerId);
    return true;
  }

  function consumeClick() {
    if (!suppressClick.current) {
      return false;
    }
    suppressClick.current = false;
    return true;
  }

  return { begin, consumeClick, rect };
}

function clamp(value: number, minimum: number, maximum: number) {
  return Math.min(maximum, Math.max(minimum, value));
}

function edgeVelocity(value: number, start: number, end: number) {
  const leading = value - start;
  const trailing = end - value;
  if (leading < AUTO_PAN_EDGE) {
    return -AUTO_PAN_MAX_STEP * (1 - Math.max(0, leading) / AUTO_PAN_EDGE);
  }
  if (trailing < AUTO_PAN_EDGE) {
    return AUTO_PAN_MAX_STEP * (1 - Math.max(0, trailing) / AUTO_PAN_EDGE);
  }
  return 0;
}
