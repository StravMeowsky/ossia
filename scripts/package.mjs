import { execFileSync, spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { readLock, verifyTree, refreshGoMtimes, sha256 } from './engine.mjs';
import { resolveBuildStamp, rpmBundleConfig } from './release/version.mjs';
import { windowsInstallerArgs, withWindowsInstallState } from './windows-install-state.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const APP_VERSION = JSON.parse(fs.readFileSync(path.join(ROOT, 'package.json'), 'utf8')).version;
const BUILD_STAMP = resolveBuildStamp(APP_VERSION, process.env.OSSIA_BUILD_STAMP);
const APP_BUNDLE = 'Ossia.app';
const DMG_BACKGROUND = path.join(ROOT, 'packages/desktop/src-tauri/dmg/background.png');
const DMG_VOLUME_ICON = path.join(ROOT, 'packages/desktop/src-tauri/icons/icon.icns');
const STARTER_MANIFEST = path.join(ROOT, 'packages/ossia-backend/resources/starter/manifest.json');
process.env.OSSIA_BUILD_STAMP = BUILD_STAMP;
const TARGETS = {
  'darwin-arm64': {
    triple: 'aarch64-apple-darwin',
    bundles: ['app'],
    platform: 'macos',
    artifact: 'macos-arm64',
  },
  'darwin-x86_64': {
    triple: 'x86_64-apple-darwin',
    bundles: ['app'],
    platform: 'macos',
    artifact: 'macos-x86_64',
  },
  'mingw-x86_64': {
    triple: 'x86_64-pc-windows-msvc',
    bundles: ['nsis'],
    platform: 'windows',
    artifact: 'windows-x86_64',
  },
  'linux-x86_64': {
    triple: 'x86_64-unknown-linux-gnu',
    bundles: ['deb', 'rpm'],
    platform: 'linux',
    artifact: 'linux-x86_64',
  },
  'linux-arm64': {
    triple: 'aarch64-unknown-linux-gnu',
    bundles: ['deb', 'rpm'],
    platform: 'linux',
    artifact: 'linux-arm64',
  },
};

export function specFor(target) {
  const spec = TARGETS[target];
  if (!spec) throw new Error(`unsupported package target: ${target}`);
  return spec;
}

/** Bundle output directories Tauri populates for one package target. */
export function bundleOutputDirectories(root, target) {
  const spec = specFor(target);
  const bundleRoot = path.join(root, 'target', spec.triple, 'release/bundle');
  return spec.bundles.map((bundle) => path.join(bundleRoot, bundle === 'app' ? 'macos' : bundle));
}

/** Remove previous bundles so stale installers cannot be collected. */
export function clearBundleOutputs(root, target) {
  for (const directory of bundleOutputDirectories(root, target)) {
    fs.rmSync(directory, { recursive: true, force: true });
  }
}

export function assertBuildable(target, lock) {
  const entry = lock.targets ? lock.targets[target] : lock;
  if (!entry) throw new Error(`engine target is not locked: ${target}`);
  if (entry.target && entry.target !== target) throw new Error(`engine target mismatch: ${target}`);
  if (!entry.archive) {
    throw new Error(
      `${target} has no built engine archive; build and lock the native engine before packaging`,
    );
  }
  return entry;
}

function targetArg(argv) {
  return argv.find((arg) => arg.startsWith('--target='))?.slice('--target='.length) ?? hostTarget();
}

function hostTarget() {
  const target = `${process.platform === 'win32' ? 'mingw' : process.platform}-${process.arch === 'x64' ? 'x86_64' : process.arch}`;
  if (!TARGETS[target]) throw new Error(`unsupported host target: ${target}`);
  return target;
}

function run(command, args, options = {}) {
  return execFileSync(command, args, {
    cwd: ROOT,
    shell: process.platform === 'win32',
    stdio: ['ignore', 'pipe', 'pipe'],
    encoding: 'utf8',
    maxBuffer: 64 << 20,
    ...options,
  });
}

function clean(root) {
  return run('git', ['status', '--porcelain', '--untracked-files=all'], { cwd: root });
}

/** Require the build to use the same sources as the archive. */
export function assertClean(root = ROOT) {
  const status = clean(root);
  if (status.trim()) {
    throw new Error(
      'packaging requires a clean working tree so the source archive matches the app',
    );
  }
}

function licenseFiles(dir) {
  if (!fs.existsSync(dir)) return [];
  return fs
    .readdirSync(dir)
    .filter((name) => /^(licen[cs]e|copying|notice|copyright)/i.test(name))
    .filter((name) => fs.statSync(path.join(dir, name)).isFile())
    .sort();
}

function copyLicenseFiles(source, destination) {
  const files = licenseFiles(source);
  fs.mkdirSync(destination, { recursive: true });
  for (const name of files) fs.copyFileSync(path.join(source, name), path.join(destination, name));
  return files;
}

/** Resolve the runtime dependencies included in the desktop executable. */
export function rustDependencies(triple) {
  const metadata = JSON.parse(run('cargo', ['metadata', '--locked', '--format-version', '1']));
  const byName = new Map();
  for (const pkg of metadata.packages) byName.set(`${pkg.name}@${pkg.version}`, pkg);
  const tree = run('cargo', [
    'tree',
    '--locked',
    '-p',
    'ossia-desktop',
    '-e',
    'normal',
    '--target',
    triple,
    '--prefix',
    'none',
    '--format',
    '{p}',
  ]);
  const seen = new Set();
  const result = [];
  for (const line of tree.split(/\r?\n/)) {
    const match = line.trim().match(/^(\S+) v(\S+)/);
    if (!match) continue;
    const key = `${match[1]}@${match[2]}`;
    if (seen.has(key)) continue;
    seen.add(key);
    const pkg = byName.get(key);
    if (pkg) result.push(pkg);
  }
  return result;
}

/** Copy each Rust dependency's local license and attribution materials. */
export function collectRustLicenses(packages, destination) {
  const entries = [];
  for (const pkg of packages) {
    const directory = path.join(destination, `${pkg.name}@${pkg.version}`);
    const manifestDir = path.dirname(pkg.manifest_path);
    const files = licenseFiles(manifestDir).map((name) => path.join(manifestDir, name));
    if (pkg.license_file) {
      const declared = path.resolve(manifestDir, pkg.license_file);
      if (fs.existsSync(declared)) files.push(declared);
    }
    const uniqueFiles = [...new Set(files)].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
    if (uniqueFiles.length) {
      fs.mkdirSync(directory, { recursive: true });
      for (const file of uniqueFiles)
        fs.copyFileSync(file, path.join(directory, path.basename(file)));
    }
    entries.push({
      name: pkg.name,
      version: pkg.version,
      license: pkg.license,
      repository: pkg.repository ?? pkg.homepage ?? null,
      source: pkg.source,
      licenseFiles: uniqueFiles.map((file) =>
        path.relative(destination, path.join(directory, path.basename(file))),
      ),
      missingLicenseText: !uniqueFiles.some(
        (file) =>
          /^(licen[cs]e|copying)/i.test(path.basename(file)) ||
          (pkg.license_file && file === path.resolve(manifestDir, pkg.license_file)),
      ),
    });
  }
  return entries;
}

/** Copy npm dependency licenses without recording local installation paths. */
export function collectNpmLicenses(
  destination,
  groups = JSON.parse(run('pnpm', ['licenses', 'list', '--prod', '--json'])),
) {
  const entries = [];
  for (const [license, packages] of Object.entries(groups)) {
    for (const pkg of packages) {
      for (const packagePath of pkg.paths ?? []) {
        const directory = path.join(
          destination,
          `${pkg.name}@${pkg.versions?.join(',') || 'unknown'}`,
        );
        const files = copyLicenseFiles(packagePath, directory);
        entries.push({
          name: pkg.name,
          versions: pkg.versions ?? [],
          license,
          source: pkg.homepage ?? null,
          licenseFiles: files,
          missingLicenseText: files.length === 0,
        });
      }
    }
  }
  const tauriApi = path.join(ROOT, 'packages/ui/node_modules/@tauri-apps/api');
  if (!entries.some((entry) => entry.name === '@tauri-apps/api')) {
    const packageJson = JSON.parse(fs.readFileSync(path.join(tauriApi, 'package.json'), 'utf8'));
    const directory = path.join(destination, `${packageJson.name}@${packageJson.version}`);
    const files = copyLicenseFiles(tauriApi, directory);
    entries.push({
      name: packageJson.name,
      versions: [packageJson.version],
      license: packageJson.license,
      source: packageJson.homepage ?? null,
      licenseFiles: files,
      missingLicenseText: files.length === 0,
    });
  }
  return entries;
}

function notices(manifest) {
  const lines = [
    '# Third-party notices',
    '',
    `Ossia ${manifest.appVersion} (${manifest.target})`,
    '',
    'The LilyPond engine and its complete license tree are included under `licenses/engine/`.',
    'Offline LilyPond manuals and their original notices are included under `docs/` and `licenses/documentation/`.',
    'Ossia itself is GPL-3.0-or-later; the full license text is `licenses/Ossia-GPL-3.0-or-later.txt`.',
    'Audio sample sources, licenses, and processing are recorded in `ui/audio/manifest.json`; Salamander Grand Piano V2 by Alexander Holm is licensed under CC BY 3.0 (https://creativecommons.org/licenses/by/3.0/).',
    '',
  ];
  const groups = new Map();
  for (const dep of manifest.npmDependencies) {
    const key = dep.license || 'unknown';
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(`- ${dep.name} ${dep.versions.join(', ')} (npm)`);
  }
  for (const dep of manifest.rustDependencies) {
    const key = dep.license || 'unknown';
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(`- ${dep.name} ${dep.version} (Rust)`);
  }
  for (const [license, packages] of [...groups.entries()].sort(([a], [b]) => a.localeCompare(b))) {
    lines.push(`## ${license}`, '');
    lines.push(...packages.sort(), '');
  }
  const missing = [
    ...manifest.npmDependencies.filter((dep) => dep.missingLicenseText),
    ...manifest.rustDependencies.filter((dep) => dep.missingLicenseText),
  ];
  if (missing.length) {
    lines.push('## Missing local license text', '');
    lines.push(
      'A local license file was not identified automatically for these packages. Check any included notices and their sources in `SOURCE-MANIFEST.json`.',
      '',
    );
    for (const dep of missing) lines.push(`- ${dep.name} (${dep.license || 'unknown'})`);
    lines.push('');
  }
  return `${lines.join('\n')}\n`;
}

function materials(stage, target, entry, sourceArchive) {
  const destination = path.join(stage, 'materials');
  const licenses = path.join(destination, 'licenses');
  fs.mkdirSync(licenses, { recursive: true });
  fs.copyFileSync(path.join(ROOT, 'LICENSE'), path.join(licenses, 'Ossia-GPL-3.0-or-later.txt'));
  const engineLicenses = path.join(licenses, 'engine');
  fs.mkdirSync(engineLicenses, { recursive: true });
  fs.cpSync(
    path.join(ROOT, 'vendor/lilypond', `${entry.version}-${target}`, 'licenses'),
    engineLicenses,
    {
      recursive: true,
    },
  );
  const rustPackages = rustDependencies(specFor(target).triple);
  const rust = collectRustLicenses(
    rustPackages.filter((pkg) => pkg.source !== null),
    path.join(licenses, 'rust'),
  );
  const workspacePackages = rustPackages
    .filter((pkg) => pkg.source === null)
    .map((pkg) => ({ name: pkg.name, version: pkg.version }));
  const npm = collectNpmLicenses(path.join(licenses, 'npm'));
  const engineLock = JSON.parse(fs.readFileSync(path.join(ROOT, 'vendor/ENGINE.lock'), 'utf8'));
  const engineSource = engineLock.targets[target]?.source ?? null;
  const dependencyLock = JSON.parse(
    fs.readFileSync(path.join(ROOT, 'vendor/ENGINE.dependencies.json'), 'utf8'),
  );
  const documentation = JSON.parse(fs.readFileSync(path.join(ROOT, 'vendor/DOCS.lock'), 'utf8'));
  const docsManifest = JSON.parse(
    fs.readFileSync(path.join(ROOT, 'vendor/docs/lilypond/2.26.0/manifest.json'), 'utf8'),
  );
  if (
    documentation.version !== entry.version ||
    docsManifest.archiveSha256 !== documentation.sha256
  )
    throw new Error('documentation does not match the packaged engine');
  const docsLicenses = path.join(licenses, 'documentation');
  fs.mkdirSync(docsLicenses, { recursive: true });
  for (const file of fs.readdirSync(path.join(ROOT, 'vendor/docs/lilypond/2.26.0/notices')))
    fs.copyFileSync(
      path.join(ROOT, 'vendor/docs/lilypond/2.26.0/notices', file),
      path.join(docsLicenses, file),
    );
  const manifest = {
    documentation: { ...documentation, contentHash: docsManifest.contentHash },
    schemaVersion: 1,
    appVersion: APP_VERSION,
    target,
    commit: run('git', ['rev-parse', 'HEAD']).trim(),
    engine: {
      version: entry.version,
      engineBuildId: entry.engineBuildId,
      archive: entry.archive,
      source: engineSource ?? null,
      dependencyLock: {
        path: 'vendor/ENGINE.dependencies.json',
        sha256: sha256(path.join(ROOT, 'vendor/ENGINE.dependencies.json')),
        packages: dependencyLock.packages.map((item) => ({
          file: item.file,
          url: item.url,
          sha256: item.sha256,
        })),
        buildScripts: dependencyLock.buildScripts.map((item) => item.path),
      },
    },
    sourceArchive: {
      file: path.basename(sourceArchive),
      sha256: sha256(sourceArchive),
    },
    workspacePackages,
    rustDependencies: rust,
    npmDependencies: npm,
    sourceReferences: [
      'LilyPond 2.26.0 and its dependency archives are referenced by checksum in vendor/ENGINE.lock and vendor/ENGINE.dependencies.json.',
      'Rust registry sources are identified by cargo metadata and copied license files where available.',
      'npm production sources are identified by pnpm licenses list.',
    ],
  };
  const manifestPath = path.join(destination, 'SOURCE-MANIFEST.json');
  fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
  fs.writeFileSync(path.join(destination, 'THIRD-PARTY-NOTICES.md'), notices(manifest));
  fs.copyFileSync(manifestPath, path.join(licenses, 'SOURCE-MANIFEST.json'));
  fs.copyFileSync(
    path.join(destination, 'THIRD-PARTY-NOTICES.md'),
    path.join(licenses, 'THIRD-PARTY-NOTICES.md'),
  );
  return { destination, licenses, manifest };
}

function buildUI() {
  run('pnpm', ['--filter', '@ossia/ui', 'build'], { stdio: 'inherit' });
}

function overlay(stage, target, entry) {
  const spec = specFor(target);
  const engine = path.join(ROOT, 'vendor/lilypond', `${entry.version}-${target}`);
  const ui = path.join(ROOT, 'packages/ui/dist');
  const licenses = path.join(stage, 'materials/licenses');
  const config = {
    bundle: {
      resources: {
        [`${engine}${path.sep}`]: 'engine',
        [`${ui}${path.sep}`]: 'ui',
        [`${path.join(ROOT, 'vendor/docs/lilypond/2.26.0')}${path.sep}`]: 'docs/lilypond/2.26.0',
        [`${licenses}${path.sep}`]: 'licenses',
      },
    },
  };
  if (spec.platform === 'windows') {
    config.bundle.windows = { nsis: { compression: 'lzma' } };
  }
  const file = path.join(stage, 'tauri.package.conf.json');
  fs.writeFileSync(file, `${JSON.stringify(config, null, 2)}\n`);
  return { file, spec };
}

function findApp(target) {
  const triple = specFor(target).triple;
  const candidates = [
    path.join(ROOT, 'target', triple, 'release/bundle/macos', APP_BUNDLE),
    path.join(ROOT, 'target/release/bundle/macos', APP_BUNDLE),
  ];
  const app = candidates.find((candidate) => fs.existsSync(candidate));
  if (!app) throw new Error(`built app not found for ${target}`);
  return app;
}

async function normalizeEngine(app, target, entry) {
  const engine = path.join(app, 'Contents/Resources/engine');
  const inventory = JSON.parse(fs.readFileSync(path.join(engine, 'resources.json'), 'utf8'));
  if (inventory.engineBuildId !== entry.engineBuildId || inventory.target !== target) {
    throw new Error('bundled engine inventory does not match the lock');
  }
  for (const file of inventory.files) {
    const absolute = path.join(engine, file.path);
    const stat = fs.statSync(absolute);
    if (stat.size !== file.bytes || sha256(absolute) !== file.sha256) {
      throw new Error(`bundled engine resource is damaged: ${file.path}`);
    }
    if (file.executable && process.platform !== 'win32' && (stat.mode & 0o111) === 0) {
      fs.chmodSync(absolute, stat.mode | 0o755);
    }
  }
  await refreshGoMtimes(engine);
  verifyTree(engine, readLock(ROOT, target), target === hostTarget());
}

async function freePort() {
  const server = net.createServer();
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const port = server.address().port;
  await new Promise((resolve, reject) =>
    server.close((error) => (error ? reject(error) : resolve())),
  );
  return port;
}

function wait(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function processExit(child) {
  return new Promise((resolve) => {
    if (child.exitCode !== null || child.signalCode !== null) resolve();
    else child.once('exit', resolve);
  });
}

async function terminate(child) {
  if (!child || child.exitCode !== null || child.signalCode !== null) return;
  child.kill('SIGTERM');
  await Promise.race([processExit(child), wait(3000)]);
  if (child.exitCode === null && child.signalCode === null) {
    child.kill('SIGKILL');
    await Promise.race([processExit(child), wait(1000)]);
  }
}

async function waitFor(check, timeout = 30000) {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    try {
      if (await check()) return;
    } catch {}
    await wait(200);
  }
  throw new Error('timed out waiting for packaged backend');
}

const EXECUTABLE_NAMES = new Set(['ossia', 'ossia.exe', 'ossia-desktop', 'ossia-desktop.exe']);

export function inspectPayload(root) {
  let executable;
  let resources;
  const pending = [root];
  while (pending.length) {
    const directory = pending.pop();
    const entries = fs.readdirSync(directory, { withFileTypes: true });
    if (
      !resources &&
      fs.existsSync(path.join(directory, 'engine/resources.json')) &&
      fs.existsSync(path.join(directory, 'ui/index.html'))
    ) {
      resources = directory;
    }
    for (const entry of entries) {
      const absolute = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        pending.push(absolute);
      } else if (entry.isFile() && EXECUTABLE_NAMES.has(entry.name.toLowerCase())) {
        if (process.platform !== 'win32' && (fs.statSync(absolute).mode & 0o111) === 0) continue;
        executable = absolute;
      }
    }
  }
  if (!executable) throw new Error(`packaged executable not found under ${root}`);
  if (!resources) throw new Error(`packaged resources not found under ${root}`);
  for (const relative of [
    'engine/resources.json',
    'ui/index.html',
    'ui/audio/manifest.json',
    'docs/lilypond/2.26.0/manifest.json',
  ]) {
    if (!fs.existsSync(path.join(resources, relative))) {
      throw new Error(`packaged resources are missing ${relative} under ${resources}`);
    }
  }
  const audio = path.join(resources, 'ui/audio');
  const catalog = JSON.parse(fs.readFileSync(path.join(audio, 'manifest.json'), 'utf8'));
  for (const [instrument, entry] of Object.entries(catalog.instruments)) {
    if (!entry.samples?.length) throw new Error(`packaged audio has no ${instrument} samples`);
    for (const sample of entry.samples) {
      for (const format of ['ogg', 'mp3']) {
        const relative = sample[format];
        if (
          typeof relative !== 'string' ||
          path.isAbsolute(relative) ||
          relative.split('/').includes('..')
        ) {
          throw new Error(`invalid packaged ${instrument} sample path: ${relative}`);
        }
        if (!fs.existsSync(path.join(audio, relative))) {
          throw new Error(`packaged ${instrument} sample is missing: ${relative}`);
        }
      }
    }
  }
  return { executable, resources };
}

