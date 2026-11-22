const STORAGE_PREFIX = 'ossia:score-version:v1:';

function storageKey(documentId: string) {
  return `${STORAGE_PREFIX}${documentId}`;
}

/** Returns the last viewed committed version for a score, or the current draft. */
export function readRememberedVersion(documentId: string | null) {
  if (!documentId) return null;
  try {
    return localStorage.getItem(storageKey(documentId));
  } catch {
    return null;
  }
}

/** Persists the viewed version per score; the current draft removes the entry. */
export function rememberVersion(documentId: string | null, versionId: string | null) {
  if (!documentId) return;
  try {
    if (versionId) localStorage.setItem(storageKey(documentId), versionId);
    else localStorage.removeItem(storageKey(documentId));
  } catch {
    // Version memory is optional browser state; storage failures must not block the score page.
  }
}
