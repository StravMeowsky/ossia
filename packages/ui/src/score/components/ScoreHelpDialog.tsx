import { CircleHelp } from 'lucide-react';

import { useI18n } from '../../i18n.tsx';
import type { MessageKey } from '../../i18n.tsx';
import { useTouchOnlyInput } from '../hooks/useTouchOnlyInput.ts';
import { resolveAnnotationModifier } from '../utils/annotationInput.ts';
import { SharedDialog } from './SharedDialog.tsx';

interface HelpRow {
  trigger: string;
  description: MessageKey;
  touchOnly?: boolean;
  desktopOnly?: boolean;
}

interface HelpGroup {
  heading: MessageKey;
  rows: HelpRow[];
}

/** Opens the score page's read-only help reference. */
export function ScoreHelpDialog() {
  const { t } = useI18n();
  const modifier = resolveAnnotationModifier();
  const touchOnly = useTouchOnlyInput();
  const groups: HelpGroup[] = [
    {
      heading: 'helpPlayback',
      rows: [
        { trigger: t('helpTriggerNoteClick'), description: 'helpPlaybackClick' },
        { trigger: '← / →', description: 'helpPlaybackNavigate', desktopOnly: true },
        {
          trigger: t('helpTriggerChordClick'),
          description: 'helpPlaybackChordClick',
          desktopOnly: true,
        },
        { trigger: 'Space', description: 'helpPlaybackToggle', desktopOnly: true },
      ],
    },
    {
      heading: 'helpAnnotation',
      rows: [
        { trigger: 'E', description: 'helpAnnotationToggle', desktopOnly: true },
        { trigger: t('helpTriggerClickElement'), description: 'helpAnnotationClickElement' },
        { trigger: t('helpTriggerBoxSelect'), description: 'helpAnnotationBoxSelect' },
        {
          trigger: t('helpTriggerModifierGroup', { modifier }),
          description: 'helpAnnotationMulti',
          desktopOnly: true,
        },
        { trigger: 'Esc', description: 'helpAnnotationDismiss', desktopOnly: true },
      ],
    },
    {
      heading: 'helpView',
      rows: [
        {
          trigger: t('helpTriggerModifierDrag', { modifier }),
          description: 'helpViewPan',
          desktopOnly: true,
        },
        {
          trigger: t('helpTriggerModifierScroll', { modifier }),
          description: 'helpViewZoom',
          desktopOnly: true,
        },
        { trigger: t('helpTriggerTouchPan'), description: 'helpViewTouchPan', touchOnly: true },
        { trigger: t('helpTriggerTouchZoom'), description: 'helpViewTouchZoom', touchOnly: true },
      ],
    },
    {
      heading: 'helpDrawing',
      rows: [
        { trigger: 'D', description: 'helpDrawingPen', desktopOnly: true },
        { trigger: 'R', description: 'helpDrawingEraser', desktopOnly: true },
        {
          trigger: t('helpTriggerErase', { modifier }),
          description: 'helpDrawingModifier',
          desktopOnly: true,
        },
        { trigger: 'Esc', description: 'helpDrawingExit', desktopOnly: true },
        { trigger: t('helpTriggerDraw'), description: 'helpDrawingTouch', touchOnly: true },
      ],
    },
  ];
  const visibleGroups = groups
    .map((group) => ({
      ...group,
      rows: group.rows.filter((row) => (touchOnly ? !row.desktopOnly : !row.touchOnly)),
    }))
    .filter((group) => group.rows.length > 0);

  return (
    <SharedDialog
      title={t('helpTitle')}
      triggerLabel={t('helpAction')}
      triggerIcon={<CircleHelp aria-hidden="true" size={17} />}
      triggerClassName="topbar-action topbar-help"
    >
      {visibleGroups.map((group) => (
        <section key={group.heading} className="shared-dialog-group">
          <h3>{t(group.heading)}</h3>
          <dl className="help-list">
            {group.rows.map((row) => (
              <div key={row.description} className="help-row">
                <dt>
                  <span className="help-trigger">{row.trigger}</span>
                </dt>
                <dd>{t(row.description)}</dd>
              </div>
            ))}
          </dl>
        </section>
      ))}
    </SharedDialog>
  );
}