async function verifyPayload(root, label, entry) {
  const { executable, resources } = inspectPayload(root);
  const version = run(executable, ['--version']).trim();
  if (!version.includes(APP_VERSION)) {
    throw new Error(
      `${label} executable reports ${JSON.stringify(version)}, expected ${APP_VERSION}`,
    );
  }
  const inventory = JSON.parse(fs.readFileSync(path.join(resources, 'engine/resources.json')));
  if (inventory.engineBuildId !== entry.engineBuildId) {
    throw new Error(`${label} bundled engine does not match the lock`);
  }
  const home = fs.mkdtempSync(path.join(os.tmpdir(), 'ossia-smoke-home-'));
  try {
    const doctor = JSON.parse(run(executable, ['--home', home, 'doctor']));
    if (!doctor.engine?.ok) {
      throw new Error(`${label} doctor failed: ${JSON.stringify(doctor.engine)}`);
    }
    const port = await freePort();
    const child = spawn(executable, ['--home', home, '--port', String(port), 'serve'], {
      stdio: 'ignore',
      windowsHide: true,
    });
    try {
      try {
        await waitFor(async () => {
          const response = await fetch(`http://127.0.0.1:${port}/`);
          return response.ok && (await response.text()).includes('Ossia');
        });
      } catch (error) {
        if (child.exitCode !== null) {
          throw new Error(`${label} backend exited with code ${child.exitCode}`);
        }
        throw error;
      }
      const status = JSON.parse(run(executable, ['--home', home, 'status']));
      if (!status.runtime?.webBaseUrl) throw new Error(`${label} status has no webBaseUrl`);
      const starters = JSON.parse(fs.readFileSync(STARTER_MANIFEST, 'utf8')).scores;
      await waitFor(
        async () => {
          const response = await fetch(
            `http://127.0.0.1:${port}/api/v1/library/scores?limit=200&offset=0`,
          );
          if (!response.ok) return false;
          const page = await response.json();
          const imported = new Map(page.items.map((score) => [score.title, score.documentId]));
          for (const starter of starters) {
            const documentId = imported.get(starter.title);
            if (!documentId) return false;
            const previewResponse = await fetch(
              `http://127.0.0.1:${port}/api/v1/library/scores/${encodeURIComponent(documentId)}/preview`,
              {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ retry: false }),
              },
            );
            if (!previewResponse.ok) return false;
            const preview = await previewResponse.json();
            if (preview.state !== 'ready') return false;
          }
          return true;
        },
        Math.max(60_000, starters.length * 30_000),
      );
    } finally {
      try {
        run(executable, ['--home', home, 'stop']);
      } catch {}
      await terminate(child);
    }
  } finally {
    fs.rmSync(home, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 });
  }
}

