import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';
import { test } from 'node:test';

function load(source, dependencies, globals = {}) {
  const exports = {};
  const code = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2021, module: ts.ModuleKind.CommonJS } }).outputText;
  vm.runInNewContext(code, { exports, $state: value => value, console, require: name => {
    if (!(name in dependencies)) throw new Error(`Unexpected dependency: ${name}`);
    return dependencies[name];
  }, ...globals });
  return exports;
}
const noop = () => {};
const tick = async () => { for (let index = 0; index < 20; index++) await Promise.resolve(); };
const dataSource = readFileSync('src/lib/stores/dataManagement.svelte.ts', 'utf8');

test('Data tab switching ignores old Overview errors and loading while Duplicates completes', async () => {
  const requests = [];
  const api = load(dataSource, {
    '@tauri-apps/api/core': { invoke: command => new Promise((resolve, reject) => requests.push({ command, resolve, reject })) },
    '../utils/logger': { logger: { error: noop } },
  });
  api.setActiveTab('overview');
  api.setActiveTab('duplicates');
  requests[1].resolve([{ show1_id: 1, show2_id: 2 }]);
  await tick();
  assert.equal(api.isLoading(), false);
  assert.equal(api.getDuplicates().length, 1);
  requests[0].reject(new Error('old summary failure'));
  await tick();
  assert.equal(api.getError(), null);
  assert.equal(api.getDuplicates().length, 1);
});

test('cleanup preview failure is visible and retry replaces stale or failed previews', async () => {
  let fail = true;
  const api = load(dataSource, {
    '@tauri-apps/api/core': { invoke: async command => { if (fail) throw new Error('preview unavailable'); return [{ id: command.includes('orphaned') ? 1 : 2 }]; } },
    '../utils/logger': { logger: { error: noop } },
  });
  assert.equal(await api.loadCleanupPreviews(), false);
  assert.match(api.getCleanupPreviewError(), /preview unavailable/);
  assert.equal(api.getOrphanedEpisodes().length, 0);
  assert.equal(api.isCleanupPreviewLoading(), false);
  fail = false;
  assert.equal(await api.loadCleanupPreviews(), true);
  assert.equal(api.getCleanupPreviewError(), null);
  assert.equal(api.getOrphanedEpisodes()[0].id, 1);
  assert.equal(api.getUnairedEpisodes()[0].id, 2);
});

function libraries(calls, failed = false) {
  const track = name => async () => { calls.push(name); return true; };
  let show = { id: 2 }, movie = { id: 21 };
  return {
    './shows.svelte': { loadTrackedShows: track('tracked shows'), loadArchivedShows: track('archived shows'), refreshCalendar: track('episode calendar') },
    './tiers.svelte': { loadTierListShows: async () => { calls.push('show tiers'); return !failed; }, loadTierListMovies: track('movie tiers') },
    './movies.svelte': { loadTrackedMovies: track('tracked movies'), loadArchivedMovies: track('archived movies'), refreshMoviesCalendar: track('movie calendar'), isMovieDetailOpen: () => true, getCurrentMovie: () => movie, openMovieDetail: async id => { calls.push('movie detail'); movie = { id }; } },
    './showDetail.svelte': { isShowDetailOpen: () => true, getCurrentShow: () => show, openShowDetail: async id => { calls.push(`show detail ${id}`); show = { id }; } },
  };
}

test('merge refreshes both persistent libraries, calendars and the deleted show detail into its keeper', async () => {
  const calls = [];
  const api = load(dataSource, {
    '@tauri-apps/api/core': { invoke: async command => command === 'merge_duplicates' ? { deleted_show_id: 2, episodes_moved: 1, episodes_merged: 0 } : command === 'find_duplicates' ? [] : {} },
    '../utils/logger': { logger: { error: noop } }, ...libraries(calls),
  });
  await api.mergeDuplicates(1, 2);
  for (const label of ['tracked shows', 'archived shows', 'show tiers', 'tracked movies', 'archived movies', 'movie tiers', 'episode calendar', 'movie calendar', 'show detail 1', 'movie detail']) assert.ok(calls.includes(label), label);
  assert.equal(api.getError(), null);
});

test('committed cleanup or merge with failed refresh reports saved changes rather than false success or rollback', async () => {
  const api = load(dataSource, {
    '@tauri-apps/api/core': { invoke: async command => command === 'cleanup_orphaned_episodes' ? 3 : command === 'merge_duplicates' ? { deleted_show_id: 2 } : command === 'find_duplicates' ? [] : command.endsWith('_preview') ? [] : {} },
    '../utils/logger': { logger: { error: noop } }, ...libraries([], true),
  });
  await assert.rejects(api.cleanupOrphaned(), /Removed 3.*refreshing Data failed/);
  assert.match(api.getError(), /Changes were saved/);
  await assert.rejects(api.mergeDuplicates(1, 2), /shows were merged.*refreshing Data failed/);
  assert.equal(api.isLoading(), false);
});

