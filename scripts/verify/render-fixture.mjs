import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { candidateRoots } from '../engine/engine.mjs';

export const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
export const fixtures = path.join(root, 'tests/fixtures/playback');

/** Build the fixture executable once, then render each requested score in isolation. */
export function fixtureRenderer({ selection = false } = {}) {
  execFileSync('cargo', ['build', '--quiet', '-p', 'ossia-engine', '--bin', 'playback-fixture'], {
    cwd: root,
    stdio: 'inherit',
  });
  const exe = path.join(
    path.resolve(root, process.env.CARGO_TARGET_DIR ?? 'target'),
    'debug',
    process.platform === 'win32' ? 'playback-fixture.exe' : 'playback-fixture',
  );
  const engine = process.env.OSSIA_TEST_LILYPOND_ROOT ?? candidateRoots(root).roots[0].dir;
  return (entry) => {
    const filename = path.resolve(fixtures, entry.source);
    const source = readFileSync(filename, 'utf8');
    const raw = execFileSync(exe, [engine, filename, ...(selection ? ['--selection'] : [])], {
      cwd: root,
      encoding: 'utf8',
      maxBuffer: 64 * 1024 * 1024,
    });
    return { ...JSON.parse(raw), source };
  };
}
