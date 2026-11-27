import { memo, useCallback, useEffect, useMemo, useState } from 'react';
import type { CSSProperties, MouseEvent, PointerEvent } from 'react';

import { describe } from '../../errors.ts';
import type {
  AnnotationBox,
  GeometryBounds,
  GeometryPage,
  PlaybackSystemSpan,
  SelectionAnnotation,
  SelectionTarget,
} from '../../generated/protocol.ts';
import { useI18n } from '../../i18n.tsx';
import { annotationColor } from '../utils/annotationColors.ts';
import { annotationOccurrences, draftOccurrences } from '../utils/annotationGeometry.ts';
import type { Api } from '../utils/api.ts';
import { selectionKey } from '../utils/api.ts';
import { cachedRenderPages, loadRenderPage } from '../utils/renderCache.ts';
import {
  selectedBounds,
  selectedElementBounds,
  selectedGraphics,
} from '../utils/selectionGeometry.ts';
import { ScoreAnnotationLayer } from './ScoreAnnotationLayer.tsx';
import { ScoreLoading } from './ScoreLoading.tsx';
import { ScoreSvg } from './ScoreSvg.tsx';

/** Stable page callbacks keep canvas motion separate from page drawing updates. */
export interface ScoreSheetActions {
  pointerDown: (layout: GeometryPage, event: PointerEvent<SVGSVGElement>) => void;
  pointerMove: (layout: GeometryPage, event: PointerEvent<SVGSVGElement>) => void;
  click: (layout: GeometryPage, event: MouseEvent<SVGSVGElement>) => void;
  open: (annotation: SelectionAnnotation, bounds: GeometryBounds, page: number) => void;
}

interface Props {
  api: Api;
  renderKey: string;
  pageLayout: GeometryPage;
  priority: number;
  selectionKeys: Set<string>;
  graphicColors: ReadonlyMap<string, string>;
  activeElementIds: Set<string>;
  playhead: { span: PlaybackSystemSpan; x: number } | null;
  annotations: SelectionAnnotation[];
  previewTargets: SelectionTarget[] | undefined;
  previewBoxes: AnnotationBox[] | undefined;
  previewIndex: number;
  draftActive: boolean;
  interactive: boolean;
  showHitBoxes: boolean;
  agentEditing: boolean;
  zoom: number;
  actions: ScoreSheetActions;
  onPageReady: (layout: GeometryPage) => void;
}
const SELECTION_STROKE_WIDTH = 0.25;
const EMPTY_IDS = new Set<string>();
const EMPTY_COLORS = new Map<string, string>();
interface DrawingSnapshot {
  key: string;
  source: string;
  layout: GeometryPage;
}

