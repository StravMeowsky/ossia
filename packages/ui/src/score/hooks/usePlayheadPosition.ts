import { useEffect, useMemo, useState } from 'react';

import type { GeometryPage, PlaybackSystemSpan } from '../../generated/protocol.ts';
import { locatePlayhead } from '../utils/playheadPosition.ts';

interface LocatedPlayhead {
  span: PlaybackSystemSpan;
  x: number;
}

/** Locate the current note in rendered geometry, retaining the last known position. */
export function usePlayheadPosition(
  renderKey: string,
  pages: GeometryPage[],
  systems: PlaybackSystemSpan[],
  elementIds: Set<string>,
  active: boolean,
) {
  const [last, setLast] = useState<{ renderKey: string; position: LocatedPlayhead } | null>(null);
  const located = useMemo(() => {
    if (!active || !elementIds.size) return null;
    return locatePlayhead(pages, systems, elementIds);
  }, [pages, systems, elementIds, active]);

  useEffect(() => {
    if (!active) {
      setLast(null);
    } else if (located) {
      setLast((previous) =>
        previous?.renderKey === renderKey &&
        previous.position.x === located.x &&
        previous.position.span.page === located.span.page &&
        previous.position.span.systemIndex === located.span.systemIndex
          ? previous
          : { renderKey, position: located },
      );
    }
  }, [active, located, renderKey]);
  return active ? (located ?? (last?.renderKey === renderKey ? last.position : null)) : null;
}
