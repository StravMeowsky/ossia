export const PAGE_GAP = 24;
export const CANVAS_PADDING = 32;
export const CANVAS_TOP_PADDING = 32;
// Page layout is viewport-independent; zoom is the only size control.
export const BASE_PAGE_WIDTH = 900;
export const MIN_ZOOM = 25;
export const MAX_ZOOM = 400;

export type ScoreLayout = 'vertical' | 'horizontal';

export function clampZoom(zoom: number) {
  return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom));
}
