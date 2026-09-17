import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

const helper = fileURLToPath(new URL('./sync-release-lockfiles.ps1', import.meta.url));

test('release lock files recover an interrupted version bump and stay synchronized', {
  skip: process.platform !== 'win32' && 'The release script uses Windows PowerShell',
}, () => {
  const fixture = mkdtempSync(join(tmpdir(), 'tvc-release-lock-test-'));
  const run = (command, args) => {
    const result = spawnSync(command, args, { cwd: fixture, encoding: 'utf8' });
    assert.ifError(result.error);
    assert.equal(result.status, 0, result.stdout + result.stderr);
    return result.stdout;
  };
  const read = path => readFileSync(join(fixture, path), 'utf8');
  const write = (path, value) => writeFileSync(join(fixture, path), value);
  const sync = () => run('powershell.exe', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', helper, '-ProjectRoot', fixture]);
  try {
    mkdirSync(join(fixture, 'src-tauri', 'src'), { recursive: true });
    mkdirSync(join(fixture, 'dependency', 'src'), { recursive: true });
    write('src-tauri/src/lib.rs', '');
    write('dependency/src/lib.rs', '');
    write('dependency/Cargo.toml', '[package]\nname = "fixture-dependency"\nversion = "1.2.3"\nedition = "2021"\n');
    const manifest = version => `[package]\nname = "release-lockfile-fixture"\nversion = "${version}"\nedition = "2021"\n[dependencies]\nfixture-dependency = { path = "../dependency" }\n`;
    write('src-tauri/Cargo.toml', manifest('0.14.5'));
    run('cargo', ['generate-lockfile', '--offline', '--manifest-path', 'src-tauri/Cargo.toml']);
    const originalCargoLock = read('src-tauri/Cargo.lock');
    const packageJson = { name: 'release-lockfile-fixture', version: '0.14.6', private: true,
      scripts: { version: 'node -e "process.exit(42)"' } };
    const packageLock = { name: packageJson.name, version: '0.14.5', lockfileVersion: 3,
      requires: true, packages: { '': { name: packageJson.name, version: '0.14.5' } } };
    write('package-lock.json', JSON.stringify(packageLock, null, 2) + '\n');

    // Recover a previous bump, then exercise the next release's bump through the same helper.
    for (const version of ['0.14.6', '0.14.7']) {
      packageJson.version = version;
      write('package.json', JSON.stringify(packageJson, null, 2) + '\n');
      write('src-tauri/Cargo.toml', manifest(version));
      sync();
      const expectedCargoLock = originalCargoLock.replace(
        'name = "release-lockfile-fixture"\nversion = "0.14.5"',
        `name = "release-lockfile-fixture"\nversion = "${version}"`,
      );
      assert.equal(read('src-tauri/Cargo.lock'), expectedCargoLock, 'Only the workspace package version should change');
      assert.deepEqual(JSON.parse(read('package.json')), packageJson, 'No lifecycle scripts or manifest changes');
      assert.deepEqual(JSON.parse(read('package-lock.json')), {
        ...packageLock, version, packages: { '': { name: packageJson.name, version } },
      });
      run('cargo', ['metadata', '--locked', '--offline', '--format-version', '1', '--manifest-path', 'src-tauri/Cargo.toml']);
    }
    const locks = [read('src-tauri/Cargo.lock'), read('package-lock.json')];
    sync();
    assert.deepEqual([read('src-tauri/Cargo.lock'), read('package-lock.json')], locks, 'Retrying should be idempotent');
    assert.equal(read('src-tauri/Cargo.toml'), manifest('0.14.7'));
  } finally {
    assert.ok(resolve(fixture).startsWith(resolve(tmpdir()) + sep));
    rmSync(fixture, { recursive: true, force: true });
  }
});