/** Mounts drawing and interaction work only while its page belongs to the visible window. */
export const ScoreSheet = memo(function ScoreSheet({
  api,
  renderKey,
  pageLayout,
  priority,
  selectionKeys,
  graphicColors,
  activeElementIds,
  playhead,
  annotations,
  previewTargets,
  previewBoxes,
  previewIndex,
  draftActive,
  interactive,
  showHitBoxes,
  agentEditing,
  zoom,
  actions,
  onPageReady,
}: Props) {
  const { t } = useI18n();
  const page = pageLayout.page;
  const [candidate, setCandidate] = useState<DrawingSnapshot | null>(() => {
    const source = cachedRenderPages(renderKey, [page])[page];
    return source === undefined ? null : { key: renderKey, source, layout: pageLayout };
  });
  const [displayed, setDisplayed] = useState<DrawingSnapshot | null>(null);
  const drawingReady = displayed !== null;
  const currentDrawing = displayed?.key === renderKey;
  const displayLayout = displayed?.layout ?? pageLayout;
  const box = displayLayout.viewBox!;
  const [error, setError] = useState<string | null>(null);
  const svgError = useCallback((reason: unknown) => setError(describe(reason)), []);
  const onReady = useCallback(() => {
    if (!candidate) return;
    setDisplayed(candidate);
    onPageReady(candidate.layout);
  }, [candidate, onPageReady]);
  useEffect(() => {
    const controller = new AbortController();
    setError(null);
    void loadRenderPage(api, renderKey, page, controller.signal, priority)
      .then((value) => {
        if (!controller.signal.aborted)
          setCandidate((previous) =>
            previous?.key === renderKey && previous.source === value
              ? previous
              : { key: renderKey, source: value, layout: pageLayout },
          );
      })
      .catch((reason: unknown) => {
        if (!controller.signal.aborted) svgError(reason);
      });
    return () => controller.abort();
  }, [api, renderKey, page, pageLayout, priority, svgError]);
  const selectedGraphicIds = useMemo(
    () => selectedGraphics(displayLayout, selectionKeys),
    [displayLayout, selectionKeys],
  );
  const playbackGraphicIds = useMemo(
    () =>
      selectedGraphics(
        displayLayout,
        new Set(
          [...activeElementIds].map((elementId) => selectionKey({ kind: 'element', elementId })),
        ),
      ),
    [displayLayout, activeElementIds],
  );
  const measureBoxes = useMemo(() => {
    const measureBoxes = annotationOccurrences(displayLayout, annotations)
      .filter(({ target }) => target?.kind === 'measure')
      .map(({ annotation, bounds, index }) => ({
        key: `${annotation.id}:${index}:${bounds.x0}:${bounds.y0}`,
        bounds,
        index,
      }));
    if (previewTargets) {
      measureBoxes.push(
        ...draftOccurrences(displayLayout, previewTargets, previewIndex)
          .filter(({ target }) => target?.kind === 'measure')
          .map(({ bounds, index }) => ({
            key: `draft:${index}:${bounds.x0}:${bounds.y0}`,
            bounds,
            index,
          })),
      );
    }
    return measureBoxes;
  }, [displayLayout, annotations, previewTargets, previewIndex]);
  const selectedRegions = useMemo(
    () => selectedBounds(displayLayout, selectionKeys),
    [displayLayout, selectionKeys],
  );
  const selectedElements = useMemo(
    () => (showHitBoxes ? selectedElementBounds(displayLayout, selectionKeys) : []),
    [displayLayout, selectionKeys, showHitBoxes],
  );
  const annotationLayer = useMemo(
    () =>
      annotations.length > 0 || draftActive || previewBoxes?.length ? (
        <ScoreAnnotationLayer
          annotations={annotations}
          draftBoxes={previewBoxes}
          draftIndex={draftActive || previewBoxes?.length ? annotations.length : undefined}
          draftTargets={previewTargets}
          layout={displayLayout}
          onOpen={(annotation, bounds) => actions.open(annotation, bounds, page)}
          scale={zoom / 100}
        />
      ) : null,
    [annotations, draftActive, previewBoxes, previewTargets, displayLayout, actions, page, zoom],
  );
  return (
    <>
      {error && (
        <p className="error score-page-error" role="alert">
          {t('scoreLoadFailed', { error })}
        </p>
      )}
      {candidate && (
        <ScoreSvg
          source={candidate.source}
          identity={candidate.key}
          label={t('scoreAlt', { page })}
          graphicIds={currentDrawing ? selectedGraphicIds : EMPTY_IDS}
          annotationColors={currentDrawing ? graphicColors : EMPTY_COLORS}
          playbackIds={currentDrawing ? playbackGraphicIds : EMPTY_IDS}
          onError={svgError}
          onReady={onReady}
        />
      )}
      {!drawingReady && !error && <ScoreLoading label={t('loadingScore')} />}
      <svg
        className={`overlay ${currentDrawing && interactive ? 'interactive' : ''}`}
        viewBox={`${box.x0} ${box.y0} ${box.x1 - box.x0} ${box.y1 - box.y0}`}
        aria-hidden="true"
        onPointerDown={(event) => {
          if (currentDrawing && interactive) actions.pointerDown(displayLayout, event);
        }}
        onPointerMove={(event) => {
          if (currentDrawing && interactive) actions.pointerMove(displayLayout, event);
        }}
        onPointerLeave={(event) => event.currentTarget.classList.remove('is-over-note')}
        onClick={(event) => {
          if (currentDrawing && interactive) actions.click(displayLayout, event);
        }}
      >
        {selectedRegions.map(({ bounds }, index) => (
          <rect
            key={index}
            className="selection-box"
            strokeWidth={SELECTION_STROKE_WIDTH}
            x={bounds.x0 + SELECTION_STROKE_WIDTH / 2}
            y={bounds.y0}
            width={Math.max(0, bounds.x1 - bounds.x0 - SELECTION_STROKE_WIDTH)}
            height={bounds.y1 - bounds.y0}
          />
        ))}
        {measureBoxes.map(({ key, bounds, index }) => (
          <rect
            key={key}
            className="annotation-box"
            strokeWidth={SELECTION_STROKE_WIDTH}
            x={bounds.x0 + SELECTION_STROKE_WIDTH / 2}
            y={bounds.y0}
            width={Math.max(0, bounds.x1 - bounds.x0 - SELECTION_STROKE_WIDTH)}
            height={bounds.y1 - bounds.y0}
            style={
              {
                '--annotation-color': annotationColor(index),
              } as CSSProperties
            }
          />
        ))}
        {playhead?.span.page === page && (
          <line
            className="score-playhead"
            x1={playhead.x}
            x2={playhead.x}
            y1={playhead.span.y0}
            y2={playhead.span.y1}
          />
        )}
        {showHitBoxes &&
          selectedElements.map(({ bounds }, index) => (
            <rect
              key={index}
              className="hit-box"
              x={bounds.x0}
              y={bounds.y0}
              width={Math.max(0, bounds.x1 - bounds.x0)}
              height={Math.max(0, bounds.y1 - bounds.y0)}
            />
          ))}
      </svg>
      {annotationLayer}
      {agentEditing && (
        <div className="agent-edit-frame" aria-hidden="true">
          <span className="agent-edit-frame-ring" />
        </div>
      )}
    </>
  );
});
