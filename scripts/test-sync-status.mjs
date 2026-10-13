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
const librarySync = exports;
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
    svelte: { onMount() {}, tick: async () => {} },
    '@tauri-apps/api/core': { invoke: async (command, args) => {
      if (command === 'set_library_sync_schedule') { frequency = args.frequency; failStatus = true; return; }
      if (failStatus) throw new Error('status unavailable');
      return [{ kind: 'movies', frequency, running: false, report: null }];
    } },
    '../../stores/dataManagement.svelte': { setActiveTab() {} },
    '../../stores/sidebar.svelte': { setSidebarTab() {} },
    '../../stores/librarySync': librarySync,
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

function makeSyncTab(invoke) {
  const script = readFileSync('src/lib/components/data/SyncTab.svelte', 'utf8').match(/<script[^>]*>([\s\S]*?)<\/script>/)[1];
  const code = ts.transpileModule(`${script}\nexports.syncTest = { refresh, setFailureAccepted, toggleHistory, loadHistory, read: () => ({ libraries, loadError, actionError, failureFeedback, changingFailure, histories }) };`, {
    compilerOptions: { target: ts.ScriptTarget.ES2021, module: ts.ModuleKind.CommonJS },
  }).outputText;
  const result = {};
  const dependencies = {
    svelte: { onMount() {}, tick: async () => {} },
    '@tauri-apps/api/core': { invoke },
    '../../stores/librarySync': librarySync,
    '../../stores/dataManagement.svelte': { setActiveTab() {} },
    '../../stores/sidebar.svelte': { setSidebarTab() {} },
  };
  vm.runInNewContext(code, { exports: result, $state: value => value, $derived: value => value, require: name => dependencies[name] });
  return result.syncTest;
}

function syncReport(kind = 'movies', started_at = '2026-10-13T12:00:00Z') {
  return {
    kind, source: 'manual', started_at, finished_at: '2026-10-13T12:01:00Z',
    total: 2, succeeded: 1, skipped: 0, failures: [{ id: 42, title: 'Alpha', error: 'TMDB timed out' }],
    current_title: null, error: null,
  };
}

function syncStatus(report) {
  return {
    kind: report.kind, frequency: 'weekly', running: false, interrupted: false,
    last_success_at: '2026-10-06T12:00:00Z', next_sync_at: '2026-10-20T12:00:00Z',
    report, last_full_report: report,
  };
}

function withAcceptance(report, accepted) {
  return { ...report, failures: report.failures.map(failure => ({ ...failure, accepted })) };
}

function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

test('accepted failures are separate from attention, with older reports remaining unaccepted and totals intact', () => {
  const report = syncReport();
  report.failures.push({ id: 43, title: 'Beta', error: 'Not found', accepted: false });
  report.failures.push({ id: 44, title: 'Gamma', error: 'Not found', accepted: true });
  assert.deepEqual([...exports.unacceptedSyncFailures(report)].map(failure => failure.id), [42, 43]);
  assert.deepEqual([...exports.acceptedSyncFailures(report)].map(failure => failure.id), [44]);
  assert.equal(report.failures.length, 3);
  for (const missing of [null, undefined]) {
    assert.equal(exports.unacceptedSyncFailures(missing).length, 0);
    assert.equal(exports.acceptedSyncFailures(missing).length, 0);
  }
});

test('a failed acceptance write leaves the failure actionable and reports the save error', async () => {
  const report = syncReport();
  const api = makeSyncTab(async command => {
    if (command === 'get_library_sync_status') return [syncStatus(report)];
    if (command === 'set_library_sync_failure_accepted') throw new Error('database is busy');
    throw new Error(`Unexpected command: ${command}`);
  });
  await api.refresh();
  await api.setFailureAccepted('movies', report, report.failures[0], true);
  assert.equal(exports.unacceptedSyncFailures(api.read().libraries[0].report).length, 1);
  assert.equal(api.read().libraries[0].report.failures[0].accepted, undefined);
  assert.match(api.read().actionError, /Could not accept this failure: Error: database is busy/);
  assert.equal(api.read().failureFeedback, null);
  assert.equal(api.read().changingFailure, null);
});

