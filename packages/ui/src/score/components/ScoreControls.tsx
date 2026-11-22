import { Minus, Plus } from 'lucide-react';
import { useState } from 'react';

import { Segmented } from '../../components/segmented.tsx';
import { Button } from '../../components/ui/button.tsx';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from '../../components/ui/dropdown-menu.tsx';
import { useI18n } from '../../i18n.tsx';
import { MAX_ZOOM, MIN_ZOOM } from '../utils/scoreLayout.ts';
import type { ScoreLayout } from '../utils/scoreLayout.ts';

const ZOOM_LEVELS = [25, 50, 75, 100, 125, 150, 200, 300, 400];

function LayoutIcon({ axis }: { axis: 'vertical' | 'horizontal' }) {
  return (
    <svg
      aria-hidden="true"
      width={15}
      height={15}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      {axis === 'vertical' ? (
        <>
          <rect x={3} y={3} width={18} height={7} rx={1} />
          <rect x={3} y={14} width={18} height={7} rx={1} />
        </>
      ) : (
        <>
          <rect x={3} y={3} width={7} height={18} rx={1} />
          <rect x={14} y={3} width={7} height={18} rx={1} />
        </>
      )}
    </svg>
  );
}

export function ScoreControls({
  layout,
  onLayout,
  zoom,
  onZoomChange,
}: {
  layout: ScoreLayout;
  onLayout: (layout: ScoreLayout) => void;
  zoom: number;
  onZoomChange: (zoom: number) => void;
}) {
  const { t } = useI18n();
  const [zoomMenuOpen, setZoomMenuOpen] = useState(false);
  const displayedZoom = Math.round(zoom);
  const clampZoom = (value: number) => Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, Math.round(value)));
  return (
    <div
      className={`score-controls ${zoomMenuOpen ? 'is-menu-open' : ''}`}
      aria-label={t('viewControls')}
    >
      <Segmented
        compact
        ariaLabel={t('layoutLabel')}
        value={layout}
        options={[
          {
            value: 'vertical',
            label: <LayoutIcon axis="vertical" />,
            ariaLabel: t('verticalLayout'),
            title: t('verticalLayout'),
          },
          {
            value: 'horizontal',
            label: <LayoutIcon axis="horizontal" />,
            ariaLabel: t('horizontalLayout'),
            title: t('horizontalLayout'),
          },
        ]}
        onChange={(value) => onLayout(value as ScoreLayout)}
      />
      <span className="score-controls-divider" aria-hidden="true" />
      <div className="score-controls-zoom">
        <Button
          type="button"
          variant="ghost"
          size="icon"
          aria-label={t('zoomOut')}
          disabled={zoom <= 25}
          onClick={() => onZoomChange(clampZoom(zoom - 10))}
        >
          <Minus aria-hidden="true" size={15} />
        </Button>
        <DropdownMenu onOpenChange={setZoomMenuOpen}>
          <DropdownMenuTrigger asChild>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="score-zoom-trigger"
              aria-label={t('zoomLabel')}
            >
              {displayedZoom}%
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" sideOffset={8}>
            <DropdownMenuRadioGroup
              value={String(displayedZoom)}
              onValueChange={(value) => onZoomChange(clampZoom(Number(value)))}
            >
              {ZOOM_LEVELS.map((level) => (
                <DropdownMenuRadioItem key={level} value={String(level)}>
                  <span className="ui-menu-item-title">{level}%</span>
                </DropdownMenuRadioItem>
              ))}
            </DropdownMenuRadioGroup>
          </DropdownMenuContent>
        </DropdownMenu>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          aria-label={t('zoomIn')}
          disabled={zoom >= 400}
          onClick={() => onZoomChange(clampZoom(zoom + 10))}
        >
          <Plus aria-hidden="true" size={15} />
        </Button>
      </div>
    </div>
  );
}
