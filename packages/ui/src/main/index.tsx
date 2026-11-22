import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';

import { ToastRegion, useToasts } from '../components/Toast.tsx';
import { describe } from '../errors.ts';
import type { ManagementPreferences, ManagementPreferencesChange } from '../generated/protocol.ts';
import { I18nProvider, matchSystemLanguage } from '../i18n.tsx';
import { MainContent } from './MainContent.tsx';
import { platformName } from './types.ts';

import './main.css';

if ('__TAURI_INTERNALS__' in window) {
  document.addEventListener('contextmenu', (event) => event.preventDefault());
}

function Main() {
  const native = '__TAURI_INTERNALS__' in window;
  const detectedLanguage = matchSystemLanguage(navigator.languages);
  const toast = useToasts();
  const [preferences, setPreferences] = useState<ManagementPreferences | null>(null);

  useEffect(() => {
    document.body.dataset.platform = !native ? 'browser' : platformName();
  }, [native]);

  useEffect(() => {
    if (!native) return;
    let live = true;
    void invoke<ManagementPreferences>('management_preferences')
      .then(async (loaded) => {
        if (!loaded.language) {
          const initialized = await invoke<ManagementPreferences>('management_preferences_change', {
            change: {
              theme: loaded.theme,
              language: detectedLanguage,
              trayClickAction: loaded.trayClickAction,
              storageCacheLimit: loaded.storageCacheLimit,
              storageAutoCleanup: loaded.storageAutoCleanup,
              lanAccess: loaded.lanAccess,
              autoDownload: loaded.autoDownload,
              updateChannel: loaded.updateChannelPreference,
              updateSource: loaded.updateSource,
            } satisfies ManagementPreferencesChange,
          });
          if (live) setPreferences(initialized);
        } else if (live) {
          setPreferences(loaded);
        }
      })
      .catch((error) => {
        if (live) toast.show(describe(error), 'error');
      });
    return () => {
      live = false;
    };
  }, [detectedLanguage, native]);

  const theme = preferences?.theme ?? 'system';
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    document.documentElement.style.colorScheme = theme === 'system' ? 'light dark' : theme;
  }, [theme]);

  const language = preferences?.language ?? detectedLanguage;
  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);

  return (
    <I18nProvider language={language}>
      <MainContent
        native={native}
        preferences={preferences}
        setPreferences={setPreferences}
        notify={toast.show}
      />
      <ToastRegion toasts={toast.toasts} onDismiss={toast.dismiss} />
    </I18nProvider>
  );
}

const root = document.getElementById('root');
if (!root) throw new Error('missing #root');
createRoot(root).render(<Main />);
