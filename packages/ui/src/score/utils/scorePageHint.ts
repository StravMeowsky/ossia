import type { DocumentState, EditActivity, Job, UiLanguage } from '../../generated/protocol.ts';
import type { MessageKey } from '../../i18n.tsx';
import { jobKindKey, jobStateKey, phaseKey } from '../../i18n.tsx';

type Translate = (key: MessageKey, values?: Record<string, string | number>) => string;

interface DocumentHintOptions {
  doc: DocumentState;
  editActivity: EditActivity | null | undefined;
  stale: boolean;
  jobs: Job[];
  language: UiLanguage;
  t: Translate;
}

/** Summarizes the document, edit ownership, render freshness and active jobs for the toolbar. */
export function formatDocumentHint({
  doc,
  editActivity,
  stale,
  jobs,
  language,
  t,
}: DocumentHintOptions) {
  if (doc.editState === 'active') {
    return t('editingScoreHint', { author: editActivity?.author ?? 'Ossia' });
  }
  return [
    doc.dirty ? t('dirtyWorkspace') : t('cleanWorkspace'),
    editActivity ? t('editor', { author: editActivity.author }) : null,
    ['interrupted', 'recovery-conflict'].includes(doc.editState) ? t('recoveryNeeded') : null,
    stale ? t('staleRender') : null,
    jobs.length
      ? t('processing', {
          jobs: jobs
            .map(
              (job) =>
                `${t(jobKindKey(job.kind))} · ${t(phaseKey(job.phase))} (${t(
                  jobStateKey(job.state),
                )})`,
            )
            .join(language === 'zh-CN' ? '；' : '; '),
        })
      : null,
  ]
    .filter(Boolean)
    .join(language === 'zh-CN' ? '；' : ' · ');
}
