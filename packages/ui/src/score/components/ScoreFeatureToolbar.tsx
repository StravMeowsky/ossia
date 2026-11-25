import {
  LoaderCircle,
  Maximize2,
  Minimize2,
  Pause,
  Play,
  Pencil,
  RotateCcw,
  SquareDashedMousePointer,
} from 'lucide-react';
import type { RefObject } from 'react';

import type { DrawingColor } from '../../generated/protocol.ts';
import { useI18n } from '../../i18n.tsx';
import type { MessageKey } from '../../i18n.tsx';
import type { DrawingMode } from '../hooks/useDrawingInput.ts';
import type { PlaybackPhase } from '../hooks/useScorePlayback.ts';
import { resolveAnnotationModifier } from '../utils/annotationInput.ts';
import { DRAWING_COLORS, DRAWING_PALETTE, ERASER_SIZES, PEN_SIZES } from '../utils/drawing.ts';
import type { EraserScope } from '../utils/drawing.ts';

const COLOR_LABELS: Record<DrawingColor, MessageKey> = {
  black: 'drawingColorBlack',
  blue: 'drawingColorBlue',
  green: 'drawingColorGreen',
  yellow: 'drawingColorYellow',
  red: 'drawingColorRed',
};

const ERASER_BODY =
  'M21 21H8a2 2 0 0 1-1.42-.587l-3.994-3.999a2 2 0 0 1 0-2.828l10-10a2 2 0 0 1 2.829 0l5.999 6a2 2 0 0 1 0 2.828L12.834 21M5.082 11.09l8.828 8.828';

/** Lucide-style eraser above a line: cut in the middle (partial) or a whole squiggle (stroke). */
function EraserScopeIcon({ scope }: { scope: EraserScope }) {
  return (
    <svg
      aria-hidden="true"
      width={17}
      height={17}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <g transform="translate(4.5 0.5) scale(0.62)">
        <path d={ERASER_BODY} vectorEffect="non-scaling-stroke" />
      </g>
      {scope === 'partial' ? (
        <path d="M3 20h4M16 20h5" />
      ) : (
        <path d="M3 20c1.5-2 3-2 4.5 0s3 2 4.5 0 3-2 4.5 0 3 2 4.5 0" />
      )}
    </svg>
  );
}