function finderDmgLayoutScript() {
  return `on run argv
  set diskName to item 1 of argv
  set mountDir to item 2 of argv
  tell application "Finder"
    tell disk diskName
      open
      set containerWindow to container window
      tell containerWindow
        set current view to icon view
        set toolbar visible to false
        set statusbar visible to false
        set the bounds to {100, 100, 760, 548}
      end tell
      set options to the icon view options of containerWindow
      tell options
        set arrangement to not arranged
        set icon size to 96
        set text size to 13
        set background picture to POSIX file (mountDir & "/.background/background.tiff")
      end tell
      set position of item "${APP_BUNDLE}" to {150, 150}
      set position of item "Applications" to {465, 150}
      set position of item ".background" to {700, 1000}
      set position of item ".VolumeIcon.icns" to {800, 1000}
      close
      open
      delay 1
    end tell
  end tell
end run
`;
}

export function createDmg(root, app, destination, { volumeName = 'Ossia' } = {}) {
  const staging = path.join(root, 'dmg-root');
  const writable = path.join(root, 'dmg-writable.dmg');
  const script = path.join(root, 'dmg-layout.applescript');
  let mounted;
  try {
    fs.rmSync(staging, { recursive: true, force: true });
    fs.rmSync(writable, { force: true });
    fs.rmSync(destination, { force: true });
    fs.mkdirSync(staging, { recursive: true });
    run('ditto', [app, path.join(staging, APP_BUNDLE)]);
    fs.symlinkSync('/Applications', path.join(staging, 'Applications'));
    const backgroundDirectory = path.join(staging, '.background');
    fs.mkdirSync(backgroundDirectory);
    const background = path.join(backgroundDirectory, 'background.tiff');
    run('sips', [
      '-s',
      'format',
      'tiff',
      '-s',
      'dpiWidth',
      '144',
      '-s',
      'dpiHeight',
      '144',
      DMG_BACKGROUND,
      '--out',
      background,
    ]);
    const volumeIcon = path.join(staging, '.VolumeIcon.icns');
    fs.copyFileSync(DMG_VOLUME_ICON, volumeIcon);
    run('SetFile', ['-c', 'icnC', volumeIcon]);
    run('chflags', ['hidden', volumeIcon]);
    run('hdiutil', [
      'create',
      '-volname',
      volumeName,
      '-srcfolder',
      staging,
      '-ov',
      '-format',
      'UDRW',
      '-fs',
      'HFS+',
      writable,
    ]);
    const attached = run('hdiutil', [
      'attach',
      '-mountrandom',
      '/Volumes',
      '-readwrite',
      '-noverify',
      '-noautoopen',
      '-nobrowse',
      writable,
    ]);
    mounted = attached
      .split(/\r?\n/)
      .flatMap((line) => line.split('\t'))
      .map((part) => part.trim())
      .find((part) => part.startsWith('/Volumes/'));
    if (!mounted) throw new Error('hdiutil did not report a DMG mount point');
    run('SetFile', ['-a', 'C', mounted]);
    fs.writeFileSync(script, finderDmgLayoutScript());
    run('osascript', [script, path.basename(mounted), mounted]);
    for (let attempt = 0; attempt < 50; attempt += 1) {
      if (fs.existsSync(path.join(mounted, '.DS_Store'))) break;
      spawnSync('sleep', ['0.1']);
    }
    if (!fs.existsSync(path.join(mounted, '.DS_Store'))) {
      throw new Error('Finder did not persist the DMG window layout');
    }
    run('hdiutil', ['detach', mounted, '-quiet']);
    mounted = undefined;
    run('hdiutil', [
      'convert',
      writable,
      '-format',
      'UDZO',
      '-imagekey',
      'zlib-level=9',
      '-o',
      destination,
    ]);
  } finally {
    if (mounted) {
      try {
        run('hdiutil', ['detach', mounted, '-quiet', '-force']);
      } catch {}
    }
    fs.rmSync(staging, { recursive: true, force: true });
    fs.rmSync(writable, { force: true });
    fs.rmSync(script, { force: true });
  }
}

