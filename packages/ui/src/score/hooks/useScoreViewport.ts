import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { PointerEvent as ReactPointerEvent, RefObject } from 'react';

import { clampZoom } from '../utils/scoreLayout.ts';
import type { ScoreLayout } from '../utils/scoreLayout.ts';
import type { ScoreViewPan } from './useScoreViewMemory.ts';

const WHEEL_ZOOM_FACTOR = 1.2;
const TRACKPAD_ZOOM_SENSITIVITY = 0.01;
const WHEEL_LINE_HEIGHT = 16;
const TAP_PAN_THRESHOLD = 4;

interface Point {
  x: number;
  y: number;
}

interface ZoomOrigin {
  page: HTMLElement | null;
  fractionX: number;
  fractionY: number;
}

interface ZoomAnchor extends ZoomOrigin {
  clientX: number;
  clientY: number;
}

interface PinchState extends ZoomOrigin {
  startDistance: number;
  startZoom: number;
  lastX: number;
  lastY: number;
}

interface SafariGestureEvent extends Event {
  readonly scale?: number;
  readonly clientX?: number;
  readonly clientY?: number;
}

interface ViewportOptions {
  zoom: number;
  onZoomChange: (zoom: number) => void;
  layout: ScoreLayout;
  pageHosts: RefObject<Map<number, HTMLDivElement>>;
  ready: boolean;
  viewKey: string | undefined;
  initialPan: ScoreViewPan;
  panEnabled: boolean;
  drawingActive?: RefObject<unknown>;
}

function pointDistance(first: Point, second: Point) {
  return Math.hypot(second.x - first.x, second.y - first.y);
}

function pointMidpoint(first: Point, second: Point) {
  return { x: (first.x + second.x) / 2, y: (first.y + second.y) / 2 };
}

function isDirectPointer(pointerType: string) {
  return pointerType === 'touch' || pointerType === 'pen';
}

