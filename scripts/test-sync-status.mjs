import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import vm from 'node:vm';
import ts from 'typescript';

const source = ts.transpileModule(readFileSync('src/lib/stores/librarySync.ts', 'utf8'), {
  compilerOptions: { target: ts.ScriptTarget.ES2021, module: ts.ModuleKind.CommonJS },
}).outputText;
const exports = {};
vm.runInNewContext(source, { exports, require: () => ({}) });
const status = (kind, synced) => ({ kind, last_success_at: synced ? '2026-10-13T12:00:00Z' : null });

test('the shared library badge requires both distinct libraries to have completed a successful sync', () => {
  const { allLibrariesHaveSynced } = exports;
  assert.equal(allLibrariesHaveSynced([]), false);
  assert.equal(allLibrariesHaveSynced([status('shows', true)]), false);
  assert.equal(allLibrariesHaveSynced([status('shows', true), status('shows', true)]), false);
  assert.equal(allLibrariesHaveSynced([status('shows', true), status('movies', false)]), false);
  assert.equal(allLibrariesHaveSynced([status('shows', false), status('movies', true)]), false);
  assert.equal(allLibrariesHaveSynced([status('shows', true), status('movies', true)]), true);
});