export function ScoreFeatureToolbar({
  editing,
  annotationEnabled,
  annotationAvailable,
  onToggleAnnotation,
  playbackPhase,
  playbackAvailable,
  playbackUnavailableReason,
  onTogglePlayback,
  onRestoreVersion,
  fullscreen,
  onToggleFullscreen,
  fullscreenButtonRef,
  floating,
  drawingMode,
  sizingMode,
  onDrawingMode,
  drawingAvailable,
  penSize,
  eraserSize,
  onPenSize,
  onEraserSize,
  eraserScope,
  onEraserScope,
  penColor,
  onPenColor,
}: {
  editing: boolean;
  annotationEnabled: boolean;
  annotationAvailable: boolean;
  onToggleAnnotation: () => void;
  playbackPhase: PlaybackPhase;
  playbackAvailable: boolean;
  playbackUnavailableReason: string | null;
  onTogglePlayback: () => void;
  onRestoreVersion?: () => void;
  fullscreen: boolean;
  onToggleFullscreen: () => void;
  fullscreenButtonRef: RefObject<HTMLButtonElement | null>;
  floating: boolean;
  drawingMode: DrawingMode;
  sizingMode: DrawingMode;
  onDrawingMode: (mode: DrawingMode) => void;
  drawingAvailable: boolean;
  penSize: number;
  eraserSize: number;
  onPenSize: (size: number) => void;
  onEraserSize: (size: number) => void;
  eraserScope: EraserScope;
  onEraserScope: (scope: EraserScope) => void;
  penColor: DrawingColor;
  onPenColor: (color: DrawingColor) => void;
}) {
  const { t } = useI18n();
  const modifier = resolveAnnotationModifier();
  const label = annotationEnabled ? t('annotationClose') : t('annotationOpen');
  const tooltip = annotationEnabled
    ? t('annotationCloseTooltip', { modifier })
    : t('annotationOpenTooltip', { modifier });
  const playing = playbackPhase === 'playing';
  const loading = playbackPhase === 'loading';
  const lockedTooltip = editing ? t('scoreEditingToolsUnavailable') : undefined;
  const playbackReason = playbackUnavailableReason?.includes('unsupported Scheme reader syntax')
    ? t('playbackUnsupportedScheme')
    : playbackUnavailableReason?.includes('playback does not support LilyPond \\book')
      ? t('playbackUnsupportedBook')
      : null;
  const playbackLabel = loading
    ? t('playbackLoading')
    : playing
      ? t('playbackPause')
      : t('playbackPlay');
  const playbackTooltip =
    lockedTooltip ??
    (loading
      ? t('playbackLoading')
      : !playbackAvailable
        ? (playbackReason ?? t('playbackUnavailable'))
        : playing
          ? t('playbackPauseTooltip')
          : t('playbackPlayTooltip'));
  const playbackDisabled = editing || !playbackAvailable || loading;
  // The whole-line eraser has no size choice, so it shows no options panel.
  const drawingOptions =
    sizingMode && drawingAvailable && !(sizingMode === 'eraser' && eraserScope === 'stroke');
  return (
    <div
      className={`score-feature-toolbar${floating ? ' is-floating' : ''}`}
      aria-label={t('scoreTools')}
    >
      <button
        type="button"
        className="score-feature-toggle score-playback-toggle"
        aria-label={playbackDisabled ? playbackTooltip : playbackLabel}
        aria-pressed={playing}
        aria-disabled={playbackDisabled}
        data-tooltip={playbackTooltip}
        onClick={() => {
          if (!playbackDisabled) onTogglePlayback();
        }}
      >
        {loading ? (
          <LoaderCircle className="score-playback-spinner" aria-hidden="true" size={17} />
        ) : playing ? (
          <Pause aria-hidden="true" size={17} fill="currentColor" strokeWidth={1.8} />
        ) : (
          <Play aria-hidden="true" size={17} fill="currentColor" strokeWidth={1.8} />
        )}
      </button>
      {(annotationAvailable || editing) && (
        <button
          type="button"
          className="score-feature-toggle score-annotation-toggle"
          aria-label={label}
          aria-pressed={annotationEnabled}
          aria-disabled={editing}
          data-tooltip={lockedTooltip ?? tooltip}
          onClick={() => {
            if (!editing) onToggleAnnotation();
          }}
        >
          <SquareDashedMousePointer aria-hidden="true" size={17} strokeWidth={2.1} />
        </button>
      )}
      {(drawingAvailable || editing) &&
        (['pen', 'eraser'] as const).map((mode) => {
          const label = mode === 'pen' ? t('drawingPen') : t('drawingEraser');
          return (
            <button
              key={mode}
              type="button"
              className="score-feature-toggle"
              aria-label={label}
              aria-pressed={sizingMode === mode}
              aria-disabled={editing}
              data-tooltip={
                lockedTooltip ??
                (sizingMode ? undefined : `${label} (${mode === 'pen' ? 'D' : 'R'})`)
              }
              data-eraser-scope={mode === 'eraser' ? eraserScope : undefined}
              onClick={() => {
                if (editing) return;
                // An active eraser switches between whole-line and partial erasing;
                // Escape, R or the pen leaves it.
                if (mode === 'eraser' && drawingMode === 'eraser') {
                  onEraserScope(eraserScope === 'stroke' ? 'partial' : 'stroke');
                } else onDrawingMode(drawingMode === mode ? null : mode);
              }}
            >
              {mode === 'pen' ? (
                <Pencil aria-hidden="true" size={17} />
              ) : (
                <EraserScopeIcon scope={eraserScope} />
              )}
            </button>
          );
        })}
      <button
        ref={fullscreenButtonRef}
        type="button"
        className="score-feature-toggle score-fullscreen-toggle"
        aria-label={fullscreen ? t('exitFullscreen') : t('fullscreenAction')}
        aria-pressed={fullscreen}
        data-tooltip={fullscreen ? t('exitFullscreen') : t('fullscreenAction')}
        onClick={onToggleFullscreen}
      >
        {fullscreen ? (
          <Minimize2 aria-hidden="true" size={17} strokeWidth={2.1} />
        ) : (
          <Maximize2 aria-hidden="true" size={17} strokeWidth={2.1} />
        )}
      </button>
      {onRestoreVersion && (
        <button
          type="button"
          className="score-feature-toggle score-restore-toggle"
          aria-label={t('restoreVersion')}
          data-tooltip={t('restoreVersion')}
          onClick={onRestoreVersion}
        >
          <RotateCcw aria-hidden="true" size={17} strokeWidth={2.1} />
        </button>
      )}
      {drawingOptions && (
        <div className="score-drawing-options">
          <div
            className="score-drawing-sizes"
            role="group"
            aria-label={sizingMode === 'pen' ? t('drawingPenSize') : t('drawingEraserSize')}
          >
            {(sizingMode === 'pen' ? PEN_SIZES : ERASER_SIZES).map((size, index) => (
              <button
                key={size}
                type="button"
                className="score-drawing-size"
                aria-label={`${sizingMode === 'pen' ? t('drawingPenSize') : t('drawingEraserSize')} ${index + 1}`}
                aria-pressed={(sizingMode === 'pen' ? penSize : eraserSize) === size}
                onClick={() => (sizingMode === 'pen' ? onPenSize(size) : onEraserSize(size))}
              >
                <span style={{ width: 4 + index * 4, height: 4 + index * 4 }} />
              </button>
            ))}
          </div>
          {sizingMode === 'pen' && (
            <div className="score-drawing-colors" role="group" aria-label={t('drawingColor')}>
              {DRAWING_PALETTE.map((color) => (
                <button
                  key={color}
                  type="button"
                  className="score-drawing-color"
                  aria-label={t(COLOR_LABELS[color])}
                  aria-pressed={penColor === color}
                  onClick={() => onPenColor(color)}
                >
                  <span
                    style={{ backgroundColor: DRAWING_COLORS[color], color: DRAWING_COLORS[color] }}
                  />
                </button>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