test('resolver searches discard old, cleared and closed results and expose current search errors', async () => {
  const requests = [], timers = new Map();
  let nextTimer = 0;
  const component = readFileSync('src/lib/components/UnmigratedShowsResolver.svelte', 'utf8').match(/<script lang="ts">([\s\S]*?)<\/script>/)[1];
  const source = component + '\nexport const harness = { input:handleQueryInput, close:closeResolver, snapshot:()=>({results,searchErrors,searching}) };';
  const api = load(source, {
    'svelte': { onDestroy: noop, untrack: callback => callback() }, 'svelte/transition': {}, 'lucide-svelte': {},
    '../stores/confirmDialog.svelte': {}, '../utils/modalFocus': {}, '../config': { config: { ui: { searchDebounceMs: 1 } } }, '../utils/logger': { logger: { error: noop } }, '../utils/dateFormat': {},
    '../stores/migration.svelte': { isUnmigratedShowBusy: () => false, searchTmdbTv: query => new Promise((resolve, reject) => requests.push({ query, resolve, reject })) },
  }, { $props: () => ({ open: true }), $effect: noop, setTimeout: callback => { const id = ++nextTimer; timers.set(id, callback); return id; }, clearTimeout: id => timers.delete(id) });
  const fire = async () => { const callbacks = [...timers.values()]; timers.clear(); callbacks.forEach(callback => void callback()); await tick(); };
  api.harness.input(1, 'old'); await fire();
  api.harness.input(1, 'new'); await fire();
  requests[1].resolve([{ id: 2 }]); await tick();
  requests[0].reject(new Error('old failure')); await tick();
  assert.equal(api.harness.snapshot().results[1][0].id, 2);
  assert.equal(api.harness.snapshot().searchErrors[1], undefined);
  api.harness.input(1, 'pending'); await fire();
  api.harness.input(1, ''); requests[2].resolve([{ id: 3 }]); await tick();
  assert.equal(api.harness.snapshot().results[1], undefined);
  assert.equal(api.harness.snapshot().searching.size, 0);
  api.harness.input(1, 'closing'); await fire(); api.harness.close(); requests[3].resolve([{ id: 4 }]); await tick();
  assert.equal(api.harness.snapshot().results[1], undefined);
  api.harness.input(1, 'offline'); await fire(); requests[4].reject(new Error('network offline')); await tick();
  assert.match(api.harness.snapshot().searchErrors[1], /network offline/);
});

test('resolver resolve and delete share a row guard; Delete waits for common confirmation', async () => {
  let finishResolve, finishConfirm;
  let confirmations = 0, deletes = 0;
  const component = readFileSync('src/lib/components/UnmigratedShowsResolver.svelte', 'utf8').match(/<script lang="ts">([\s\S]*?)<\/script>/)[1];
  const api = load(component + '\nexport const harness = { resolve:handleResolve, delete:handleDelete };', {
    'svelte': { onDestroy: noop, untrack: callback => callback() }, 'svelte/transition': {}, 'lucide-svelte': {},
    '../stores/confirmDialog.svelte': { openConfirmDialog: () => { confirmations++; return new Promise(resolve => finishConfirm = resolve); } },
    '../utils/modalFocus': {}, '../config': {}, '../utils/logger': { logger: { error: noop } }, '../utils/dateFormat': {},
    '../stores/migration.svelte': { isUnmigratedShowBusy: () => false, resolveUnmigratedShow: () => new Promise(resolve => finishResolve = resolve), deleteUnmigratedShow: async () => { deletes++; } },
  }, { $props: () => ({ open: true }), $effect: noop, clearTimeout: noop });
  const resolving = api.harness.resolve({ id: 1, name: 'Legacy' }, 101);
  await api.harness.delete({ id: 1, name: 'Legacy' });
  assert.equal(confirmations, 0);
  finishResolve({}); await resolving;
  const deleting = api.harness.delete({ id: 1, name: 'Legacy' });
  assert.equal(confirmations, 1); assert.equal(deletes, 0);
  await api.harness.resolve({ id: 1, name: 'Legacy' }, 101);
  finishConfirm(false); await deleting;
  assert.equal(deletes, 0);
});