async function verifyDmg(dmgPath, entry) {
  const mount = fs.mkdtempSync(path.join(os.tmpdir(), 'ossia-dmg-'));
  try {
    run('hdiutil', ['attach', '-nobrowse', '-readonly', '-mountpoint', mount, dmgPath]);
    await verifyPayload(mount, 'DMG', entry);
  } finally {
    try {
      run('hdiutil', ['detach', mount, '-quiet', '-force']);
    } catch {}
    fs.rmSync(mount, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 });
  }
}

function temporaryDirectory(prefix) {
  const base = process.env.RUNNER_TEMP || os.tmpdir();
  const directory = fs.mkdtempSync(path.join(base, prefix));
  if (process.platform !== 'win32' || !/\s/.test(directory)) return directory;
  return fs.mkdtempSync(path.join(process.env.SystemDrive || 'C:', prefix));
}

function findFile(root, match) {
  const pending = [root];
  while (pending.length) {
    const directory = pending.pop();
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const absolute = path.join(directory, entry.name);
      if (entry.isDirectory()) pending.push(absolute);
      else if (entry.isFile() && match(entry.name)) return absolute;
    }
  }
}

async function installWindows(installer, root) {
  const result = spawnSync(installer, windowsInstallerArgs(root), {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(`Windows installer failed (${result.status}): ${result.stderr.trim()}`);
  }
  await waitFor(() => inspectPayload(root));
}

async function uninstallWindows(root) {
  if (!fs.existsSync(root)) return;
  const uninstaller = findFile(root, (name) => /^uninstall.*\.exe$/i.test(name));
  if (uninstaller) {
    const result = spawnSync(uninstaller, ['/S'], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
    });
    if (result.error) throw result.error;
    if (result.status !== 0) {
      throw new Error(`Windows uninstaller failed (${result.status}): ${result.stderr.trim()}`);
    }
  }
  await waitFor(() => !fs.existsSync(root), 10000).catch(() => {});
  fs.rmSync(root, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 });
}

