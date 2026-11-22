import { Check, Mic, Trash2, X } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';

import { useI18n } from '../../i18n.tsx';
import {
  placeAnnotationEditor,
  type AnnotationEditorPlacement,
} from '../utils/annotationEditorLayout.ts';
import { createSpeechRecognition, isSpeechRecognitionSupported } from '../utils/speech.ts';

const EDITOR_WIDTH = 360;
const EDITOR_HEIGHT = 50;

export function AnnotationEditor({
  placement,
  existing,
  initialText,
  onConfirm,
  onDelete,
  onDismiss,
}: {
  placement: AnnotationEditorPlacement;
  existing: boolean;
  initialText: string;
  onConfirm: (text: string) => void;
  onDelete: () => void;
  onDismiss: () => void;
}) {
  const { t, language } = useI18n();
  const [text, setText] = useState(initialText);
  const [listening, setListening] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const editorRef = useRef<HTMLDivElement>(null);
  const recognitionRef = useRef<ReturnType<typeof createSpeechRecognition>>(undefined);
  const dismissRef = useRef(onDismiss);
  dismissRef.current = onDismiss;
  const { left, top, width } = placeAnnotationEditor({
    placement,
    preferredWidth: EDITOR_WIDTH,
    height: EDITOR_HEIGHT,
  });

  useEffect(() => {
    const dismissOutside = (event: PointerEvent) => {
      if (event.target instanceof Node && editorRef.current?.contains(event.target)) {
        return;
      }
      if (
        event.target instanceof Element &&
        event.target.closest('.score-view .overlay, .score-annotation-pin')
      ) {
        return;
      }
      dismissRef.current();
    };
    document.addEventListener('pointerdown', dismissOutside);
    return () => {
      document.removeEventListener('pointerdown', dismissOutside);
      recognitionRef.current?.abort();
    };
  }, []);

  function confirm() {
    const value = text.trim();
    if (value) {
      onConfirm(value);
    }
  }

  function toggleVoice() {
    if (listening) {
      recognitionRef.current?.abort();
      setListening(false);
      return;
    }
    const recognition = createSpeechRecognition();
    if (!recognition) {
      return;
    }
    recognitionRef.current = recognition;
    recognition.lang = language;
    recognition.onend = () => setListening(false);
    recognition.onerror = () => setListening(false);
    recognition.onresult = (event) => {
      const transcript = event.results[0]?.[0]?.transcript ?? '';
      if (transcript) {
        setText((current) => `${current}${current ? ' ' : ''}${transcript}`);
      }
      inputRef.current?.focus();
    };
    setListening(true);
    recognition.start();
    inputRef.current?.focus();
  }

  return (
    <div
      ref={editorRef}
      className="score-annotation-editor"
      role="dialog"
      aria-label={t('annotationInput')}
      style={{ left, top, width }}
    >
      <div className="score-annotation-input-shell">
        <input
          ref={inputRef}
          autoFocus
          type="text"
          value={text}
          aria-label={t('annotationInput')}
          placeholder={t('annotationPlaceholder')}
          onChange={(event) => setText(event.target.value)}
          onKeyDown={(event) => {
            if (event.nativeEvent.isComposing) {
              return;
            }
            if (event.key === 'Escape') {
              event.preventDefault();
              event.stopPropagation();
              onDismiss();
            } else if (event.key === 'Enter') {
              event.preventDefault();
              confirm();
            }
          }}
        />
        {isSpeechRecognitionSupported() && (
          <button
            type="button"
            className={`score-annotation-voice ${listening ? 'is-listening' : ''}`}
            aria-label={t('annotationVoiceInput')}
            aria-pressed={listening}
            onClick={toggleVoice}
          >
            <Mic aria-hidden="true" size={15} strokeWidth={2.1} />
          </button>
        )}
      </div>
      <button
        type="button"
        className="score-annotation-editor-action is-confirm"
        aria-label={t('annotationConfirm')}
        disabled={!text.trim()}
        onClick={confirm}
      >
        <Check aria-hidden="true" size={17} strokeWidth={2.5} />
      </button>
      <button
        type="button"
        className="score-annotation-editor-action is-dismiss"
        aria-label={existing ? t('annotationDeleteGroup') : t('cancel')}
        onClick={existing ? onDelete : onDismiss}
      >
        {existing ? (
          <Trash2 aria-hidden="true" size={16} strokeWidth={2.2} />
        ) : (
          <X aria-hidden="true" size={17} strokeWidth={2.4} />
        )}
      </button>
    </div>
  );
}
