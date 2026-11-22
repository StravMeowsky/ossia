export function documentIdFromLocation(): string | null {
  const documentId = new URLSearchParams(location.search).get('documentId');
  return documentId?.trim() || null;
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
