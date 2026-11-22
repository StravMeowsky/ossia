import type { ComponentProps } from 'react';

import { cn } from '../../lib/utils.ts';

import './ui.css';

export function Input({ className, type, ...props }: ComponentProps<'input'>) {
  return <input type={type} data-slot="input" className={cn('ui-input', className)} {...props} />;
}