/** Owns free canvas panning, pointer-anchored zoom and view restoration. */
export function useScoreViewport({
  zoom,
  onZoomChange,
  layout,
  pageHosts,
  ready,
  viewKey,
  initialPan,
  panEnabled,
  drawingActive,
}: ViewportOptions) {
  const viewport = useRef<HTMLDivElement>(null);
  const restoredView = useRef<string | null>(null);
  const zoomRef = useRef(zoom);
  const [dragging, setDragging] = useState(false);
  const [modifierDown, setModifierDown] = useState(false);
  const [pan, setPan] = useState<ScoreViewPan>(initialPan);
  const panRef = useRef(pan);
  const pendingAnchor = useRef<ZoomAnchor | null>(null);
  const touchPointers = useRef(new Map<number, Point>());
  const pinch = useRef<PinchState | null>(null);
  const safariGesture = useRef<PinchState | null>(null);
  const touchClickSuppressed = useRef(false);
  const drag = useRef<{
    pointerId: number;
    x: number;
    y: number;
    startX: number;
    startY: number;
  } | null>(null);

  const applyPan = useCallback((next: ScoreViewPan) => {
    panRef.current = next;
    setPan((current) => (current.x === next.x && current.y === next.y ? current : next));
  }, []);

  const panBy = useCallback(
    (deltaX: number, deltaY: number) => {
      applyPan({ x: panRef.current.x + deltaX, y: panRef.current.y + deltaY });
    },
    [applyPan],
  );

  const zoomOriginAt = useCallback(
    (clientX: number, clientY: number, target?: EventTarget | null): ZoomOrigin => {
      const page =
        (target instanceof Element ? target.closest<HTMLElement>('.page-host') : null) ??
        document.elementFromPoint(clientX, clientY)?.closest<HTMLElement>('.page-host') ??
        pageHosts.current.values().next().value ??
        null;
      const pageBounds = page?.getBoundingClientRect();
      return {
        page,
        fractionX: pageBounds ? (clientX - pageBounds.left) / Math.max(1, pageBounds.width) : 0,
        fractionY: pageBounds ? (clientY - pageBounds.top) / Math.max(1, pageBounds.height) : 0,
      };
    },
    [pageHosts],
  );

  const applyZoomAt = useCallback(
    (value: number, origin: ZoomOrigin, clientX: number, clientY: number) => {
      const next = clampZoom(value);
      if (next === zoomRef.current) return;
      zoomRef.current = next;
      pendingAnchor.current = { ...origin, clientX, clientY };
      onZoomChange(next);
    },
    [onZoomChange],
  );

  const startPinch = useCallback(() => {
    const [first, second] = touchPointers.current.values();
    if (!first || !second) return;
    const midpoint = pointMidpoint(first, second);
    pinch.current = {
      ...zoomOriginAt(midpoint.x, midpoint.y),
      startDistance: Math.max(1, pointDistance(first, second)),
      startZoom: zoomRef.current,
      lastX: midpoint.x,
      lastY: midpoint.y,
    };
  }, [zoomOriginAt]);

  useEffect(() => {
    zoomRef.current = zoom;
  }, [zoom]);

  useEffect(() => {
    const syncModifier = (event: KeyboardEvent) => setModifierDown(event.metaKey || event.ctrlKey);
    const clearModifier = () => setModifierDown(false);
    window.addEventListener('keydown', syncModifier);
    window.addEventListener('keyup', syncModifier);
    window.addEventListener('blur', clearModifier);
    return () => {
      window.removeEventListener('keydown', syncModifier);
      window.removeEventListener('keyup', syncModifier);
      window.removeEventListener('blur', clearModifier);
    };
  }, []);

  useEffect(() => {
    const node = viewport.current;
    if (!node) return;
    const handleWheel = (event: WheelEvent) => {
      event.preventDefault();
      if (drawingActive?.current) return;
      if (!event.metaKey && !event.ctrlKey) {
        const unit =
          event.deltaMode === WheelEvent.DOM_DELTA_LINE
            ? WHEEL_LINE_HEIGHT
            : event.deltaMode === WheelEvent.DOM_DELTA_PAGE
              ? node.clientHeight
              : 1;
        if (event.deltaX || event.deltaY) {
          panBy(-event.deltaX * unit, -event.deltaY * unit);
        }
        return;
      }
      const direction = event.deltaY < 0 ? 1 : event.deltaY > 0 ? -1 : 0;
      const largeStep = event.deltaMode !== 0 || Math.abs(event.deltaY) >= 24;
      if (!direction) return;
      const scaled = largeStep
        ? zoomRef.current * (direction > 0 ? WHEEL_ZOOM_FACTOR : 1 / WHEEL_ZOOM_FACTOR)
        : zoomRef.current * Math.exp(-event.deltaY * TRACKPAD_ZOOM_SENSITIVITY);
      applyZoomAt(
        scaled,
        zoomOriginAt(event.clientX, event.clientY, event.target),
        event.clientX,
        event.clientY,
      );
    };
    node.addEventListener('wheel', handleWheel, { passive: false });
    return () => node.removeEventListener('wheel', handleWheel);
  }, [applyZoomAt, drawingActive, panBy, zoomOriginAt]);

  useEffect(() => {
    const node = viewport.current;
    if (!node) return;
    const handleGestureStart = (raw: Event) => {
      if (drawingActive?.current) {
        raw.preventDefault();
        return;
      }
      if (!panEnabled) return;
      raw.preventDefault();
      if (touchPointers.current.size) return;
      const event = raw as SafariGestureEvent;
      const bounds = node.getBoundingClientRect();
      const clientX = event.clientX ?? bounds.left + bounds.width / 2;
      const clientY = event.clientY ?? bounds.top + bounds.height / 2;
      touchClickSuppressed.current = true;
      safariGesture.current = {
        ...zoomOriginAt(clientX, clientY, raw.target),
        startDistance: 1,
        startZoom: zoomRef.current,
        lastX: clientX,
        lastY: clientY,
      };
    };
    const handleGestureChange = (raw: Event) => {
      if (!panEnabled || drawingActive?.current) {
        raw.preventDefault();
        return;
      }
      raw.preventDefault();
      const gesture = safariGesture.current;
      if (!gesture) return;
      const event = raw as SafariGestureEvent;
      const scale = event.scale;
      if (!scale || !Number.isFinite(scale)) return;
      const bounds = node.getBoundingClientRect();
      const clientX = event.clientX ?? bounds.left + bounds.width / 2;
      const clientY = event.clientY ?? bounds.top + bounds.height / 2;
      panBy(clientX - gesture.lastX, clientY - gesture.lastY);
      gesture.lastX = clientX;
      gesture.lastY = clientY;
      applyZoomAt(gesture.startZoom * scale, gesture, clientX, clientY);
    };
    const handleGestureEnd = (event: Event) => {
      if (!panEnabled) return;
      event.preventDefault();
      safariGesture.current = null;
    };
    node.addEventListener('gesturestart', handleGestureStart);
    node.addEventListener('gesturechange', handleGestureChange);
    node.addEventListener('gestureend', handleGestureEnd);
    return () => {
      node.removeEventListener('gesturestart', handleGestureStart);
      node.removeEventListener('gesturechange', handleGestureChange);
      node.removeEventListener('gestureend', handleGestureEnd);
    };
  }, [applyZoomAt, drawingActive, panBy, panEnabled, zoomOriginAt]);

  const handlePointerDown = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      if (!panEnabled || event.button !== 0) return;
      if (isDirectPointer(event.pointerType)) {
        safariGesture.current = null;
        if (!touchPointers.current.size) {
          touchClickSuppressed.current = false;
        }
        const point = { x: event.clientX, y: event.clientY };
        touchPointers.current.set(event.pointerId, point);
        if (touchPointers.current.size === 1) {
          if (drawingActive?.current) return;
          drag.current = {
            pointerId: event.pointerId,
            ...point,
            startX: event.clientX,
            startY: event.clientY,
          };
          setDragging(true);
        } else if (touchPointers.current.size === 2) {
          touchClickSuppressed.current = true;
          drag.current = null;
          startPinch();
        }
        return;
      }
      if (drawingActive?.current) return;
      if (!event.metaKey && !event.ctrlKey) return;
      event.preventDefault();
      const node = event.currentTarget;
      drag.current = {
        pointerId: event.pointerId,
        x: event.clientX,
        y: event.clientY,
        startX: event.clientX,
        startY: event.clientY,
      };
      node.setPointerCapture(event.pointerId);
      setDragging(true);
    },
    [drawingActive, panEnabled, startPinch],
  );

  const handlePointerMove = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      if (isDirectPointer(event.pointerType)) {
        const point = touchPointers.current.get(event.pointerId);
        if (!point) return;
        point.x = event.clientX;
        point.y = event.clientY;
        if (drawingActive?.current) return;
        const gesture = pinch.current;
        if (gesture && touchPointers.current.size >= 2) {
          const [first, second] = touchPointers.current.values();
          if (!first || !second) return;
          const midpoint = pointMidpoint(first, second);
          panBy(midpoint.x - gesture.lastX, midpoint.y - gesture.lastY);
          gesture.lastX = midpoint.x;
          gesture.lastY = midpoint.y;
          applyZoomAt(
            gesture.startZoom * (pointDistance(first, second) / gesture.startDistance),
            gesture,
            midpoint.x,
            midpoint.y,
          );
          return;
        }
        const current = drag.current;
        if (!current || current.pointerId !== event.pointerId) return;
        if (
          Math.hypot(event.clientX - current.startX, event.clientY - current.startY) >
          TAP_PAN_THRESHOLD
        ) {
          touchClickSuppressed.current = true;
        }
        panBy(event.clientX - current.x, event.clientY - current.y);
        current.x = event.clientX;
        current.y = event.clientY;
        return;
      }
      if (drawingActive?.current) return;
      const current = drag.current;
      if (!current || current.pointerId !== event.pointerId) return;
      panBy(event.clientX - current.x, event.clientY - current.y);
      current.x = event.clientX;
      current.y = event.clientY;
    },
    [applyZoomAt, drawingActive, panBy],
  );

  const finishPointer = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>, cancelled: boolean) => {
      if (isDirectPointer(event.pointerType)) {
        if (!touchPointers.current.delete(event.pointerId)) return;
        if (cancelled) touchClickSuppressed.current = true;
        if (touchPointers.current.size === 1) {
          const [pointerId, point] = touchPointers.current.entries().next().value ?? [];
          pinch.current = null;
          if (pointerId !== undefined && point) {
            drag.current = {
              pointerId,
              ...point,
              startX: point.x,
              startY: point.y,
            };
          }
        } else if (touchPointers.current.size >= 2) {
          startPinch();
        } else {
          drag.current = null;
          pinch.current = null;
          setDragging(false);
        }
        return;
      }
      if (drag.current?.pointerId !== event.pointerId) return;
      drag.current = null;
      setDragging(false);
      if (event.currentTarget.hasPointerCapture(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId);
      }
    },
    [startPinch],
  );

  const handlePointerUp = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => finishPointer(event, false),
    [finishPointer],
  );
  const handlePointerCancel = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => finishPointer(event, true),
    [finishPointer],
  );

  const consumeClick = useCallback(() => {
    const suppressed = touchClickSuppressed.current;
    touchClickSuppressed.current = false;
    return suppressed;
  }, []);

  useLayoutEffect(() => {
    const anchor = pendingAnchor.current;
    if (!anchor) return;
    pendingAnchor.current = null;
    const pageBounds = anchor.page?.isConnected ? anchor.page.getBoundingClientRect() : null;
    if (!pageBounds) return;
    const deltaX = pageBounds.left + pageBounds.width * anchor.fractionX - anchor.clientX;
    const deltaY = pageBounds.top + pageBounds.height * anchor.fractionY - anchor.clientY;
    if (deltaX || deltaY) {
      applyPan({ x: panRef.current.x - deltaX, y: panRef.current.y - deltaY });
    }
  }, [applyPan, zoom]);

  const firstLayout = useRef(true);
  useLayoutEffect(() => {
    if (firstLayout.current) {
      firstLayout.current = false;
      return;
    }
    applyPan({ x: 0, y: 0 });
  }, [applyPan, layout]);

  useLayoutEffect(() => {
    if (!ready || !viewKey || restoredView.current === viewKey) return;
    applyPan(initialPan);
    restoredView.current = viewKey;
  }, [applyPan, initialPan.x, initialPan.y, ready, viewKey]);

  return {
    viewport,
    dragging,
    modifierDown,
    pan,
    panBy,
    gesture: {
      resetDirectPointers: () => {
        touchPointers.current.clear();
        drag.current = null;
        pinch.current = null;
        setDragging(false);
      },
      onPointerDown: handlePointerDown,
      onPointerMove: handlePointerMove,
      onPointerUp: handlePointerUp,
      onPointerCancel: handlePointerCancel,
      consumeClick,
    },
  };
}
