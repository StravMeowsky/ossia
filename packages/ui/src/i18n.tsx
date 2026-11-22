import { createContext, useContext, useMemo } from 'react';
import type { ReactNode } from 'react';

import type { UiLanguage } from './generated/protocol.ts';
import { messages } from './i18n/messages.ts';
import type { MessageKey } from './i18n/messages.ts';

export type { MessageKey } from './i18n/messages.ts';

type Variables = Record<string, string | number>;

export function matchSystemLanguage(languages: readonly string[]): UiLanguage {
  for (const language of languages) {
    const primary = /^([a-z]{2,8})(?:[-_][a-z0-9]{1,8})*$/i.exec(language)?.[1]?.toLowerCase();
    if (primary === 'zh') return 'zh-CN';
    if (primary === 'en') return 'en-US';
  }
  return 'en-US';
}

export function translate(language: UiLanguage, key: MessageKey, values: Variables = {}) {
  return Object.entries(values).reduce(
    (text, [name, value]) => text.replaceAll(`{${name}}`, String(value)),
    messages[language][key] as string,
  );
}

export function formatDate(language: UiLanguage, value: string) {
  return new Date(value).toLocaleString(language);
}

export function jobKindKey(kind: string): MessageKey {
  if (kind === 'import') return 'jobKindImport';
  if (kind === 'check') return 'jobKindCheck';
  if (kind === 'render') return 'jobKindRender';
  if (kind === 'export') return 'jobKindExport';
  if (kind === 'restore') return 'jobKindRestore';
  return 'jobKindPreview';
}

export function jobStateKey(state: string): MessageKey {
  if (state === 'queued') return 'jobStateQueued';
  if (state === 'running') return 'jobStateRunning';
  if (state === 'done') return 'jobStateDone';
  if (state === 'cancelled') return 'jobStateCancelled';
  return 'jobStateError';
}

export function phaseKey(phase: string): MessageKey {
  if (phase === 'queued') return 'phaseQueued';
  if (phase === 'converting') return 'phaseConverting';
  if (phase === 'finished') return 'phaseFinished';
  if (phase === 'failed') return 'phaseFailed';
  if (phase === 'cancelled') return 'phaseCancelled';
  return 'phaseCompiling';
}

const Context = createContext<{
  language: UiLanguage;
  t: (key: MessageKey, values?: Variables) => string;
}>({
  language: 'en-US',
  t: (key, values) => translate('en-US', key, values),
});

export function I18nProvider({
  language,
  children,
}: {
  language: UiLanguage;
  children: ReactNode;
}) {
  const value = useMemo(
    () => ({
      language,
      t: (key: MessageKey, values?: Variables) => translate(language, key, values),
    }),
    [language],
  );
  return <Context.Provider value={value}>{children}</Context.Provider>;
}

export function useI18n() {
  return useContext(Context);
}