function extractDeb(deb, root) {
  try {
    run('dpkg-deb', ['-x', deb, root]);
  } catch (error) {
    if (error.code === 'ENOENT') {
      throw new Error('verifying a DEB package requires dpkg-deb');
    }
    throw error;
  }
}

function extractRpm(rpm, root) {
  const result = spawnSync(
    'bash',
    ['-o', 'pipefail', '-c', 'rpm2cpio "$1" | cpio -idm --quiet', 'bash', rpm],
    { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] },
  );
  if (result.error?.code === 'ENOENT') {
    throw new Error('verifying an RPM package requires bash, rpm2cpio and cpio');
  }
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(`cannot extract RPM package: ${result.stderr.trim()}`);
  }
}

async function verifyArtifacts(target, artifactDir, files, entry) {
  const spec = specFor(target);
  if (spec.platform === 'macos') {
    await verifyPayload(path.join(artifactDir, APP_BUNDLE), 'app bundle', entry);
    const disk = files.find((file) => file.endsWith('.dmg'));
    await verifyDmg(disk, entry);
    return [APP_BUNDLE, path.basename(disk)];
  }
  if (spec.platform === 'windows') {
    const installer = files.find((file) => file.endsWith('.exe'));
    const root = temporaryDirectory('ossia-install-');
    const config = JSON.parse(
      fs.readFileSync(path.join(ROOT, 'packages/desktop/src-tauri/tauri.conf.json'), 'utf8'),
    );
    const productName = config.productName;
    const manufacturer = config.bundle?.publisher;
    if (!productName || !manufacturer) {
      throw new Error('Windows packaging requires productName and bundle.publisher');
    }
    await withWindowsInstallState({ productName, manufacturer }, async () => {
      try {
        await installWindows(installer, root);
        await verifyPayload(root, 'Windows installer', entry);
      } finally {
        await uninstallWindows(root);
      }
    });
    return [path.basename(installer)];
  }
  const deb = files.find((file) => file.endsWith('.deb'));
  const rpm = files.find((file) => file.endsWith('.rpm'));
  for (const [label, archive, extract] of [
    ['DEB package', deb, extractDeb],
    ['RPM package', rpm, extractRpm],
  ]) {
    const root = temporaryDirectory(`ossia-${label.slice(0, 3).toLowerCase()}-`);
    try {
      extract(archive, root);
      await verifyPayload(root, label, entry);
    } finally {
      fs.rmSync(root, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 });
    }
  }
  return [path.basename(deb), path.basename(rpm)];
}

