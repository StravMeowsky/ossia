import { spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { candidateRoots } from '../../../scripts/engine.mjs';
import { detachedChild, stopProcessTree, windows } from '../../../scripts/process-tree.mjs';

const desktop = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const root = path.resolve(desktop, '../..');
const child = spawn(
  path.join(desktop, 'node_modules/.bin', windows ? 'tauri.cmd' : 'tauri'),
  [
    'dev',
    '--config',
    'src-tauri/tauri.dev.conf.json',
    '--',
    '--',
    '--home',
    path.join(root, '.ossia'),
    '--port',
    '3211',
    '--engine-root',
    candidateRoots(root).roots[0].dir,
    '--docs-root',
    path.join(root, 'vendor/docs/lilypond/2.26.0'),
    '--static-dir',
    path.join(root, 'packages/ui/dist'),
    '--dev-origin',
    'http://127.0.0.1:5173',
    ...process.argv.slice(2),
  ],
  { cwd: desktop, stdio: 'inherit', shell: windows, detached: detachedChild },
);
let stopping = false;

function stop(signal = 'SIGTERM') {
  if (stopping) return;
  stopping = true;
  stopProcessTree(child, signal);
}

for (const signal of ['SIGINT', 'SIGTERM']) {
  process.on(signal, () => stop(signal));
}
process.on('exit', () => stop());

child.on('error', (error) => {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
  stop();
});
child.on('exit', (code, signal) => {
  if (stopping) return;
  process.exitCode = code ?? (signal ? 1 : 0);
  stop();
});
