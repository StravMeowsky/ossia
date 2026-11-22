import { X } from 'lucide-react';

import { useI18n } from '../../i18n.tsx';

export function ScoreAnnotationBadge({
  annotationCount,
  onClear,
}: {
  annotationCount: number;
  onClear: () => void;
}) {
  const { t } = useI18n();
  if (annotationCount === 0) {
    return null;
  }
  const summary = t('annotationBadge', { annotations: annotationCount });
  return (
    <div className="score-annotation-badge" role="status" aria-live="polite">
      <span className="score-annotation-badge-text">{summary}</span>
      <button
        type="button"
        className="score-annotation-badge-clear"
        aria-label={t('clearAnnotations')}
        data-tooltip={t('clearAnnotations')}
        onClick={onClear}
      >
        <X aria-hidden="true" size={14} strokeWidth={2.3} />
      </button>
    </div>
  );
}
