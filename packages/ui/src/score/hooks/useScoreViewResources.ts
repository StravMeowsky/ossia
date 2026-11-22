import { useEffect, useMemo, useState } from 'react';

import { describe } from '../../errors.ts';
import type { RenderGeometry, RenderReference } from '../../generated/protocol.ts';
import type { Api } from '../utils/api.ts';
import { loadRenderGeometry, peekRenderGeometry } from '../utils/renderCache.ts';

/** Retains the displayed geometry until its replacement has loaded and validated. */
export function useScoreViewResources(api: Api, render: RenderReference) {
  const [result, setResult] = useState<{
    render: RenderReference;
    geometry: RenderGeometry | null;
  }>({
    render,
    geometry: peekRenderGeometry(render.renderKey) ?? null,
  });
  const [error, setError] = useState<string | null>(null);
  const geometry = result.geometry;
  const pages = useMemo(
    () => [...result.render.manifest.pages].sort((left, right) => left.page - right.page),
    [result.render.manifest.pages],
  );
  useEffect(() => {
    let live = true;
    setError(null);
    loadRenderGeometry(api, render.renderKey)
      .then((geometry) => {
        if (!live) return;
        if (
          geometry.renderKey !== render.renderKey ||
          geometry.sourceHash !== render.manifest.sourceHash
        ) {
          throw new Error('Render geometry identity mismatch');
        }
        if (
          render.manifest.pages.some(
            (page) => !geometry.pages.find((layout) => layout.page === page.page)?.viewBox,
          )
        ) {
          throw new Error('Missing page geometry');
        }
        setResult({ render, geometry });
      })
      .catch((reason) => {
        if (live) setError(describe(reason));
      });
    return () => {
      live = false;
    };
  }, [api, render.renderKey]);
  const layouts = useMemo(() => {
    const byPage = new Map(geometry?.pages.map((page) => [page.page, page]));
    return pages.map((page) => ({ page: page.page, layout: byPage.get(page.page) }));
  }, [geometry, pages]);
  const missingGeometry = !!geometry && layouts.some((page) => !page.layout?.viewBox);
  const ready = !!geometry && !missingGeometry && layouts.length > 0;
  return { geometry, error, layouts, missingGeometry, ready };
}
