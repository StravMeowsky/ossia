import { enUS } from './en-US.ts';
import { zhCN } from './zh-CN.ts';

export type { MessageKey } from './en-US.ts';

export const messages = {
  'en-US': enUS,
  'zh-CN': zhCN,
} as const;
