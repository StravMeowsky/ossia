import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { test } from 'vite-plus/test';

import {
  assertBuildable,
  assertClean,
  bundleOutputDirectories,
  clearBundleOutputs,
  collectNpmLicenses,
  collectRustLicenses,
  inspectPayload,
  packageTarget,
  rustDependencies,
  specFor,
  verifyMaterials,
} from '../../scripts/package.mjs';
import {
  windowsInstallerArgs,
  withWindowsInstallState,
} from '../../scripts/windows-install-state.mjs';

function temporaryRoot() {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'ossia-package-test-'));
}

test('private resources stay untracked and out of the source archive', () => {
  const root = temporaryRoot();
  try {
    const privateFiles = [
      '.docs/private.md',
      'docs/install-guide/index.html',
      'packages/ossia-omr/src/main.rs',
    ];
    const publicFiles = [
      'packages/ossia-backend/src/lib.rs',
      'scripts/release/cos.mjs',
      'scripts/release/update-feed.mjs',
      'scripts/release/upload-update.mjs',
      'scripts/release/publish-update-feed.mjs',
    ];
    for (const file of [...privateFiles, ...publicFiles]) {
      fs.mkdirSync(path.dirname(path.join(root, file)), { recursive: true });
      fs.writeFileSync(path.join(root, file), file);
    }
    fs.copyFileSync('.gitattributes', path.join(root, '.gitattributes'));
    fs.copyFileSync('.gitignore', path.join(root, '.gitignore'));
    execFileSync('git', ['init', '-q', root]);
    execFileSync('git', ['-C', root, 'add', '.']);
    const tracked = execFileSync('git', ['-C', root, 'ls-files'], { encoding: 'utf8' });
    for (const file of privateFiles) assert.ok(!tracked.split('\n').includes(file), file);
    execFileSync('git', [
      '-C',
      root,
      '-c',
      'user.name=Test',
      '-c',
      'user.email=test@example.invalid',
      'commit',
      '-qm',
      'test',
    ]);
    const archive = execFileSync('git', ['-C', root, 'archive', '--format=tar', 'HEAD']);
    const entries = execFileSync('tar', ['-tf', '-'], { input: archive, encoding: 'utf8' });
    for (const file of publicFiles) assert.ok(entries.includes(file), file);
    for (const file of privateFiles) assert.ok(!entries.includes(file), file);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('source packaging rejects modified tracked files and untracked files', () => {
  const root = temporaryRoot();
  try {
    execFileSync('git', ['init', '-q', root]);
    fs.writeFileSync(path.join(root, 'tracked'), 'committed');
    execFileSync('git', ['-C', root, 'add', 'tracked']);
    execFileSync('git', [
      '-C',
      root,
      '-c',
      'user.name=Test',
      '-c',
      'user.email=test@example.invalid',
      'commit',
      '-qm',
      'test',
    ]);
    assert.doesNotThrow(() => assertClean(root));
    fs.writeFileSync(path.join(root, 'tracked'), 'changed');
    assert.throws(() => assertClean(root), /clean/);
    execFileSync('git', ['-C', root, 'restore', 'tracked']);
    fs.writeFileSync(path.join(root, 'untracked'), 'new');
    assert.throws(() => assertClean(root), /clean/);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('packaging cannot bypass source consistency checks', async () => {
  await assert.rejects(packageTarget('darwin-arm64', { allowDirty: true }), /dirty/i);
});

test(
  'Rust notice inventory follows the desktop executable dependency tree',
  { timeout: 60_000 },
  () => {
    const host = execFileSync('rustc', ['-vV'], { encoding: 'utf8' })
      .split('\n')
      .find((line) => line.startsWith('host: '))
      .slice('host: '.length);
    const names = new Set(rustDependencies(host).map((pkg) => pkg.name));
    assert.ok(names.has('ossia-desktop'));
    assert.ok(names.has('tauri'));
    assert.ok(names.has('imagequant'));
    assert.ok(!names.has('ossia-omr'));
  },
);

test('Rust materials include copyright notices alongside an explicit license file', () => {
  const root = temporaryRoot();
  try {
    const source = path.join(root, 'crate');
    const destination = path.join(root, 'licenses');
    fs.mkdirSync(source);
    fs.writeFileSync(path.join(source, 'LICENSE'), 'license');
    fs.writeFileSync(path.join(source, 'COPYRIGHT'), 'copyright');
    const [entry] = collectRustLicenses(
      [
        {
          name: 'example',
          version: '1.0.0',
          manifest_path: path.join(source, 'Cargo.toml'),
          license_file: 'LICENSE',
          license: 'MIT',
          source: 'registry',
        },
      ],
      destination,
    );
    assert.deepEqual(entry.licenseFiles, ['example@1.0.0/COPYRIGHT', 'example@1.0.0/LICENSE']);
    assert.equal(
      fs.readFileSync(path.join(destination, 'example@1.0.0/COPYRIGHT'), 'utf8'),
      'copyright',
    );
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('Rust materials include a copyright file when it is the only local notice', () => {
  const root = temporaryRoot();
  try {
    const source = path.join(root, 'crate');
    const destination = path.join(root, 'licenses');
    fs.mkdirSync(source);
    fs.writeFileSync(path.join(source, 'COPYRIGHT'), 'license and attribution');
    const [entry] = collectRustLicenses(
      [
        {
          name: 'example',
          version: '1.0.0',
          manifest_path: path.join(source, 'Cargo.toml'),
          license_file: null,
          license: 'GPL-3.0-or-later',
          source: 'registry',
        },
      ],
      destination,
    );
    assert.deepEqual(entry.licenseFiles, ['example@1.0.0/COPYRIGHT']);
    assert.equal(entry.missingLicenseText, true);
    assert.equal(
      fs.readFileSync(path.join(destination, 'example@1.0.0/COPYRIGHT'), 'utf8'),
      'license and attribution',
    );
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('npm license inventory does not expose local dependency paths', () => {
  const root = temporaryRoot();
  try {
    const source = path.join(root, 'dependency');
    const destination = path.join(root, 'licenses');
    fs.mkdirSync(source);
    fs.writeFileSync(path.join(source, 'LICENSE'), 'license');
    const groups = {
      MIT: [{ name: '@tauri-apps/api', versions: ['1.0.0'], paths: [source] }],
    };
    const entries = collectNpmLicenses(destination, groups);
    assert.equal(entries.length, 1);
    assert.deepEqual(entries[0].licenseFiles, ['LICENSE']);
    assert.ok(!JSON.stringify(entries).includes(root));
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

function writePayload(root, executable) {
  fs.mkdirSync(path.join(root, 'engine'), { recursive: true });
  fs.writeFileSync(path.join(root, 'engine/resources.json'), '{}');
  fs.mkdirSync(path.join(root, 'ui'), { recursive: true });
  fs.writeFileSync(path.join(root, 'ui/index.html'), '<html></html>');
  fs.mkdirSync(path.join(root, 'ui/audio'), { recursive: true });
  fs.writeFileSync(
    path.join(root, 'ui/audio/manifest.json'),
    JSON.stringify({
      instruments: { piano: { samples: [{ ogg: 'test.ogg', mp3: 'test.mp3' }] } },
    }),
  );
  fs.writeFileSync(path.join(root, 'ui/audio/test.ogg'), 'sample');
  fs.writeFileSync(path.join(root, 'ui/audio/test.mp3'), 'sample');
  fs.mkdirSync(path.join(root, 'docs/lilypond/2.26.0'), { recursive: true });
  fs.writeFileSync(path.join(root, 'docs/lilypond/2.26.0/manifest.json'), '{}');
  fs.mkdirSync(path.dirname(executable), { recursive: true });
  fs.writeFileSync(executable, '');
  fs.chmodSync(executable, 0o755);
}

test('installed materials identify the matching source and build', () => {
  const root = temporaryRoot();
  try {
    const licenses = path.join(root, 'licenses');
    for (const directory of ['engine', 'documentation', 'rust', 'npm']) {
      fs.mkdirSync(path.join(licenses, directory), { recursive: true });
    }
    for (const file of ['Ossia-GPL-3.0-or-later.txt', 'THIRD-PARTY-NOTICES.md']) {
      fs.writeFileSync(path.join(licenses, file), 'notice');
    }
    const version = JSON.parse(fs.readFileSync('package.json', 'utf8')).version;
    const commit = 'a'.repeat(40);
    const build = {
      appVersion: version,
      buildStamp: `v${version}`,
      target: 'test',
      commit,
      engineBuildId: 'engine',
    };
    const manifest = {
      appVersion: version,
      target: 'test',
      commit,
      source: {
        tag: build.buildStamp,
        commit,
        url: `https://github.com/micooz/ossia/archive/${commit}.tar.gz`,
      },
    };
    fs.writeFileSync(path.join(licenses, 'BUILD-INFO.json'), JSON.stringify(build));
    fs.writeFileSync(path.join(licenses, 'SOURCE-MANIFEST.json'), JSON.stringify(manifest));
    assert.doesNotThrow(() => verifyMaterials(root, { engineBuildId: 'engine' }, commit));
    fs.rmSync(path.join(licenses, 'THIRD-PARTY-NOTICES.md'));
    assert.throws(
      () => verifyMaterials(root, { engineBuildId: 'engine' }, commit),
      /THIRD-PARTY-NOTICES/,
    );
    fs.writeFileSync(path.join(licenses, 'THIRD-PARTY-NOTICES.md'), 'notice');
    manifest.source.commit = 'different';
    fs.writeFileSync(path.join(licenses, 'SOURCE-MANIFEST.json'), JSON.stringify(manifest));
    assert.throws(() => verifyMaterials(root, { engineBuildId: 'engine' }, commit), /do not match/);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('targets map to Rust triples and platform bundles', () => {
  assert.deepEqual(specFor('darwin-arm64'), {
    triple: 'aarch64-apple-darwin',
    bundles: ['app'],
    platform: 'macos',
    artifact: 'macos-arm64',
  });
  assert.equal(specFor('mingw-x86_64').triple, 'x86_64-pc-windows-msvc');
  assert.equal(specFor('linux-arm64').triple, 'aarch64-unknown-linux-gnu');
  assert.deepEqual(specFor('linux-x86_64').bundles, ['deb', 'rpm']);
});

test('clearing bundle outputs removes stale installers', () => {
  const root = temporaryRoot();
  try {
    assert.deepEqual(bundleOutputDirectories(root, 'darwin-arm64'), [
      path.join(root, 'target/aarch64-apple-darwin/release/bundle/macos'),
    ]);
    assert.deepEqual(bundleOutputDirectories(root, 'linux-x86_64'), [
      path.join(root, 'target/x86_64-unknown-linux-gnu/release/bundle/deb'),
      path.join(root, 'target/x86_64-unknown-linux-gnu/release/bundle/rpm'),
    ]);

    const [nsis] = bundleOutputDirectories(root, 'mingw-x86_64');
    const sibling = path.join(path.dirname(nsis), 'msi');
    fs.mkdirSync(nsis, { recursive: true });
    fs.mkdirSync(sibling, { recursive: true });
    fs.writeFileSync(path.join(nsis, 'Ossia_0.1.0-beta.18_x64-setup.exe'), '');
    fs.writeFileSync(path.join(sibling, 'keep.txt'), '');

    clearBundleOutputs(root, 'mingw-x86_64');

    assert.equal(fs.existsSync(nsis), false);
    assert.equal(fs.existsSync(path.join(sibling, 'keep.txt')), true);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('payload inspection rejects missing audio samples', () => {
  const root = temporaryRoot();
  try {
    writePayload(root, path.join(root, 'ossia'));
    fs.rmSync(path.join(root, 'ui/audio/test.mp3'));
    assert.throws(() => inspectPayload(root), /piano sample is missing: test.mp3/);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('unbuilt linux arm64 is rejected instead of producing a fake artifact', () => {
  const lock = {
    targets: {
      'linux-arm64': { archive: null },
      'darwin-arm64': { archive: { sha256: 'a'.repeat(64) } },
    },
  };
  assert.throws(() => assertBuildable('linux-arm64', lock), /no built engine archive/);
  assert.equal(assertBuildable('darwin-arm64', lock).archive.sha256.length, 64);
});

test('payload inspection accepts macOS, Windows and Linux install layouts', () => {
  const root = temporaryRoot();
  try {
    const macResources = path.join(root, 'mac/Ossia.app/Contents/Resources');
    const macExecutable = path.join(root, 'mac/Ossia.app/Contents/MacOS/ossia');
    writePayload(macResources, macExecutable);
    assert.deepEqual(inspectPayload(path.join(root, 'mac')), {
      executable: macExecutable,
      resources: macResources,
    });

    const windowsRoot = path.join(root, 'windows');
    const windowsExecutable = path.join(windowsRoot, 'ossia.exe');
    writePayload(windowsRoot, windowsExecutable);
    assert.deepEqual(inspectPayload(windowsRoot), {
      executable: windowsExecutable,
      resources: windowsRoot,
    });

    const linuxRoot = path.join(root, 'linux');
    const linuxResources = path.join(linuxRoot, 'usr/lib/ossia');
    const linuxExecutable = path.join(linuxRoot, 'usr/bin/ossia');
    writePayload(linuxResources, linuxExecutable);
    assert.deepEqual(inspectPayload(linuxRoot), {
      executable: linuxExecutable,
      resources: linuxResources,
    });
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('payload inspection rejects a package without documentation', () => {
  const root = temporaryRoot();
  try {
    const resources = path.join(root, 'Ossia');
    const executable = path.join(resources, 'ossia');
    writePayload(resources, executable);
    fs.rmSync(path.join(resources, 'docs'), { recursive: true });
    assert.throws(() => inspectPayload(root), /missing docs\/lilypond/);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('windows smoke installs do not create user shortcuts', () => {
  assert.deepEqual(windowsInstallerArgs('C:\\Temp\\Ossia'), ['/S', '/NS', '/D=C:\\Temp\\Ossia']);
});

function registry(args) {
  const result = spawnSync('reg', args, {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(result.stderr.trim());
  return result.stdout;
}

test(
  'windows smoke state restores installer registry keys',
  { skip: process.platform !== 'win32', timeout: 60_000 },
  async () => {
    const suffix = `${process.pid}-${Date.now()}`;
    const manufacturer = `OssiaStateTest-${suffix}`;
    const productName = `OssiaStateTest-${suffix}`;
    const software = `HKCU\\Software\\${manufacturer}`;
    const uninstall = `HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${productName}`;
    try {
      registry(['add', software, '/ve', '/d', 'before', '/f']);
      registry(['add', uninstall, '/v', 'DisplayVersion', '/d', '1.2.3', '/f']);

      await withWindowsInstallState({ productName, manufacturer }, async () => {
        registry(['add', software, '/ve', '/d', 'during', '/f']);
        registry(['add', uninstall, '/v', 'DisplayVersion', '/d', '9.9.9', '/f']);
        registry(['add', uninstall, '/v', 'Extra', '/d', 'temporary', '/f']);
      });

      assert.match(registry(['query', software, '/ve']), /before/);
      assert.match(registry(['query', uninstall, '/v', 'DisplayVersion']), /1\.2\.3/);
      assert.throws(() => registry(['query', uninstall, '/v', 'Extra']));
    } finally {
      spawnSync('reg', ['delete', software, '/f'], { stdio: 'ignore', windowsHide: true });
      spawnSync('reg', ['delete', uninstall, '/f'], { stdio: 'ignore', windowsHide: true });
    }
  },
);

test(
  'windows smoke state removes installer registry keys it created',
  { skip: process.platform !== 'win32', timeout: 60_000 },
  async () => {
    const suffix = `${process.pid}-${Date.now()}`;
    const manufacturer = `OssiaStateMissing-${suffix}`;
    const productName = `OssiaStateMissing-${suffix}`;
    const software = `HKCU\\Software\\${manufacturer}`;
    const uninstall = `HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${productName}`;

    await withWindowsInstallState({ productName, manufacturer }, async () => {
      registry(['add', software, '/ve', '/d', 'temporary', '/f']);
      registry(['add', uninstall, '/v', 'DisplayVersion', '/d', '9.9.9', '/f']);
    });

    assert.throws(() => registry(['query', software]));
    assert.throws(() => registry(['query', uninstall]));
  },
);
