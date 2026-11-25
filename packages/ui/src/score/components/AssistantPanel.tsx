import { ArrowUp, SquarePen, Square, X } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';

import { Button } from '../../components/ui/button.tsx';
import type { UiLanguage } from '../../generated/protocol.ts';
import { translate, useI18n } from '../../i18n.tsx';
import type { MessageKey } from '../../i18n.tsx';
import type { AssistantController, AssistantMessage } from '../hooks/useAssistant.ts';

const STEPS: [RegExp, MessageKey][] = [
  [/^ossia_guide$|^ossia_docs$/, 'assistantStepGuide'],
  [/^ossia_(context|status|list)$/, 'assistantStepContext'],
  [/^ossia_read_(source|drawing)$/, 'assistantStepRead'],
  [/^ossia_(edit|write_source|write_drawing|update_metadata|convert)$/, 'assistantStepEdit'],
  [/^ossia_(check|job)$/, 'assistantStepCheck'],
  [/^ossia_commit$/, 'assistantStepCommit'],
  [/^ossia_(evidence|layout)$/, 'assistantStepAnalyze'],
  [/^ossia_output$/, 'assistantStepOutput'],
  [/^ossia_import$/, 'assistantStepImport'],
  [/^ossia_(history|restore)$/, 'assistantStepHistory'],
];

/** What the assistant is doing now, from the last Ossia tool it called. */
function currentStep(tools: string[]): MessageKey {
  const tool = tools.findLast((name) => name.startsWith('ossia_'));
  if (!tool) return 'assistantThinking';
  return STEPS.find(([pattern]) => pattern.test(tool))?.[1] ?? 'assistantStepWork';
}

/**
 * Step lines follow the conversation's language rather than the app's: Claude's reply so far,
 * or else the message it is answering.
 */
function conversationLanguage(messages: AssistantMessage[], index: number): UiLanguage {
  const reply = messages[index]?.text;
  const asked = messages[index - 1]?.text ?? '';
  return /[\u3400-\u9fff]/.test(reply || asked) ? 'zh-CN' : 'en-US';
}

/** Right-hand chat with Claude Code about the open score. */
export function AssistantPanel({
  assistant,
  onClose,
}: {
  assistant: AssistantController;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const { messages, running } = assistant;
  const [draft, setDraft] = useState('');
  const list = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    input.current?.focus();
  }, []);
  useEffect(() => {
    const element = list.current;
    if (element) element.scrollTop = element.scrollHeight;
  }, [messages]);

  const submit = () => {
    if (!draft.trim() || running) return;
    assistant.send(draft);
    setDraft('');
  };

  return (
    <aside className="score-assistant" aria-label={t('assistantTitle')}>
      <header className="score-assistant-header">
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="score-assistant-action"
          aria-label={t('assistantNew')}
          title={t('assistantNew')}
          disabled={running || messages.length === 0}
          onClick={assistant.clear}
        >
          <SquarePen aria-hidden="true" size={16} />
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="score-assistant-action"
          aria-label={t('assistantClose')}
          title={t('assistantClose')}
          onClick={onClose}
        >
          <X aria-hidden="true" size={16} />
        </Button>
      </header>
      <div className="score-assistant-messages" ref={list} aria-live="polite">
        {messages.map((message, index) => {
          const working = running && message.role === 'assistant' && index === messages.length - 1;
          return (
            <div key={message.id} className={`score-assistant-message is-${message.role}`}>
              {message.text && <p className="score-assistant-text">{message.text}</p>}
              {working && !message.error && (
                <p className="score-assistant-pending">
                  {translate(conversationLanguage(messages, index), currentStep(message.tools))}
                </p>
              )}
              {message.error && (
                <p className="score-assistant-error">
                  {message.error === 'stopped' ? t('assistantStopped') : message.error}
                </p>
              )}
            </div>
          );
        })}
      </div>
      <form
        className="score-assistant-composer"
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        <textarea
          ref={input}
          value={draft}
          rows={3}
          aria-label={t('assistantMessage')}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === 'Enter' && !event.shiftKey && !event.nativeEvent.isComposing) {
              event.preventDefault();
              submit();
            }
          }}
        />
        {running ? (
          <Button
            type="button"
            size="icon"
            className="score-assistant-send"
            aria-label={t('assistantStop')}
            title={t('assistantStop')}
            onClick={assistant.stop}
          >
            <Square aria-hidden="true" size={12} fill="currentColor" />
          </Button>
        ) : (
          <Button
            type="submit"
            size="icon"
            className="score-assistant-send"
            aria-label={t('assistantSend')}
            title={t('assistantSend')}
            disabled={!draft.trim()}
          >
            <ArrowUp aria-hidden="true" size={16} />
          </Button>
        )}
      </form>
    </aside>
  );
}
