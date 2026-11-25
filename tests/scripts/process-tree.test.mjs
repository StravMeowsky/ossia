import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { test } from 'vite-plus/test';

import { detachedChild, stopProcessTree, windows } from '../../scripts/shared/process-tree.mjs';

function isRunning(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

async function waitForExit(pid, timeout = 3000) {
  const deadline = Date.now() + timeout;
  while (isRunning(pid)) {
    if (Date.now() >= deadline) assert.fail(`process ${pid} is still running`);
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
}

test.skipIf(windows)('stops a process group after its leader has exited', async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ossia-process-tree-'));
  const pidFile = path.join(root, 'child.pid');
  let descendant;
  let leader;
  try {
    leader = spawn(
      process.execPath,
      [
        '-e',
        `
          const { spawn } = require('node:child_process');
          const fs = require('node:fs');
          const child = spawn(process.execPath, ['-e', 'setInterval(() => {}, 1000)'], {
            stdio: 'ignore',
          });
          fs.writeFileSync(${JSON.stringify(pidFile)}, String(child.pid));
          child.unref();
        `,
      ],
      { detached: detachedChild, stdio: 'ignore' },
    );
    await new Promise((resolve, reject) => {
      leader.once('error', reject);
      leader.once('exit', resolve);
    });

    descendant = Number(fs.readFileSync(pidFile, 'utf8'));
    assert.equal(isRunning(descendant), true);

    stopProcessTree(leader, 'SIGKILL');
    await waitForExit(descendant);
  } finally {
    if (descendant && isRunning(descendant)) process.kill(descendant, 'SIGKILL');
    fs.rmSync(root, { recursive: true, force: true });
  }
});
