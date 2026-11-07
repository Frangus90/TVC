import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import vm from 'node:vm';
import * as dateFns from 'date-fns';
import ts from 'typescript';

const compilerOptions = { target: ts.ScriptTarget.ES2021, module: ts.ModuleKind.CommonJS };
const compile = path => ts.transpileModule(readFileSync(path, 'utf8'), { compilerOptions }).outputText;
const dateSource = compile('src/lib/utils/dateFormat.ts');
const racingSource = compile('src/lib/stores/racing.svelte.ts');
const syncSource = compile('src/lib/stores/librarySync.ts');

function load(source, dependencies) {
  const exports = {};
  vm.runInNewContext(source, {
    exports,
    $state: value => value,
    require: name => {
      assert.ok(name in dependencies, `Unexpected test dependency: ${name}`);
      return dependencies[name];
    },
  });
  return exports;
}

function setup({ refresh = async () => {} } = {}) {
  const requests = [], errors = [], libraryReloads = [];
  const listeners = new Map();
  let startTime = '2026-11-07T17:00:00Z';
  const dates = load(dateSource, { 'date-fns': dateFns });
  const toast = { showError: message => errors.push(message) };
  const emit = kind => listeners.get('library-sync-finished')?.({ payload: kind });
  const racing = load(racingSource, {
    '../utils/dateFormat': dates,
    '../utils/database': { getDatabase: () => { throw new Error('No database in the IPC fixture'); } },
    '../utils/logger': { logger: { error() {} } },
    './toast.svelte': toast,
    '@tauri-apps/api/core': { invoke: async (command, args) => {
      requests.push([command, structuredClone(args)]);
      if (command === 'get_racing_events_for_range') {
        return [{ id: 1, event_title: 'Qatar Grand Prix', series_slug: 'motogp', start_time: startTime }];
      }
      await refresh(command);
      if (command === 'run_library_sync') {
        startTime = '2026-11-07T16:00:00Z';
        emit(args.kind);
      } else if (command === 'refresh_single_racing_series') {
        startTime = '2026-11-07T16:00:00Z';
      } else assert.fail(`Unexpected IPC: ${command}`);
    } },
  });
  const librarySync = load(syncSource, {
    '@tauri-apps/api/event': { listen: async (event, callback) => {
      listeners.set(event, callback);
      return () => listeners.delete(event);
    } },
    './racing.svelte': racing,
    './shows.svelte': { loadTrackedShows: async () => libraryReloads.push('shows') },
    './movies.svelte': { isMovieDetailOpen: () => { libraryReloads.push('movies'); return false; } },
    './showDetail.svelte': { isShowDetailOpen: () => false },
    './tiers.svelte': {},
    './toast.svelte': toast,
  });
  const stop = librarySync.setupLibrarySyncListener();
  return {
    dates, racing, requests, errors, libraryReloads, emit, stop,
    publishCorrectedFeed: () => { startTime = '2026-11-07T16:00:00Z'; },
  };
}

const reload = () => new Promise(resolve => setImmediate(resolve));

test('a background Racing sync updates the open calendar with the corrected winter session time', async () => {
  const previousTz = process.env.TZ;
  process.env.TZ = 'Europe/Oslo';
  const app = setup();
  try {
    await app.racing.loadRacingEventsForRange('2026-11-07', '2026-11-08');
    const sprintTime = () => app.dates.formatTime(app.racing.getRacingEvents()[0].start_time);
    assert.equal(sprintTime(), '18:00');
    app.publishCorrectedFeed();
    app.emit('racing');
    await reload();
    assert.equal(sprintTime(), '17:00');
    assert.deepEqual(app.requests, Array(2).fill(['get_racing_events_for_range', {
      start: '2026-11-06T23:00:00.000Z', end: '2026-11-08T23:00:00.000Z',
    }]));
    assert.deepEqual(app.libraryReloads, []);
    assert.deepEqual(app.errors, []);
  } finally {
    app.stop();
    if (previousTz === undefined) delete process.env.TZ;
    else process.env.TZ = previousTz;
  }
});

test('Racing Refresh runs the shared sync, then its completion updates the calendar', async () => {
  let finish;
  const pending = new Promise(resolve => { finish = resolve; });
  const app = setup({ refresh: () => pending });
  await app.racing.loadRacingEventsForRange('2026-11-07', '2026-11-08');
  const running = app.racing.refreshRacingData();
  assert.equal(app.racing.isRacingRefreshing(), true);
  assert.deepEqual(app.requests[1], ['run_library_sync', { kind: 'racing', retryFailed: false }]);
  finish();
  await running;
  await reload();
  assert.equal(app.racing.isRacingRefreshing(), false);
  assert.equal(app.racing.getRacingEvents()[0].start_time, '2026-11-07T16:00:00Z');
  assert.deepEqual(app.errors, []);
  app.stop();
});

test('refreshing one Racing series still reloads the calendar without syncing the other feeds', async () => {
  const app = setup();
  await app.racing.loadRacingEventsForRange('2026-11-07', '2026-11-08');
  await app.racing.refreshSingleSeries('motogp');
  assert.deepEqual(app.requests.map(([command]) => command), [
    'get_racing_events_for_range', 'refresh_single_racing_series', 'get_racing_events_for_range',
  ]);
  assert.deepEqual(app.requests[1][1], { slug: 'motogp' });
  assert.equal(app.racing.getRacingEvents()[0].start_time, '2026-11-07T16:00:00Z');
  assert.deepEqual(app.errors, []);
  app.stop();
});
