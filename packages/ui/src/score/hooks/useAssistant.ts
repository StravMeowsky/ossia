import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useCallback, useEffect, useRef, useState } from 'react';

import { describe } from '../../errors.ts';

export type AssistantMessage = {
  id: number;
  role: 'user' | 'assistant';
  text: string;
  tools: string[];
  error?: string;
};

type AssistantEvent =
  | { kind: 'session'; session: string }
  | { kind: 'text'; text: string }
  | { kind: 'tool'; name: string }
  | { kind: 'done'; session: string | null; error: string | null };

type Saved = { session: string | null; messages: AssistantMessage[] };

/** The assistant runs through the desktop app, so it exists only inside its score windows. */
export function assistantAvailable() {
  return '__TAURI_INTERNALS__' in window;
}

const storageKey = (documentId: string) => `ossia:assistant:${documentId}`;

function load(documentId: string): Saved {
  try {
    const value = JSON.parse(
      localStorage.getItem(storageKey(documentId)) ?? 'null',
    ) as Saved | null;
    if (value && Array.isArray(value.messages)) return value;
  } catch {
    // A missing or unreadable conversation starts fresh.
  }
  return { session: null, messages: [] };
}

function save(documentId: string, saved: Saved) {
  try {
    localStorage.setItem(storageKey(documentId), JSON.stringify(saved));
  } catch {
    // Keep the conversation for this window when storage is unavailable.
  }
}

/** One Claude Code conversation per score, continued across messages and window reopens. */
export function useAssistant(documentId: string | undefined, title: string) {
  const [messages, setMessages] = useState<AssistantMessage[]>([]);
  const [running, setRunning] = useState(false);
  const session = useRef<string | null>(null);
  const nextId = useRef(1);

  useEffect(() => {
    if (!documentId) return;
    const saved = load(documentId);
    session.current = saved.session;
    nextId.current = saved.messages.reduce((id, message) => Math.max(id, message.id + 1), 1);
    setMessages(saved.messages);
    setRunning(false);
  }, [documentId]);

  useEffect(() => {
    if (documentId) save(documentId, { session: session.current, messages });
  }, [documentId, messages]);

  const updateReply = useCallback(
    (change: (message: AssistantMessage) => AssistantMessage) =>
      setMessages((current) => {
        const last = current.at(-1);
        if (!last || last.role !== 'assistant') return current;
        return [...current.slice(0, -1), change(last)];
      }),
    [],
  );

  useEffect(() => {
    if (!assistantAvailable()) return;
    let live = true;
    let unlisten: (() => void) | undefined;
    void listen<AssistantEvent>('assistant-event', ({ payload }) => {
      if (payload.kind === 'session') {
        session.current = payload.session;
      } else if (payload.kind === 'text') {
        updateReply((message) => ({
          ...message,
          text: message.text ? `${message.text}\n\n${payload.text}` : payload.text,
        }));
      } else if (payload.kind === 'tool') {
        updateReply((message) => ({ ...message, tools: [...message.tools, payload.name] }));
      } else {
        if (payload.session) session.current = payload.session;
        const error = payload.error;
        // Claude Code also sends a failure as reply text; show it once, as the error.
        updateReply((message) =>
          error
            ? { ...message, text: message.text.replace(error, '').trim(), error }
            : { ...message },
        );
        setRunning(false);
      }
    }).then((stop) => {
      if (live) unlisten = stop;
      else stop();
    });
    return () => {
      live = false;
      unlisten?.();
    };
  }, [updateReply]);

  const send = useCallback(
    (text: string) => {
      const message = text.trim();
      if (!documentId || !message || running) return;
      const id = nextId.current;
      nextId.current += 2;
      setMessages((current) => [
        ...current,
        { id, role: 'user', text: message, tools: [] },
        { id: id + 1, role: 'assistant', text: '', tools: [] },
      ]);
      setRunning(true);
      invoke('assistant_send', {
        args: { documentId, title, message, session: session.current },
      }).catch((reason: unknown) => {
        updateReply((reply) => ({ ...reply, error: describe(reason) }));
        setRunning(false);
      });
    },
    [documentId, running, title, updateReply],
  );

  const stop = useCallback(() => {
    void invoke('assistant_stop').catch(() => undefined);
  }, []);

  const clear = useCallback(() => {
    if (running) return;
    session.current = null;
    setMessages([]);
  }, [running]);

  return { messages, running, send, stop, clear };
}

export type AssistantController = ReturnType<typeof useAssistant>;
