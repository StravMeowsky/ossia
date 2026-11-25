import { spawn } from 'node:child_process';

export const windows = process.platform === 'win32';
export const detachedChild = !windows;

/** Stop a child and its descendants; POSIX uses the process group, Windows uses taskkill. */
export function stopProcessTree(child, signal = 'SIGTERM') {
  if (child === undefined || child.pid === undefined) return;
  if (windows) {
    if (child.exitCode !== null || child.signalCode !== null) return;
    const killer = spawn('taskkill', ['/pid', String(child.pid), '/t', '/f'], {
      stdio: 'ignore',
    });
    killer.on('error', () => child.kill(signal));
  } else {
    try {
      process.kill(-child.pid, signal);
    } catch {}
  }
}
