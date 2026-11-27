import { memo, useEffect, useId, useLayoutEffect, useRef, useState } from 'react';

import { parseScoreSvg, playbackLayer, scorePainter } from '../utils/scoreSvg.ts';

const EMPTY_PLAYBACK = new Set<string>();

interface Props {
  source: string;
  identity: string;
  label: string;
  graphicIds: Set<string>;
  annotationColors?: ReadonlyMap<string, string>;
  playbackIds?: Set<string>;
  onError: (error: unknown) => void;
  onReady: () => void;
}

/** Own the sanitized drawing DOM independently of React's selection overlay. */
export const ScoreSvg = memo(function ScoreSvg({
  source,
  identity,
  label,
  graphicIds,
  annotationColors,
  playbackIds = EMPTY_PLAYBACK,
  onError,
  onReady,
}: Props) {
  const host = useRef<HTMLDivElement>(null);
  const painter = useRef<ReturnType<typeof scorePainter> | null>(null);
  const playback = useRef<ReturnType<typeof playbackLayer> | null>(null);
  const [prepared, setPrepared] = useState<{
    identity: string;
    source: string;
    svg: SVGSVGElement;
  } | null>(null);
  const prefix = useId().replace(/[^\w-]/g, '') + '-';
  const currentLabel = useRef(label);
  useLayoutEffect(() => {
    currentLabel.current = label;
    host.current?.firstElementChild?.setAttribute('aria-label', label);
  }, [label]);
  const paint = useRef({ graphicIds, annotationColors, playbackIds });
  useLayoutEffect(() => {
    paint.current = { graphicIds, annotationColors, playbackIds };
  });
  useLayoutEffect(() => {
    painter.current?.(graphicIds, annotationColors ?? new Map());
  }, [graphicIds, annotationColors]);
  // Playback highlights change on every audition, so they never restyle the page drawing.
  useLayoutEffect(() => {
    playback.current?.paint(playbackIds);
  }, [playbackIds]);
  useEffect(() => {
    const controller = new AbortController();
    void parseScoreSvg(source, prefix, controller.signal)
      .then((svg) => {
        if (!controller.signal.aborted) setPrepared({ identity, source, svg });
      })
      .catch((error: unknown) => {
        if (!controller.signal.aborted) onError(error);
      });
    return () => controller.abort();
  }, [source, identity, prefix, onError]);
  useLayoutEffect(() => {
    if (
      !prepared ||
      prepared.identity !== identity ||
      prepared.source !== source ||
      host.current?.firstElementChild === prepared.svg
    )
      return;
    const svg = prepared.svg;
    svg.setAttribute('role', 'img');
    svg.setAttribute('aria-label', currentLabel.current);
    painter.current = scorePainter(svg);
    playback.current = playbackLayer(svg);
    host.current!.replaceChildren(svg, playback.current.element);
    const current = paint.current;
    painter.current(current.graphicIds, current.annotationColors ?? new Map());
    playback.current.paint(current.playbackIds);
    // Geometry and page dimensions commit before the browser can paint this replacement.
    onReady();
  }, [prepared, identity, source, onReady]);
  useLayoutEffect(
    () => () => {
      painter.current = null;
      playback.current = null;
      host.current?.replaceChildren();
    },
    [],
  );
  return <div className="score-drawing" ref={host} />;
});
