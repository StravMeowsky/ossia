import { useLayoutEffect, useMemo, useState } from 'react';
import type { RefObject } from 'react';

/** Tracks page visibility while leaving every page's layout host in the canvas. */
export function useVisibleScorePages(
  viewport: RefObject<HTMLDivElement | null>,
  hosts: RefObject<Map<number, HTMLDivElement>>,
  pageNumbers: readonly number[],
  renderKey: string,
  ready: boolean,
  pinnedPage: number | undefined,
) {
  const [state, setState] = useState<{ key: string; pages: Set<number> }>({
    key: renderKey,
    pages: new Set(),
  });
  useLayoutEffect(() => {
    if (!ready || !viewport.current) return;
    let live = true;
    const numbers = new Map([...hosts.current].map(([page, host]) => [host, page]));
    const visible = new Set<number>();
    const observer = new IntersectionObserver(
      (entries) => {
        if (!live) return;
        for (const entry of entries) {
          const page = numbers.get(entry.target as HTMLDivElement);
          if (page === undefined) continue;
          if (
            entry.isIntersecting &&
            entry.intersectionRect.width > 0 &&
            entry.intersectionRect.height > 0
          )
            visible.add(page);
          else visible.delete(page);
        }
        setState((previous) =>
          previous.key === renderKey &&
          previous.pages.size === visible.size &&
          [...visible].every((page) => previous.pages.has(page))
            ? previous
            : { key: renderKey, pages: new Set(visible) },
        );
      },
      { root: viewport.current },
    );
    for (const host of numbers.keys()) observer.observe(host);
    return () => {
      live = false;
      observer.disconnect();
    };
  }, [hosts, pageNumbers, ready, renderKey, viewport]);
  const visible = state.key === renderKey ? state.pages : new Set<number>();
  const mounted = useMemo(() => {
    const result = new Set<number>();
    for (let index = 0; index < pageNumbers.length; index++) {
      if (!visible.has(pageNumbers[index]!)) continue;
      for (
        let neighbour = Math.max(0, index - 1);
        neighbour <= Math.min(pageNumbers.length - 1, index + 1);
        neighbour++
      )
        result.add(pageNumbers[neighbour]!);
    }
    if (pinnedPage !== undefined) result.add(pinnedPage);
    return result;
  }, [pageNumbers, pinnedPage, visible]);
  return { visible, mounted };
}
