import type { DrawingColor, DrawingPoint, DrawingStroke } from '../../generated/protocol.ts';

export const PEN_SIZES = [2, 4, 7, 12];
export const ERASER_SIZES = [12, 20, 32, 48];
export const DRAWING_COLORS: Record<DrawingColor, string> = {
  black: '#000000',
  blue: '#007aff',
  green: '#34c759',
  yellow: '#ffcc00',
  red: '#ff3b30',
};
export const DRAWING_PALETTE: DrawingColor[] = ['black', 'blue', 'green', 'yellow', 'red'];

/** Remove portions of a stroke within a circular eraser in page-width units. */
export function eraseStrokes(
  strokes: DrawingStroke[],
  page: number,
  center: DrawingPoint,
  radius: number,
  aspect: number,
) {
  const result: DrawingStroke[] = [];
  for (const stroke of strokes) {
    if (stroke.page !== page) {
      result.push(stroke);
      continue;
    }
    const r = radius + stroke.width / 2;
    const inside = (p: DrawingPoint) => Math.hypot(p.x - center.x, (p.y - center.y) * aspect) <= r;
    if (stroke.points.length === 1) {
      if (!inside(stroke.points[0]!)) result.push(stroke);
      continue;
    }
    let fragment: DrawingPoint[] = [];
    const flush = () => {
      if (fragment.length) result.push({ ...stroke, points: fragment });
      fragment = [];
    };
    for (let i = 1; i < stroke.points.length; i++) {
      const a = stroke.points[i - 1]!;
      const b = stroke.points[i]!;
      const dx = b.x - a.x;
      const dy = (b.y - a.y) * aspect;
      const ax = a.x - center.x;
      const ay = (a.y - center.y) * aspect;
      const aa = dx * dx + dy * dy;
      const bb = 2 * (ax * dx + ay * dy);
      const cc = ax * ax + ay * ay - r * r;
      const discriminant = bb * bb - 4 * aa * cc;
      const cuts = [0, 1];
      if (aa > 0 && discriminant > 0) {
        const root = Math.sqrt(discriminant);
        for (const t of [(-bb - root) / (2 * aa), (-bb + root) / (2 * aa)]) {
          if (t > 0 && t < 1) cuts.push(t);
        }
      }
      cuts.sort((x, y) => x - y);
      const at = (t: number) => ({ x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t });
      for (let j = 1; j < cuts.length; j++) {
        const start = cuts[j - 1]!;
        const end = cuts[j]!;
        if (inside(at((start + end) / 2))) {
          flush();
        } else {
          if (!fragment.length) fragment.push(at(start));
          fragment.push(at(end));
        }
      }
    }
    flush();
  }
  return result;
}
