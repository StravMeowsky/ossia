export function documentIdFromLocation(): string | null {
  const documentId = new URLSearchParams(location.search).get('documentId');
  return documentId?.trim() || null;
}

/**
 * The desktop app opens each score in its own window, titled with the score, so the page shows
 * neither the title nor the score switcher.
 */
export function scoreWindowFromLocation(): boolean {
  return new URLSearchParams(location.search).get('window') === '1';
}

/**
 * Take the composition brief a new score window was opened with. It is removed from the address
 * so reloading the window does not send it again.
 */
export function takeComposeBriefFromLocation(): string | null {
  const url = new URL(location.href);
  const brief = url.searchParams.get('compose')?.trim() || null;
  if (url.searchParams.has('compose')) {
    url.searchParams.delete('compose');
    history.replaceState(history.state, '', url.href);
  }
  return brief;
}

export function scoreDebugFlagsFromLocation(): Set<string> {
  const value = new URLSearchParams(location.search).get('debug') ?? '';
  return new Set(
    value
      .split(',')
      .map((flag) => flag.trim())
      .filter(Boolean),
  );
}

export function pushDocumentIdToLocation(documentId: string) {
  const url = new URL(location.href);
  url.searchParams.set('documentId', documentId);
  history.pushState(history.state, '', url.href);
}
