import { useEffect, useState } from 'react';

import { describe } from '../../errors.ts';
import type { RenderReference } from '../../generated/protocol.ts';
import type { Api } from '../utils/api.ts';
import { loadVersionRender, peekVersionRender } from '../utils/renderCache.ts';

interface VersionRenderState {
  key: string | null;
  render: RenderReference | null;
  error: string;
  loading: boolean;
}

const EMPTY_STATE: VersionRenderState = {
  key: null,
  render: null,
  error: '',
  loading: false,
};

/** Loads the render associated with a committed version. */
export function useVersionRender(api: Api, documentId: string | null, versionId: string | null) {
  const [state, setState] = useState<VersionRenderState>(EMPTY_STATE);
  const key = documentId && versionId ? `${documentId}:${versionId}` : null;
  const cached = peekVersionRender(documentId, versionId);
  const current = cached
    ? { render: cached, error: '', loading: false }
    : state.key === key
      ? state
      : { render: null, error: '', loading: !!key };

  useEffect(() => {
    if (!documentId || !versionId) {
      setState(EMPTY_STATE);
      return;
    }
    const cached = peekVersionRender(documentId, versionId);
    if (cached) {
      setState({ key, render: cached, error: '', loading: false });
      return;
    }
    let live = true;
    setState({ key, render: null, error: '', loading: true });
    loadVersionRender(api, documentId, versionId)
      .then((render) => {
        if (live) setState({ key, render, error: '', loading: false });
      })
      .catch((reason) => {
        if (live) setState({ key, render: null, error: describe(reason), loading: false });
      });
    return () => {
      live = false;
    };
  }, [api, documentId, key, versionId]);

  return { render: current.render, error: current.error, loading: current.loading };
}
