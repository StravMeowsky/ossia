import type { AgentIntegration, AgentNotice, AgentState } from '../generated/protocol.ts';
import type { MessageKey } from '../i18n.tsx';
import antigravityIcon from './assets/agents/antigravity.svg';
import claudeIcon from './assets/agents/claude.svg';
import codexIcon from './assets/agents/codex.svg';
import dshIcon from './assets/agents/dsh.svg';
import piIcon from './assets/agents/pi.svg?no-inline';
import workbuddyIcon from './assets/agents/workbuddy.svg';

export type Page = 'agents' | 'scores' | 'general' | 'about';

export const stateKeys: Record<AgentState, MessageKey> = {
  available: 'stateAvailable',
  installed: 'stateInstalled',
  disabled: 'stateDisabled',
  broken: 'stateBroken',
  conflict: 'stateConflict',
  unavailable: 'stateUnavailable',
  'update-required': 'stateUpdateRequired',
};

/** Translation keys for the integration notices defined by the backend. */
export const noticeKeys: Record<AgentNotice['kind'], MessageKey> = {
  'restart-session': 'agentRestartSession',
  'pi-reload': 'agentPiReload',
  'pi-mcp-unsupported': 'agentPiMcpUnsupported',
  'claude-desktop-restart': 'agentClaudeDesktopRestart',
  'codex-plugin-disabled': 'agentCodexDisabled',
  'codex-plugin-missing': 'agentCodexMissing',
  'codex-unmanaged-config': 'agentCodexConflict',
  'dsh-bundle-missing': 'agentDshMissing',
  'dsh-unmanaged-bundle': 'agentDshConflict',
  'workbuddy-plugin-disabled': 'agentWorkbuddyDisabled',
  'workbuddy-plugin-missing': 'agentWorkbuddyMissing',
  'workbuddy-unmanaged-config': 'agentWorkbuddyConflict',
  'workbuddy-reload-and-trust': 'agentWorkbuddyReload',
  'legacy-reinstall': 'agentLegacyReinstall',
  'host-unavailable': 'agentHostUnavailable',
  'legacy-host-unavailable': 'agentLegacyHostUnavailable',
};

const agentIcons: Record<string, { src: string; monochrome?: boolean }> = {
  antigravity: { src: antigravityIcon },
  claude: { src: claudeIcon },
  codex: { src: codexIcon },
  dsh: { src: dshIcon },
  pi: { src: piIcon, monochrome: true },
  workbuddy: { src: workbuddyIcon },
};

export function agentIcon(agent: AgentIntegration) {
  return agentIcons[agent.icon] ?? null;
}

/** Hide the Windows verbatim prefix used internally by canonical paths. */
export function displayPath(path: string) {
  return path.replace(/^\\\\\?\\UNC\\/i, '\\\\').replace(/^\\\\\?\\/i, '');
}

export function platformName() {
  if (/Macintosh|Mac OS X/.test(navigator.userAgent)) return 'macos';
  if (/Windows/.test(navigator.userAgent)) return 'windows';
  return 'linux';
}
