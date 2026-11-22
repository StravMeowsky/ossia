import { ChevronDown, History } from 'lucide-react';
import { useRef } from 'react';

import { Button } from '../../components/ui/button.tsx';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from '../../components/ui/dropdown-menu.tsx';
import type { Version } from '../../generated/protocol.ts';
import { useI18n } from '../../i18n.tsx';

export function ScoreVersionMenu({
  versions,
  draftAvailable,
  editing,
  viewingVersionId,
  onViewVersion,
  documentHint,
}: {
  versions: Version[];
  draftAvailable: boolean;
  editing: boolean;
  viewingVersionId: string | null;
  onViewVersion: (versionId: string | null) => void;
  documentHint: string;
}) {
  const { t } = useI18n();
  const pointerSelection = useRef(false);
  const activeVersion = versions.find((version) => version.versionId === viewingVersionId);
  const latestVersion = versions.reduce<Version | null>(
    (latest, version) => (!latest || version.seq > latest.seq ? version : latest),
    null,
  );
  const draftLabel = editing ? t('editingDraft') : t('currentDraft');
  const versionLabel = activeVersion
    ? `${t('version')} ${activeVersion.seq}`
    : draftAvailable
      ? draftLabel
      : latestVersion
        ? t('currentVersion', { seq: latestVersion.seq })
        : t('version');
  const selectedVersion =
    viewingVersionId ?? (draftAvailable ? 'draft' : (latestVersion?.versionId ?? ''));

  return (
    <div
      className="score-version-control"
      data-viewing-version={activeVersion ? 'true' : undefined}
    >
      <DropdownMenu
        onOpenChange={(open) => {
          if (open) pointerSelection.current = false;
        }}
      >
        <DropdownMenuTrigger asChild>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="score-version-trigger score-selector-trigger"
            aria-label={versionLabel}
            title={versionLabel}
          >
            <History
              className="score-version-menu-icon score-selector-menu-icon"
              aria-hidden="true"
              size={17}
            />
            <span className="score-selector-label">{versionLabel}</span>
            <ChevronDown className="score-selector-chevron" aria-hidden="true" size={15} />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent
          align="start"
          onPointerDown={() => {
            pointerSelection.current = true;
          }}
          onCloseAutoFocus={(event) => {
            if (!pointerSelection.current) return;
            event.preventDefault();
            pointerSelection.current = false;
          }}
        >
          <DropdownMenuRadioGroup
            value={selectedVersion}
            onValueChange={(value) => {
              if (value === 'draft') {
                onViewVersion(null);
              } else if (value === latestVersion?.versionId && !draftAvailable) {
                onViewVersion(null);
              } else {
                onViewVersion(value);
              }
            }}
          >
            {draftAvailable && (
              <DropdownMenuRadioItem value="draft">
                <span className="ui-menu-item-title">{draftLabel}</span>
                <span className="ui-menu-item-description">{documentHint}</span>
              </DropdownMenuRadioItem>
            )}
            {versions.map((version) => (
              <DropdownMenuRadioItem key={version.versionId} value={version.versionId}>
                <span className="ui-menu-item-title">
                  {t('version')} {version.seq}
                </span>
                <span className="ui-menu-item-description">
                  {version.summary || version.origin}
                </span>
              </DropdownMenuRadioItem>
            ))}
          </DropdownMenuRadioGroup>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
