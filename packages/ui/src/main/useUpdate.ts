import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useCallback, useEffect, useRef, useState } from 'react';

import type { UpdateSnapshot } from '../generated/protocol.ts';

const UPDATE_EVENT = 'ossia://update';
const POLL_INTERVAL = 400;

export function useUpdate(native: boolean, onError: (message: string) => void) {
  const [snapshot, setSnapshot] = useState<UpdateSnapshot | null>(null);
  const poll = useRef<number | undefined>(undefined);
  const onErrorRef = useRef(onError);
  onErrorRef.current = onError;

  const stopPolling = useCallback(() => {
    window.clearTimeout(poll.current);
    poll.current = undefined;
  }, []);

  const watch = useCallback(
    (current: UpdateSnapshot | null) => {
      stopPolling();
      if (
        current?.state !== 'checking' &&
        current?.state !== 'downloading' &&
        current?.state !== 'installing'
      ) {
        return;
      }
      poll.current = window.setTimeout(() => {
        void invoke<UpdateSnapshot>('update_state')
          .then((next) => {
            setSnapshot(next);
            watch(next);
          })
          .catch((error) => onErrorRef.current(String(error)));
      }, POLL_INTERVAL);
    },
    [stopPolling],
  );

  const refresh = useCallback(async () => {
    if (!native) return;
    try {
      const next = await invoke<UpdateSnapshot>('update_state');
      setSnapshot(next);
      watch(next);
    } catch (error) {
      onErrorRef.current(String(error));
    }
  }, [native, watch]);

  useEffect(() => {
    if (!native) return;
    let live = true;
    let dispose: (() => void) | undefined;
    void listen<UpdateSnapshot>(UPDATE_EVENT, (event) => {
      if (!live) return;
      setSnapshot(event.payload);
      watch(event.payload);
    }).then((unlisten) => {
      if (live) dispose = unlisten;
      else unlisten();
    });
    void refresh();
    return () => {
      live = false;
      dispose?.();
      stopPolling();
    };
  }, [native, refresh, stopPolling, watch]);

  const run = useCallback(
    async (command: 'update_check' | 'update_apply' | 'release_notes') => {
      try {
        const next = await invoke<UpdateSnapshot>(command);
        setSnapshot(next);
        watch(next);
        return next;
      } catch (error) {
        onErrorRef.current(String(error));
        return null;
      }
    },
    [watch],
  );

  return {
    snapshot,
    refresh,
    check: () => run('update_check'),
    apply: () => run('update_apply'),
    notes: () => run('release_notes'),
  };
}
