interface SpeechResultEvent {
  results: {
    [index: number]: { [index: number]: { transcript: string } | undefined } | undefined;
  };
}

/** Minimal Web Speech recognition surface used by annotation input. */
export interface SpeechRecognitionSession {
  abort(): void;
  lang: string;
  onend: (() => void) | null;
  onerror: (() => void) | null;
  onresult: ((event: SpeechResultEvent) => void) | null;
  start(): void;
}

type SpeechWindow = typeof window & {
  SpeechRecognition?: new () => SpeechRecognitionSession;
  webkitSpeechRecognition?: new () => SpeechRecognitionSession;
};

function speechWindow() {
  return globalThis.window as SpeechWindow | undefined;
}

/** Reports whether the browser exposes speech recognition. */
export function isSpeechRecognitionSupported() {
  return Boolean(speechWindow()?.SpeechRecognition ?? speechWindow()?.webkitSpeechRecognition);
}

/** Starts one speech recognition session, or returns undefined when unsupported. */
export function createSpeechRecognition() {
  const Constructor = speechWindow()?.SpeechRecognition ?? speechWindow()?.webkitSpeechRecognition;
  return Constructor ? new Constructor() : undefined;
}
