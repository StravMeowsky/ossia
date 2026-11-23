import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { candidateRoots } from './engine.mjs';
import { detachedChild, stopProcessTree, windows } from './process-tree.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const executable = path.join(root, 'target/debug', windows ? 'ossia-desktop.exe' : 'ossia-desktop');
const tempRoot = windows ? fs.mkdtempSync(path.join(os.tmpdir(), 'ossia-backend-dev-')) : null;
const backendArgs = [
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
  'serve',
];
const watchRoots = [
  'packages/ossia-types',
  'packages/ossia-score',
  'packages/ossia-engine',
  'packages/ossia-library',
  'packages/ossia-backend',
  'packages/ossia-integrations',
  'packages/ossia-omr',
  'packages/desktop/src-tauri',
];
const watchedPatterns = [
  /^Cargo\.(?:toml|lock)$/,
  /^packages\/ossia-(?:types|score|engine|library|backend|omr)\/(?:Cargo\.toml|build\.rs|src(?:\/|$))/,
  /^packages\/ossia-(?:backend|integrations)\/resources(?:\/|$)/,
  /^packages\/ossia-integrations\/(?:Cargo\.toml|build\.rs|src(?:\/|$))/,
  /^packages\/desktop\/src-tauri\/(?:Cargo\.toml|build\.rs|src(?:\/|$)|capabilities(?:\/|$)|tauri[^/]*\.json$)/,
];
const skippedDirectories = new Set([
  '.git',
  '.ossia',
  'artifacts',
  'dist',
  'node_modules',
  'target',
  'tests',
]);

let backend = null;
let ui = null;
let build = null;
let building = false;
let pending = false;
let stopping = false;
let debounce = null;
let watcherTimer = null;
let lastSnapshot = null;

function spawnProcess(command, args, options = {}) {
  return spawn(command, args, {
    cwd: root,
    stdio: 'inherit',
    detached: detachedChild,
    ...options,
  });
}

async function stopChild(child, signal = 'SIGTERM') {
  if (!child || child.exitCode !== null || child.signalCode !== null) return;
  await new Promise((resolve) => {
    let killTimer;
    let finishTimer;
    const finish = () => {
      clearTimeout(killTimer);
      clearTimeout(finishTimer);
      resolve();
    };
    child.once('exit', finish);
    stopProcessTree(child, signal);
    killTimer = setTimeout(() => stopProcessTree(child, 'SIGKILL'), 5000);
    finishTimer = setTimeout(finish, 6500);
  });
}

function cleanupCopy(copy) {
  if (copy) fs.rmSync(copy, { force: true });
}

function startBackend() {
  let copy = null;
  let command = executable;
  if (windows) {
    copy = path.join(tempRoot, `ossia-desktop-${Date.now()}.exe`);
    fs.copyFileSync(executable, copy);
    command = copy;
  }
  const child = spawnProcess(command, backendArgs);
  const entry = { child, copy };
  backend = entry;
  child.on('error', (error) => {
    process.stderr.write(`backend failed: ${error.message}\n`);
    if (backend === entry) backend = null;
    cleanupCopy(copy);
  });
  child.on('exit', (code, signal) => {
    if (backend === entry) backend = null;
    if (!entry.expected && !stopping) {
      process.stderr.write(
        `backend exited (${code ?? signal ?? 'unknown'}); waiting for source changes\n`,
      );
    }
    cleanupCopy(copy);
  });
}

async function stopBackend() {
  const entry = backend;
  if (!entry) return;
  backend = null;
  entry.expected = true;
  await stopChild(entry.child);
  cleanupCopy(entry.copy);
}

function startUi() {
  const child = spawnProcess('pnpm', ['dev:ui'], { shell: windows });
  ui = child;
  child.on('error', (error) => {
    process.stderr.write(`ui dev server failed: ${error.message}\n`);
    process.exitCode = 1;
    void shutdown();
  });
  child.on('exit', (code, signal) => {
    if (stopping) return;
    process.stderr.write(`ui dev server exited (${code ?? signal ?? 'unknown'})\n`);
    process.exitCode = code ?? (signal ? 1 : 0);
    void shutdown();
  });
}

