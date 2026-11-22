import type { DrawingStroke } from '../../generated/protocol.ts';
import { DRAWING_COLORS } from '../utils/drawing.ts';

// WebKit can miss repaints when drawing paths use very small SVG coordinates.
const VIEW_WIDTH = 1000;

export function DrawingLayer({
  strokes,
  page,
  aspect,
}: {
  strokes: DrawingStroke[];
  page: number;
  aspect: number;
}) {
  return (
    <svg
      className="score-freehand-layer"
      viewBox={`0 0 ${VIEW_WIDTH} ${aspect * VIEW_WIDTH}`}
      preserveAspectRatio="none"
      aria-hidden="true"
    >
      {strokes
        .filter((stroke) => stroke.page === page)
        .map((stroke, index) => {
          const points = stroke.points
            .map((point) => `${point.x * VIEW_WIDTH},${point.y * aspect * VIEW_WIDTH}`)
            .join(' ');
          const color = DRAWING_COLORS[stroke.color ?? 'black'];
          return stroke.points.length === 1 ? (
            <circle
              key={index}
              cx={stroke.points[0]!.x * VIEW_WIDTH}
              cy={stroke.points[0]!.y * aspect * VIEW_WIDTH}
              r={(stroke.width * VIEW_WIDTH) / 2}
              fill={color}
            />
          ) : (
            <polyline
              key={index}
              points={points}
              fill="none"
              stroke={color}
              strokeWidth={stroke.width * VIEW_WIDTH}
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          );
        })}
    </svg>
  );
}
