import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ui = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

export function prepareTauriAssets() {
  const source = path.join(ui, 'dist');
  const destination = path.join(ui, 'dist-tauri');
  fs.rmSync(destination, { recursive: true, force: true });
  fs.cpSync(source, destination, {
    recursive: true,
    filter: (file) => file !== path.join(source, 'audio'),
  });
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  prepareTauriAssets();
}