function checksums(files, destination) {
  const root = path.dirname(destination);
  const lines = files
    .filter((file) => fs.statSync(file).isFile())
    .sort()
    .map((file) => `${sha256(file)}  ${path.relative(root, file)}`);
  fs.writeFileSync(destination, `${lines.join('\n')}\n`);
}

function builtBinary(target) {
  const spec = specFor(target);
  const candidates = [
    path.join(ROOT, 'target', spec.triple, 'release', 'ossia'),
    path.join(ROOT, 'target', spec.triple, 'release', 'ossia.exe'),
    path.join(ROOT, 'target', spec.triple, 'release', 'ossia-desktop'),
    path.join(ROOT, 'target', spec.triple, 'release', 'ossia-desktop.exe'),
    path.join(ROOT, 'target/release/ossia'),
    path.join(ROOT, 'target/release/ossia.exe'),
    path.join(ROOT, 'target/release/ossia-desktop'),
    path.join(ROOT, 'target/release/ossia-desktop.exe'),
  ];
  const binary = candidates.find((candidate) => fs.existsSync(candidate));
  if (!binary) throw new Error(`built executable not found for ${target}`);
  return binary;
}

function assertBinaryVersion(target) {
  const output = run(builtBinary(target), ['--version']).trim();
  if (!output.includes(APP_VERSION)) {
    throw new Error(`built executable reports ${JSON.stringify(output)}, expected ${APP_VERSION}`);
  }
}

