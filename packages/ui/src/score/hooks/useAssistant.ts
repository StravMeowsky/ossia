import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useRef, useState } from 'react';

import { describe } from '../../errors.ts';
import type {
  AssistantProvider,
  AssistantSendArgs,
  AssistantUpdate,
} from '../../generated/protocol.ts';
import { createUuid } from '../utils/uuid.ts';

export type AssistantMessage = {
  id: number;
  role: 'user' | 'assistant';
  text: string;
  tools: string[];
  error?: string;
};

type Saved = { session: string | null; messages: AssistantMessage[] };
type Conversation = Saved & { key: string };

const providerKey = 'ossia:assistant:provider';
const emptyConversation: Saved = { session: null, messages: [] };

/** The assistant runs through the desktop app, so it exists only inside its score windows. */
export function assistantAvailable() {
  return '__TAURI_INTERNALS__' in window;
}

function initialProvider(): AssistantProvider {
  try {
    if (localStorage.getItem(providerKey) === 'codex') return 'codex';
  } catch {
    // Storage may be unavailable in this window.
  }
  return 'claude-code';
}

function load(key: string): Conversation {
  try {
    const saved = JSON.parse(localStorage.getItem(key) ?? 'null') as Saved | null;
    if (saved && Array.isArray(saved.messages)) return { ...saved, key };
  } catch {
    // A missing or unreadable conversation starts fresh.
  }
  return { ...emptyConversation, key };
}

/** Each score has an independent resumable conversation with each provider. */
export function useAssistant(documentId: string | undefined, title: string) {
  const [provider, setProvider] = useState(initialProvider);
  // Use the original key for Claude Code so existing conversations continue on upgrade.
  const key = documentId
    ? `ossia:assistant:${documentId}${provider === 'codex' ? ':codex' : ''}`
    : null;
  const [conversation, setConversation] = useState<Conversation | null>(null);
  const [running, setRunning] = useState(false);
  const active = useRef<{ id: string; key: string } | null>(null);
  const saved = conversation?.key === key ? conversation : null;

  useEffect(() => {
    if (active.current && active.current.key !== key) {
      active.current = null;
      setRunning(false);
      void invoke('assistant_stop').catch(() => undefined);
    }
    setConversation(key ? load(key) : null);
  }, [key]);

  useEffect(() => {
    if (!conversation) return;
    const { key, session, messages } = conversation;
    try {
      localStorage.setItem(key, JSON.stringify({ session, messages }));
    } catch {
      // Keep the conversation for this window when storage is unavailable.
    }
  }, [conversation]);

  useEffect(() => {
    if (!assistantAvailable()) return;
    let live = true;
    let unlisten: (() => void) | undefined;
    void listen<AssistantUpdate>('assistant-event', ({ payload: { runId, event } }) => {
      const run = active.current;
      if (!run || run.id !== runId) return;
      setConversation((current) => {
        if (current?.key !== run.key) return current;
        if (event.kind === 'session') return { ...current, session: event.session };

        const reply = current.messages.at(-1);
        if (reply?.role !== 'assistant') return current;
        let updated = reply;
        if (event.kind === 'text') {
          updated = {
            ...reply,
            text: reply.text ? `${reply.text}\n\n${event.text}` : event.text,
          };
        } else if (event.kind === 'tool') {
          updated = { ...reply, tools: [...reply.tools, event.name] };
        } else if (event.error) {
          // Some CLI failures appear as reply text first. Show the error once.
          updated = {
            ...reply,
            text: reply.text.replace(event.error, '').trim(),
            error: event.error,
          };
        }
        return {
          ...current,
          session: event.kind === 'done' ? (event.session ?? current.session) : current.session,
          messages: [...current.messages.slice(0, -1), updated],
        };
      });
      if (event.kind === 'done') {
        active.current = null;
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
  }, []);

  function selectProvider(next: AssistantProvider) {
    if (active.current || next === provider) return;
    setProvider(next);
    try {
      localStorage.setItem(providerKey, next);
    } catch {
      // Keep the selection for this window when storage is unavailable.
    }
  }

  function send(text: string) {
    const message = text.trim();
    if (!documentId || !saved || !message || active.current) return;
    const run = { id: createUuid(), key: saved.key };
    const id = (saved.messages.at(-1)?.id ?? 0) + 1;
    const reply: AssistantMessage = { id: id + 1, role: 'assistant', text: '', tools: [] };
    const args: AssistantSendArgs = {
      provider,
      runId: run.id,
      documentId,
      title,
      message,
      session: saved.session,
    };
    active.current = run;
    setRunning(true);
    setConversation({
      ...saved,
      messages: [...saved.messages, { id, role: 'user', text: message, tools: [] }, reply],
    });
    void invoke('assistant_send', { args }).catch((reason: unknown) => {
      if (active.current?.id !== run.id) return;
      active.current = null;
      setConversation((current) =>
        current?.key === run.key
          ? {
              ...current,
              messages: [...current.messages.slice(0, -1), { ...reply, error: describe(reason) }],
            }
          : current,
      );
      setRunning(false);
    });
  }

  function stop() {
    void invoke('assistant_stop').catch(() => undefined);
  }

  function clear() {
    if (!active.current && key) setConversation({ key, ...emptyConversation });
  }

  return {
    provider,
    selectProvider,
    /** Whether this score's conversation is loaded, so `send` will deliver a message. */
    ready: !!saved,
    messages: saved?.messages ?? emptyConversation.messages,
    running,
    send,
    stop,
    clear,
  };
}

export type AssistantController = ReturnType<typeof useAssistant>;