test('acceptance waits for an older status read and then publishes the saved result for only its library', async () => {
  const report = syncReport();
  const showReport = syncReport('shows');
  const saved = withAcceptance(report, true);
  const pendingRead = deferred();
  let reads = 0, savedArgs;
  const api = makeSyncTab(async (command, args) => {
    if (command === 'get_library_sync_status') return ++reads === 1 ? [syncStatus(report), syncStatus(showReport)] : pendingRead.promise;
    if (command === 'set_library_sync_failure_accepted') { savedArgs = args; return saved; }
    throw new Error(`Unexpected command: ${command}`);
  });
  await api.refresh();
  const staleRead = api.refresh();
  const accepting = api.setFailureAccepted('movies', report, report.failures[0], true);
  await Promise.resolve();
  assert.equal(api.read().changingFailure, 'movies:42');
  pendingRead.resolve([syncStatus(report), syncStatus(showReport)]);
  await Promise.all([staleRead, accepting]);
  const movie = api.read().libraries.find(library => library.kind === 'movies');
  const show = api.read().libraries.find(library => library.kind === 'shows');
  assert.equal(movie.report.failures[0].accepted, true);
  assert.equal(movie.last_full_report.failures[0].accepted, true);
  assert.equal(show.report.failures[0].accepted, undefined, 'the same title ID in another library is untouched');
  assert.equal(movie.report.failures.length, 1, 'the failed total stays truthful');
  assert.equal(movie.last_success_at, '2026-10-06T12:00:00Z');
  assert.equal(movie.next_sync_at, '2026-10-20T12:00:00Z');
  assert.deepEqual({ ...savedArgs }, { kind: 'movies', startedAt: report.started_at, id: 42, accepted: true });
  assert.match(api.read().failureFeedback, /Failure accepted for Alpha.*future full syncs/);
  assert.equal(api.read().changingFailure, null);
});

test('a saved acceptance cannot replace a newer run returned by a pending status read', async () => {
  const report = syncReport();
  const newer = syncReport('movies', '2026-10-13T12:02:00Z');
  const pendingRead = deferred();
  let reads = 0;
  const api = makeSyncTab(async command => {
    if (command === 'get_library_sync_status') return ++reads === 1 ? [syncStatus(report)] : pendingRead.promise;
    if (command === 'set_library_sync_failure_accepted') return withAcceptance(report, true);
    throw new Error(`Unexpected command: ${command}`);
  });
  await api.refresh();
  const staleRead = api.refresh();
  const accepting = api.setFailureAccepted('movies', report, report.failures[0], true);
  pendingRead.resolve([syncStatus(newer)]);
  await Promise.all([staleRead, accepting]);
  assert.equal(api.read().libraries[0].report.started_at, newer.started_at);
  assert.equal(api.read().libraries[0].report.failures[0].accepted, undefined);
  assert.equal(api.read().libraries[0].last_full_report.started_at, newer.started_at);
});

test('open history reloads acceptance and Undo even though the run timestamps do not change', async () => {
  let report = syncReport(), historyReads = 0;
  const api = makeSyncTab(async (command, args) => {
    if (command === 'get_library_sync_status') return [syncStatus(report)];
    if (command === 'get_library_sync_history') { historyReads++; return [report]; }
    if (command === 'set_library_sync_failure_accepted') { report = withAcceptance(report, args.accepted); return report; }
    throw new Error(`Unexpected command: ${command}`);
  });
  await api.refresh();
  api.toggleHistory('movies', { currentTarget: { open: true } });
  await api.loadHistory('movies');
  const originalTimestamp = report.started_at;
  const originalFinish = report.finished_at;
  await api.setFailureAccepted('movies', report, report.failures[0], true);
  assert.equal(historyReads, 2);
  assert.equal(api.read().histories.movies[0].failures[0].accepted, true);
  await api.setFailureAccepted('movies', report, report.failures[0], false);
  assert.equal(historyReads, 3);
  assert.equal(api.read().histories.movies[0].failures[0].accepted, false);
  assert.equal(api.read().libraries[0].report.started_at, originalTimestamp);
  assert.equal(api.read().libraries[0].report.finished_at, originalFinish);
  assert.equal(exports.unacceptedSyncFailures(api.read().libraries[0].report).length, 1);
  assert.match(api.read().failureFeedback, /Acceptance undone for Alpha.*retry/);
});

