import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/** Arguments for a throwaway NSIS install that does not create user shortcuts. */
export function windowsInstallerArgs(root) {
  return ['/S', '/NS', `/D=${root}`];
}

function windowsInstallRegistryKeys(productName, manufacturer) {
  return [
    `HKCU\\Software\\${manufacturer}`,
    `HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${productName}`,
  ];
}

function runRegistry(args) {
  const result = spawnSync('reg', args, {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  });
  if (result.error) throw result.error;
  return result;
}

function registryKeyExists(key) {
  const providerPath = key.replace(/^HKCU\\/, 'HKCU:\\');
  const result = spawnSync(
    'powershell',
    [
      '-NoProfile',
      '-NonInteractive',
      '-Command',
      `if (Test-Path -LiteralPath '${providerPath.replace(/'/g, "''")}') { exit 0 } else { exit 1 }`,
    ],
    {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
    },
  );
  if (result.error) throw result.error;
  if (result.status === 0) return true;
  if (result.status === 1 && !result.stderr.trim()) return false;
  throw new Error(`cannot inspect registry key ${key}: ${result.stderr.trim()}`);
}

function exportRegistryKey(key, backup) {
  const result = runRegistry(['export', key, backup, '/y']);
  if (result.status !== 0) {
    throw new Error(`cannot export registry key ${key}: ${result.stderr.trim()}`);
  }
}

function deleteRegistryKey(key) {
  const result = runRegistry(['delete', key, '/f']);
  if (result.status !== 0 && registryKeyExists(key)) {
    throw new Error(`cannot delete registry key ${key}: ${result.stderr.trim()}`);
  }
}

function importRegistryKey(backup) {
  const result = runRegistry(['import', backup]);
  if (result.status !== 0) {
    throw new Error(`cannot import registry backup ${backup}: ${result.stderr.trim()}`);
  }
}

/**
 * Preserve the user's installer registry state while a smoke install runs.
 * NSIS writes HKCU entries even when the payload lives in a temporary directory.
 */
export async function withWindowsInstallState({ productName, manufacturer }, operation) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'ossia-install-state-'));
  const snapshots = [];
  try {
    for (const key of windowsInstallRegistryKeys(productName, manufacturer)) {
      if (!registryKeyExists(key)) {
        snapshots.push({ key, backup: null });
        continue;
      }
      const backup = path.join(directory, `${snapshots.length}.reg`);
      exportRegistryKey(key, backup);
      snapshots.push({ key, backup });
    }
    return await operation();
  } finally {
    try {
      for (const snapshot of snapshots.reverse()) {
        if (registryKeyExists(snapshot.key)) deleteRegistryKey(snapshot.key);
        if (snapshot.backup) importRegistryKey(snapshot.backup);
      }
    } finally {
      fs.rmSync(directory, { recursive: true, force: true });
    }
  }
}
