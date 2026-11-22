import { useEffect } from 'react';

const GESTURE_EVENTS = [
  'gesturestart',
  'gesturechange',
  'gestureend',
  'touchstart',
  'touchmove',
  'touchend',
  'touchcancel',
] as const;

function isMultiTouch(event: Event) {
  const touchEvent = event as TouchEvent;
  return touchEvent.touches.length > 1 || touchEvent.changedTouches.length > 1;
}

function isBackdropTouch(event: Event) {
  if (!(event.target instanceof HTMLDialogElement)) return false;
  const touch = (event as TouchEvent).changedTouches[0];
  if (!touch) return false;
  const bounds = event.target.getBoundingClientRect();
  return (
    touch.clientX < bounds.left ||
    touch.clientX > bounds.right ||
    touch.clientY < bounds.top ||
    touch.clientY > bounds.bottom
  );
}

function scrollableAncestor(target: EventTarget | null, dialog: HTMLDialogElement) {
  if (!(target instanceof Element)) return null;
  for (let element: Element | null = target; element && dialog.contains(element);) {
    const overflowY = getComputedStyle(element).overflowY;
    if (
      (overflowY === 'auto' || overflowY === 'scroll') &&
      element.scrollHeight > element.clientHeight
    )
      return element;
    element = element.parentElement;
  }
  return null;
}

/** Prevents Safari page gestures while a modal dialog is open. */
export function useModalGestureLock() {
  useEffect(() => {
    let scrollGesture: {
      element: Element | null;
      startY: number;
      startScrollTop: number;
    } | null = null;
    const preventPageGesture = (event: Event) => {
      const dialog = document.querySelector<HTMLDialogElement>('dialog[open]');
      if (!dialog) return;
      if (!event.type.startsWith('touch')) {
        if (event.cancelable) event.preventDefault();
        return;
      }
      if (isMultiTouch(event) || isBackdropTouch(event)) {
        if (event.cancelable) event.preventDefault();
        if (event.type === 'touchend' || event.type === 'touchcancel') scrollGesture = null;
        return;
      }
      const touch = (event as TouchEvent).touches[0] ?? (event as TouchEvent).changedTouches[0];
      if (!touch) return;
      if (event.type === 'touchstart') {
        const element = scrollableAncestor(event.target, dialog);
        scrollGesture = { element, startY: touch.clientY, startScrollTop: element?.scrollTop ?? 0 };
        return;
      }
      if (event.type === 'touchmove') {
        if (event.cancelable) event.preventDefault();
        if (scrollGesture?.element)
          scrollGesture.element.scrollTop =
            scrollGesture.startScrollTop + scrollGesture.startY - touch.clientY;
        return;
      }
      if (event.type === 'touchend' || event.type === 'touchcancel') scrollGesture = null;
    };
    for (const type of GESTURE_EVENTS) {
      document.addEventListener(type, preventPageGesture, { capture: true, passive: false });
    }
    return () => {
      for (const type of GESTURE_EVENTS) {
        document.removeEventListener(type, preventPageGesture, { capture: true });
      }
    };
  }, []);
}
