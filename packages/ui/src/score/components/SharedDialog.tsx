import { X } from 'lucide-react';
import type { ReactNode } from 'react';
import { useId, useRef } from 'react';

import { Button } from '../../components/ui/button.tsx';
import { useI18n } from '../../i18n.tsx';

/** Native modal shell shared by the score page's settings and help dialogs. */
export function SharedDialog({
  title,
  triggerLabel,
  triggerIcon,
  triggerClassName,
  className,
  children,
}: {
  title: string;
  triggerLabel: string;
  triggerIcon: ReactNode;
  triggerClassName?: string;
  className?: string;
  children: ReactNode;
}) {
  const { t } = useI18n();
  const dialog = useRef<HTMLDialogElement>(null);
  const titleId = useId();

  return (
    <>
      <Button
        type="button"
        variant="ghost"
        size="icon"
        className={triggerClassName}
        aria-label={triggerLabel}
        data-tooltip={triggerLabel}
        onClick={() => {
          if (!dialog.current?.open) dialog.current?.showModal();
        }}
      >
        {triggerIcon}
      </Button>
      <dialog
        ref={dialog}
        className={['shared-dialog', className].filter(Boolean).join(' ')}
        aria-labelledby={titleId}
      >
        <div className="shared-dialog-header">
          <h2 id={titleId} tabIndex={-1} autoFocus>
            {title}
          </h2>
          <button
            type="button"
            className="shared-dialog-close"
            aria-label={t('close')}
            onClick={() => dialog.current?.close()}
          >
            <X aria-hidden="true" size={18} />
          </button>
        </div>
        <div className="shared-dialog-body ui-scrollbar">{children}</div>
      </dialog>
    </>
  );
}