test('a pending failure change prevents a second acceptance request', async () => {
  const report = syncReport();
  const pendingWrite = deferred();
  let writes = 0;
  const api = makeSyncTab(async command => {
    if (command === 'get_library_sync_status') return [syncStatus(report)];
    if (command === 'set_library_sync_failure_accepted') { writes++; return pendingWrite.promise; }
    throw new Error(`Unexpected command: ${command}`);
  });
  await api.refresh();
  const accepting = api.setFailureAccepted('movies', report, report.failures[0], true);
  await api.setFailureAccepted('movies', report, report.failures[0], false);
  assert.equal(writes, 1);
  assert.equal(api.read().changingFailure, 'movies:42');
  pendingWrite.resolve(withAcceptance(report, true));
  await accepting;
  assert.equal(api.read().libraries[0].report.failures[0].accepted, true);
  assert.equal(api.read().changingFailure, null);
});

function makeStatusView(component, invoke) {
  const script = readFileSync(component, 'utf8').match(/<script[^>]*>([\s\S]*?)<\/script>/)[1];
  const sidebar = component.endsWith('Sidebar.svelte');
  const read = sidebar ? '({ syncStatus, syncError })' : '({ sync, syncError })';
  const code = ts.transpileModule(`${script}\nexports.statusTest = { read: () => ${read} };`, {
    compilerOptions: { target: ts.ScriptTarget.ES2021, module: ts.ModuleKind.CommonJS },
  }).outputText;
  const result = {}, listeners = new Map();
  let mount;
  const dependencies = {
    svelte: { onMount: callback => { mount = callback; } },
    '@tauri-apps/api/core': { invoke },
    '@tauri-apps/api/event': { listen: async (event, callback) => { listeners.set(event, callback); return () => {}; } },
    '../../stores/librarySync': librarySync,
    '../../stores/sidebar.svelte': { loadSidebarWidth: async () => {} },
    '../../stores/dataManagement.svelte': { getDatabaseStats: () => null },
    '../../stores/backups.svelte': { getBackupStatus: () => null, loadBackupStatus: async () => {} },
    '../../stores/migration.svelte': { loadUnmigratedShows: async () => {} },
    '../../utils/logger': { logger: { error() {} } },
  };
  vm.runInNewContext(code, {
    exports: result, $state: value => value, $derived: value => value,
    setInterval: () => 1, clearInterval() {}, require: name => dependencies[name],
  });
  return { ...result.statusTest, mount: () => mount(), emit: event => listeners.get(event)() };
}

async function flushMicrotasks() {
  for (let i = 0; i < 12; i++) await Promise.resolve();
}

test('an older sidebar read cannot restore an attention badge after the accepted-status event', async () => {
  const report = syncReport();
  const initialRead = deferred(), acceptedRead = deferred();
  let reads = 0;
  const api = makeStatusView('src/lib/components/layout/Sidebar.svelte', async command => {
    assert.equal(command, 'get_library_sync_status');
    return ++reads === 1 ? initialRead.promise : acceptedRead.promise;
  });
  api.mount();
  api.emit('library-sync-status-changed');
  acceptedRead.resolve([syncStatus(withAcceptance(report, true))]);
  await flushMicrotasks();
  assert.equal(api.read().syncStatus[0].report.failures[0].accepted, true);
  initialRead.resolve([syncStatus(report)]);
  await flushMicrotasks();
  assert.equal(api.read().syncStatus[0].report.failures[0].accepted, true);
  assert.equal(api.read().syncError, false);
});

test('Overview refreshes accepted status after an event arriving during its initial health read', async () => {
  const report = syncReport();
  const initialRead = deferred();
  let reads = 0;
  const api = makeStatusView('src/lib/components/data/OverviewTab.svelte', async command => {
    assert.equal(command, 'get_library_sync_status');
    return ++reads === 1 ? initialRead.promise : [syncStatus(withAcceptance(report, true))];
  });
  api.mount();
  const notified = api.emit('library-sync-status-changed');
  initialRead.resolve([syncStatus(report)]);
  await notified;
  await flushMicrotasks();
  assert.equal(reads, 2, 'the saved-status event is not lost while the old request finishes');
  assert.equal(api.read().sync[0].report.failures[0].accepted, true);
  assert.equal(api.read().syncError, null);
});
