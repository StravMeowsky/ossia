import { useCallback, useEffect, useRef, useState } from 'react';

import type { SampleInstrument, ScoreSettings } from '../../generated/protocol.ts';
import type { Api } from '../utils/api.ts';

/** Loads and saves the settings shared by a score's versions. */
export function useScoreSettings(
  api: Api,
  documentId: string | undefined,
  onError: (error: unknown) => void,
) {
  const [loaded, setLoaded] = useState<{ documentId: string; settings: ScoreSettings } | null>(
    null,
  );
  const settings = loaded && loaded.documentId === documentId ? loaded.settings : null;
  const current = useRef<ScoreSettings | null>(null);
  const generation = useRef(0);
  const refreshing = useRef(0);
  useEffect(() => {
    const run = ++generation.current;
    current.current = null;
    setLoaded(null);
    if (!documentId) return;
    const refresh = () => {
      const request = ++refreshing.current;
      void api
        .scoreSettings(documentId)
        .then((value) => {
          if (run !== generation.current || request !== refreshing.current) return;
          if (current.current && value.revision < current.current.revision) return;
          current.current = value;
          setLoaded({ documentId, settings: value });
        })
        .catch(onError);
    };
    const changed = (event: Event) => {
      if ((event as CustomEvent<{ documentId: string }>).detail.documentId === documentId)
        refresh();
    };
    window.addEventListener('ossia-score-settings-changed', changed);
    refresh();
    return () => {
      generation.current++;
      window.removeEventListener('ossia-score-settings-changed', changed);
    };
  }, [api, documentId, onError]);

  const save = useCallback(
    async (partId: string, value: SampleInstrument | null) => {
      if (!documentId || !current.current) return;
      const run = generation.current;
      const base = current.current;
      const instruments = { ...base.instruments };
      if (value === null) delete instruments[partId];
      else instruments[partId] = value;
      try {
        const saved = await api.saveScoreSettings(documentId, { ...base, instruments });
        if (run !== generation.current) return;
        if (current.current && saved.revision < current.current.revision) return;
        current.current = saved;
        setLoaded({ documentId, settings: saved });
      } catch (error) {
        if (run === generation.current) {
          const latest = await api.scoreSettings(documentId);
          if (
            run === generation.current &&
            (!current.current || latest.revision >= current.current.revision)
          ) {
            current.current = latest;
            setLoaded({ documentId, settings: latest });
          }
        }
        throw error;
      }
    },
    [api, documentId],
  );
  return { settings, save };
}
