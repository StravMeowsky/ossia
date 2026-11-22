import * as PopoverPrimitive from '@radix-ui/react-popover';
import { QrCode } from 'lucide-react';
import { QRCodeSVG } from 'qrcode.react';
import { useEffect, useState } from 'react';

import { Button } from '../../components/ui/button.tsx';
import { useI18n } from '../../i18n.tsx';
import { focusScorePage } from '../utils/scoreFocus.ts';

function isLoopbackHost(hostname: string) {
  const normalized = hostname.toLowerCase().replace(/^\[|\]$/g, '');
  return normalized === 'localhost' || normalized === '::1' || normalized.startsWith('127.');
}

function scoreUrl(webBaseUrl: string | null, documentId: string | null) {
  if (!webBaseUrl || !documentId) return null;
  try {
    const base = new URL(webBaseUrl);
    if (!['http:', 'https:'].includes(base.protocol) || isLoopbackHost(base.hostname)) return null;
    const url = new URL('/scores', base);
    url.searchParams.set('documentId', documentId);
    return url.toString();
  } catch {
    return null;
  }
}

/** Shows a LAN-only QR link for the current score. */
export function ScoreQrCode({
  webBaseUrl,
  documentId,
}: {
  webBaseUrl: string | null;
  documentId: string | null;
}) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const url = scoreUrl(webBaseUrl, documentId);
  const available = url !== null;

  useEffect(() => {
    if (!available) setOpen(false);
  }, [available]);

  return (
    <PopoverPrimitive.Root open={open} onOpenChange={setOpen}>
      <PopoverPrimitive.Trigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="topbar-action topbar-qr"
          aria-label={available ? t('qrCodeAction') : t('qrCodeRequiresLan')}
          data-tooltip={available ? t('qrCodeAction') : t('qrCodeRequiresLan')}
          disabled={!available}
        >
          <QrCode aria-hidden="true" size={17} />
        </Button>
      </PopoverPrimitive.Trigger>
      {available && (
        <PopoverPrimitive.Portal>
          <PopoverPrimitive.Content
            className="score-qr-panel"
            align="end"
            sideOffset={8}
            collisionPadding={12}
            aria-label={t('qrCodeAction')}
            onCloseAutoFocus={(event) => {
              event.preventDefault();
              focusScorePage();
            }}
          >
            <div
              className="score-qr-panel-code"
              role="img"
              aria-label={t('qrCodeLabel', { url: url! })}
              data-qr-url={url!}
            >
              <QRCodeSVG
                value={url!}
                size={192}
                level="M"
                marginSize={2}
                bgColor="#ffffff"
                fgColor="#111111"
              />
            </div>
            <p>{t('qrCodeSameNetwork')}</p>
          </PopoverPrimitive.Content>
        </PopoverPrimitive.Portal>
      )}
    </PopoverPrimitive.Root>
  );
}