function buildBackend() {
  return new Promise((resolve) => {
    process.stdout.write('\n[backend] building...\n');
    const child = spawnProcess('cargo', ['build', '-p', 'ossia-desktop', '--no-default-features']);
    build = child;
    let settled = false;
    const finish = (code) => {
      if (settled) return;
      settled = true;
      if (build === child) build = null;
      resolve(code);
    };
    child.on('error', (error) => {
      process.stderr.write(`backend build failed: ${error.message}\n`);
      finish(1);
    });
    child.on('exit', (code, signal) => finish(code ?? (signal ? 1 : 0)));
  });
}

async function rebuild() {
  if (stopping) return;
  if (building) {
    pending = true;
    return;
  }
  building = true;
  let built = false;
  while (!stopping) {
    pending = false;
    const code = await buildBackend();
    if (stopping) break;
    if (code !== 0) {
      pending = false;
      process.stderr.write('[backend] build failed; keeping the current backend\n');
      break;
    }
    if (pending) continue;
    built = true;
    break;
  }
  building = false;
  if (built && !stopping) {
    await stopBackend();
    if (!stopping) {
      try {
        startBackend();
      } catch (error) {
        process.stderr.write(`backend failed to start: ${error.message}\n`);
      }
    }
  }
  if (pending && !stopping) scheduleBuild(0);
}

function scheduleBuild(wait = 300) {
  if (stopping) return;
  pending = true;
  if (building) return;
  clearTimeout(debounce);
  debounce = setTimeout(() => {
    debounce = null;
    void rebuild();
  }, wait);
}

function isWatched(relative) {
  return watchedPatterns.some((pattern) => pattern.test(relative));
}

function collectFiles(dir, files) {
  let entries;
  try {
    entries = fs.readdirSync(dir, { withFileTypes: true });
  } catch {
    return;
  }
  for (const entry of entries) {
    const absolute = path.join(dir, entry.name);
    const relative = path.relative(root, absolute).split(path.sep).join('/');
    if (entry.isDirectory()) {
      if (!skippedDirectories.has(entry.name)) collectFiles(absolute, files);
    } else if (entry.isFile() && isWatched(relative)) {
      const stat = fs.statSync(absolute, { throwIfNoEntry: false });
      if (stat) files.push(`${relative}\0${stat.size}\0${stat.mtimeMs}`);
    }
  }
}

function sourceSnapshot() {
  const files = [];
  for (const relative of ['Cargo.toml', 'Cargo.lock']) {
    const stat = fs.statSync(path.join(root, relative), { throwIfNoEntry: false });
    if (stat) files.push(`${relative}\0${stat.size}\0${stat.mtimeMs}`);
  }
  for (const relative of watchRoots) collectFiles(path.join(root, relative), files);
  files.sort();
  return files.join('\n');
}

function startWatcher() {
  lastSnapshot = sourceSnapshot();
  watcherTimer = setInterval(() => {
    if (stopping) return;
    const next = sourceSnapshot();
    if (next === lastSnapshot) return;
    lastSnapshot = next;
    scheduleBuild();
  }, 750);
}

async function shutdown() {
  if (stopping) return;
  stopping = true;
  clearTimeout(debounce);
  clearInterval(watcherTimer);
  const buildChild = build;
  build = null;
  const uiChild = ui;
  ui = null;
  await Promise.all([
    buildChild ? stopChild(buildChild, 'SIGKILL') : Promise.resolve(),
    stopBackend(),
    uiChild ? stopChild(uiChild) : Promise.resolve(),
  ]);
  if (tempRoot) fs.rmSync(tempRoot, { recursive: true, force: true });
}

for (const signal of ['SIGINT', 'SIGTERM']) {
  process.on(signal, () => {
    void shutdown().then(() => {
      process.exitCode = 0;
    });
  });
}
process.on('exit', () => {
  for (const entry of [backend, ui, build]) {
    const child = entry?.child ?? entry;
    if (child) stopProcessTree(child, 'SIGKILL');
  }
});

startUi();
process.stdout.write('[backend] UI: http://127.0.0.1:5173/scores\n');
startWatcher();
void rebuild();
