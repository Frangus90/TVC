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

test('a failed loader result after sync is surfaced and does not reopen the current detail', async () => {
  let listener, reopened = 0;
  const errors = [];
  const result = {};
  const dependencies = {
    '@tauri-apps/api/event': { listen: async (_, callback) => { listener = callback; return () => {}; } },
    './shows.svelte': { loadTrackedShows: async () => false, loadArchivedShows: async () => true, refreshCalendar: async () => {} },
    './movies.svelte': {},
    './showDetail.svelte': { isShowDetailOpen: () => true, getCurrentShow: () => ({ id: 42 }), openShowDetail: async () => { reopened++; } },
    './tiers.svelte': { loadTierListShows: async () => true },
    './toast.svelte': { showError: message => errors.push(message) },
  };
  vm.runInNewContext(source, { exports: result, require: name => dependencies[name] });
  const stop = result.setupLibrarySyncListener();
  listener({ payload: 'shows' });
  for (let i = 0; i < 12; i++) await Promise.resolve();
  assert.equal(reopened, 0);
  assert.equal(errors.length, 1);
  assert.match(errors[0], /TV show views could not reload/);
  stop();
});

test('a saved sync schedule stays visible when status readback fails, and recovery clears the notice', async () => {
  const script = readFileSync('src/lib/components/data/SyncTab.svelte', 'utf8').match(/<script[^>]*>([\s\S]*?)<\/script>/)[1];
  const code = ts.transpileModule(`${script}\nexports.syncTest = { refresh, saveSchedule, read: () => ({ libraries, loadError, actionError, savedScheduleStatusUnavailable }) };`, {
    compilerOptions: { target: ts.ScriptTarget.ES2021, module: ts.ModuleKind.CommonJS },
  }).outputText;
  let failStatus = false, frequency = 'weekly';
  const exports = {};
  const dependencies = {
    svelte: { onMount() {} },
    '@tauri-apps/api/core': { invoke: async (command, args) => {
      if (command === 'set_library_sync_schedule') { frequency = args.frequency; failStatus = true; return; }
      if (failStatus) throw new Error('status unavailable');
      return [{ kind: 'movies', frequency, running: false, report: null }];
    } },
    '../../stores/dataManagement.svelte': { setActiveTab() {} },
    '../../stores/sidebar.svelte': { setSidebarTab() {} },
  };
  vm.runInNewContext(code, { exports, $state: value => value, $derived: value => value, require: name => dependencies[name] });
  const api = exports.syncTest;
  await api.refresh();
  const select = { value: 'monthly' };
  await api.saveSchedule(api.read().libraries[0], select);
  assert.equal(api.read().libraries[0].frequency, 'monthly');
  assert.equal(select.value, 'monthly');
  assert.equal(api.read().actionError, null, 'a readback failure is not a failed save');
  assert.equal(api.read().savedScheduleStatusUnavailable, true);
  assert.match(api.read().loadError, /status unavailable/);
  failStatus = false;
  await api.refresh();
  assert.equal(api.read().loadError, null);
  assert.equal(api.read().savedScheduleStatusUnavailable, false);
});
