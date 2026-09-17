import { spawnSync } from 'node:child_process';
import { readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const tests = readdirSync(new URL('./', import.meta.url)).filter(name => /^test-.*\.mjs$/.test(name)).map(name => `scripts/${name}`);
const steps = [
  [process.execPath, ['node_modules/svelte-check/bin/svelte-check', '--tsconfig', './tsconfig.json']],
  [process.execPath, ['--test', ...tests]],
  ['cargo', ['test', '--lib', '--locked', '--offline', '--manifest-path', 'src-tauri/Cargo.toml']],
  [process.execPath, ['node_modules/vite/bin/vite.js', 'build']],
];
for (const [command, args] of steps) {
  const result = spawnSync(command, args, { cwd: root, stdio: 'inherit' });
  if (result.error) console.error(result.error.message);
  if (result.status !== 0) process.exit(result.status ?? 1);
}