function peMachine(binary) {
  const image = fs.readFileSync(binary);
  if (image.length < 0x40 || image.toString('ascii', 0, 2) !== 'MZ') {
    throw new Error(`not a PE image: ${binary}`);
  }
  const offset = image.readUInt32LE(0x3c);
  if (offset + 6 > image.length || image.toString('ascii', offset, offset + 4) !== 'PE\0\0') {
    throw new Error(`invalid PE header: ${binary}`);
  }
  return image
    .readUInt16LE(offset + 4)
    .toString(16)
    .padStart(4, '0');
}

function verifyReleaseArtifacts(target, artifactDir, files) {
  const spec = specFor(target);
  const arm64 = spec.triple.startsWith('aarch64');
  if (spec.platform === 'windows') {
    const installers = files.filter((file) => file.endsWith('.exe'));
    if (installers.length !== 1)
      throw new Error(`expected one Windows installer, got ${installers.length}`);
    if (peMachine(builtBinary(target)) !== (arm64 ? 'aa64' : '8664')) {
      throw new Error(`unexpected Windows executable architecture for ${target}`);
    }
    assertBinaryVersion(target);
    return;
  }
  if (spec.platform === 'linux') {
    const deb = files.find((file) => file.endsWith('.deb'));
    const rpm = files.find((file) => file.endsWith('.rpm'));
    if (!deb || !rpm) throw new Error(`expected DEB and RPM bundles for ${target}`);
    if (run('dpkg-deb', ['--field', deb, 'Architecture']).trim() !== (arm64 ? 'arm64' : 'amd64')) {
      throw new Error(`unexpected DEB architecture for ${target}`);
    }
    if (
      run('rpm', ['-qp', '--queryformat', '%{ARCH}', rpm]).trim() !== (arm64 ? 'aarch64' : 'x86_64')
    ) {
      throw new Error(`unexpected RPM architecture for ${target}`);
    }
    assertBinaryVersion(target);
    return;
  }
  if (!fs.readdirSync(artifactDir).some((name) => name.endsWith('.dmg'))) {
    throw new Error(`expected a DMG for ${target}`);
  }
}

export async function packageTarget(target, options = {}) {
  if (options.allowDirty) throw new Error('packaging cannot use a dirty source tree');
  assertClean();
  const lock = readLock(ROOT, target);
  const entry = assertBuildable(target, lock);
  const spec = specFor(target);
  const engine = path.join(ROOT, 'vendor/lilypond', `${entry.version}-${target}`);
  verifyTree(engine, lock, target === hostTarget() && process.platform !== 'win32');
  if (
    !fs.existsSync(path.join(ROOT, 'packages/ui/dist/main.html')) ||
    !fs.existsSync(path.join(ROOT, 'packages/ui/dist/scores.html'))
  )
    buildUI();
  const stage = path.join(ROOT, 'target/package', target);
  fs.rmSync(stage, { recursive: true, force: true });
  fs.mkdirSync(stage, { recursive: true });
  const sourceArchive = path.join(stage, `ossia-${APP_VERSION}-source.tar.gz`);
  run('git', [
    'archive',
    '--format=tar.gz',
    `--prefix=ossia-${APP_VERSION}/`,
    '-o',
    sourceArchive,
    'HEAD',
  ]);
  const material = materials(stage, target, entry, sourceArchive);
  const config = overlay(stage, target, entry);
  if (spec.platform === 'linux') {
    const packaged = JSON.parse(fs.readFileSync(config.file, 'utf8'));
    packaged.bundle.linux = {
      ...packaged.bundle.linux,
      ...rpmBundleConfig(`v${APP_VERSION}`).bundle.linux,
    };
    fs.writeFileSync(config.file, `${JSON.stringify(packaged, null, 2)}\n`);
  }
  const tauri = path.join(
    ROOT,
    'packages/desktop/node_modules/.bin',
    process.platform === 'win32' ? 'tauri.cmd' : 'tauri',
  );
  clearBundleOutputs(ROOT, target);
  run(
    tauri,
    [
      'build',
      '--target',
      spec.triple,
      '--bundles',
      spec.bundles.join(','),
      '--no-sign',
      '--config',
      config.file,
    ],
    { cwd: path.join(ROOT, 'packages/desktop'), stdio: 'inherit' },
  );

  const artifactDir = path.join(ROOT, 'artifacts', spec.artifact);
  fs.rmSync(artifactDir, { recursive: true, force: true });
  fs.mkdirSync(artifactDir, { recursive: true });
  const files = [];
  if (spec.platform === 'macos') {
    const app = findApp(target);
    await normalizeEngine(app, target, entry);
    const installed = path.join(artifactDir, APP_BUNDLE);
    run('ditto', [app, installed]);
    const zip = path.join(artifactDir, `ossia_${APP_VERSION}_${spec.triple}.app.zip`);
    run('ditto', ['-c', '-k', '--sequesterRsrc', '--keepParent', installed, zip]);
    files.push(zip);
    const disk = path.join(artifactDir, `ossia_${APP_VERSION}_${spec.triple}.dmg`);
    createDmg(artifactDir, installed, disk);
    files.push(disk);
  } else {
    const bundleRoot = path.join(ROOT, 'target', spec.triple, 'release/bundle');
    for (const bundle of fs.existsSync(bundleRoot) ? fs.readdirSync(bundleRoot) : []) {
      const directory = path.join(bundleRoot, bundle);
      if (!fs.statSync(directory).isDirectory()) continue;
      for (const name of fs.readdirSync(directory)) {
        const source = path.join(directory, name);
        if (fs.statSync(source).isFile()) {
          const destination = path.join(artifactDir, name);
          fs.copyFileSync(source, destination);
          files.push(destination);
        }
      }
    }
  }
  fs.cpSync(material.destination, path.join(artifactDir, 'materials'), { recursive: true });
  fs.copyFileSync(sourceArchive, path.join(artifactDir, 'materials', path.basename(sourceArchive)));
  files.push(path.join(artifactDir, 'materials', path.basename(sourceArchive)));
  verifyReleaseArtifacts(target, artifactDir, files);
  const verifiedArtifacts = await verifyArtifacts(target, artifactDir, files, entry);
  const release = {
    schemaVersion: 1,
    appVersion: APP_VERSION,
    buildStamp: BUILD_STAMP,
    target,
    triple: spec.triple,
    commit: run('git', ['rev-parse', 'HEAD']).trim(),
    engineBuildId: entry.engineBuildId,
    engineArchive: entry.archive,
    signing: spec.platform === 'macos' ? 'ad-hoc-linker-signed' : 'unsigned',
    notarized: false,
    verification: 'runtime-smoke',
    verifiedArtifacts,
    artifacts: files.map((file) => path.relative(artifactDir, file)),
  };
  fs.writeFileSync(path.join(artifactDir, 'release.json'), `${JSON.stringify(release, null, 2)}\n`);
  checksums(
    [...files, path.join(artifactDir, 'release.json')],
    path.join(artifactDir, 'SHA256SUMS'),
  );
  return { artifactDir, release, materials: material.manifest };
}

async function main(argv) {
  const target = targetArg(argv);
  const result = await packageTarget(target, {
    allowDirty: argv.includes('--allow-dirty'),
  });
  console.log(JSON.stringify(result, null, 2));
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  await main(process.argv.slice(2)).catch((error) => {
    console.error(error.stack || error.message);
    process.exitCode = 1;
  });
}
