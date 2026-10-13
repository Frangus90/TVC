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

function fixture(kind) {
  const single = kind === 'shows' ? 'show' : 'movie';
  const suffix = kind === 'shows' ? 'Shows' : 'Movies';
  const row = { id: 42, name: 'Saved title', title: 'Saved title', archived: true, tier_only: true, tier_id: 1 };
  const commands = [], messages = [];
  let failWrite = false, failCalendar = false, failFallback = false, failTiers = false, failTracked = false;
  const dependencies = {
    '@tauri-apps/api/core': { invoke: async command => {
      commands.push(command);
      if (command === `unarchive_${single}`) {
        if (failWrite) throw new Error('Write rejected');
        row.archived = false;
        return;
      }
      if (command === `get_archived_${kind}`) return row.archived ? [structuredClone(row)] : [];
      if (command === `get_tracked_${kind}`) {
        if (failTracked) throw new Error('Tracked library unavailable');
        return row.archived || row.tier_only ? [] : [structuredClone(row)];
      }
      if (command === (kind === 'shows' ? 'get_episodes_for_range' : 'get_movies_for_range')) {
        if (failCalendar) throw new Error('Calendar unavailable');
        return row.archived || row.tier_only ? [] : [{ id: 7, show_id: row.id }];
      }
      throw new Error(`Unexpected IPC: ${command}`);
    } },
    '../utils/logger': { logger: { error: noop, debug: noop } },
    '../utils/validation': {}, '../utils/requestDedup': {},
    '../utils/database': { getDatabase: async () => {
      if (failFallback) throw new Error('Fallback unavailable');
      return { select: async () => [] };
    } },
    './toast.svelte': { showError: message => messages.push(message) },
    './tiers.svelte': { [`loadTierList${suffix}`]: async () => { commands.push('refresh tiers'); return !failTiers; } },
  };
  const api = load(readFileSync(`src/lib/stores/${kind}.svelte.ts`, 'utf8'), dependencies);
  return { api, row, commands, messages, suffix, single,
    fail: options => { ({ failWrite = false, failCalendar = false, failFallback = false, failTiers = false, failTracked = false } = options); },
  };
}

for (const kind of ['shows', 'movies']) {
  test(`${kind}: Archive restore refreshes persistent libraries, tiers and the active calendar`, async () => {
    for (const tierOnly of [true, false]) {
      const { api, row, commands, suffix, single } = fixture(kind);
      row.tier_only = tierOnly;
      await api[`loadArchived${suffix}`]();
      await api[kind === 'shows' ? 'loadEpisodesForRange' : 'loadMoviesForRange']('2026-10-01', '2026-10-31');
      commands.length = 0;
      assert.equal(await api[`unarchive${single === 'show' ? 'Show' : 'Movie'}`](row.id), true);
      assert.equal(row.archived, false);
      assert.equal(row.tier_only, tierOnly, 'restore preserves tracking or tier-only membership');
      assert.equal(api[`getArchived${suffix}`]().length, 0);
      assert.equal(api[`getTracked${suffix}`]().length, tierOnly ? 0 : 1, 'persistent navigation sees restored tracked titles immediately');
      assert.equal(api[kind === 'shows' ? 'getCalendarEpisodes' : 'getCalendarMovies']().length, tierOnly ? 0 : 1);
      for (const command of [`unarchive_${single}`, `get_archived_${kind}`, `get_tracked_${kind}`, 'refresh tiers', kind === 'shows' ? 'get_episodes_for_range' : 'get_movies_for_range']) {
        assert.ok(commands.includes(command), command);
      }
    }
  });

  test(`${kind}: rejected restore retains Archive state and never claims the write succeeded`, async () => {
    const data = fixture(kind);
    await data.api[`loadArchived${data.suffix}`]();
    data.commands.length = 0; data.fail({ failWrite: true });
    let message;
    assert.equal(await data.api[`unarchive${kind === 'shows' ? 'Show' : 'Movie'}`](42, error => { message = error; }), false);
    assert.equal(data.row.archived, true);
    assert.equal(data.api[`getArchived${data.suffix}`]()[0].id, 42);
    assert.match(message, /Could not restore.*Write rejected/);
    assert.equal(data.commands.length, 1, 'a rejected write must not refresh or erase cached Archive rows');
  });

  test(`${kind}: failed tier or calendar refresh distinguishes a committed restore from a rejected write`, async () => {
    for (const failure of [{ failTiers: true }, { failCalendar: true, failFallback: true }]) {
      const data = fixture(kind);
      await data.api[kind === 'shows' ? 'loadEpisodesForRange' : 'loadMoviesForRange']('2026-10-01', '2026-10-31');
      data.fail(failure);
      let message;
      assert.equal(await data.api[`unarchive${kind === 'shows' ? 'Show' : 'Movie'}`](42, error => { message = error; }), false);
      assert.equal(data.row.archived, false, 'the write stays committed even if a subsequent refresh fails');
      assert.match(message, /was restored, but some views could not refresh/);
    }
  });
}

test('show calendar fallback success remains a successful refresh; failed fallback returns false', async () => {
  const data = fixture('shows');
  data.fail({ failCalendar: true });
  assert.equal(await data.api.loadEpisodesForRange('2026-10-01', '2026-10-31'), true);
  data.fail({ failCalendar: true, failFallback: true });
  assert.equal(await data.api.refreshCalendar(), false);
});

test('Archive Reload repairs the persistent movie library after a committed restore refresh fails', async () => {
  const data = fixture('movies');
  data.row.tier_only = false;
  data.fail({ failTracked: true });
  assert.equal(await data.api.unarchiveMovie(42), false);
  assert.equal(data.row.archived, false);
  assert.equal(data.api.getTrackedMovies().length, 0, 'the failed refresh retained the previous movie cache');
  data.fail({});
  data.commands.length = 0;
  const source = readFileSync('src/lib/components/ArchiveManager.svelte', 'utf8').match(/<script lang="ts">([\s\S]*?)<\/script>/)[1];
  const api = load(source + '\nexport const harness = { reload, error:()=>loadError };', {
    'svelte': { onMount: noop, onDestroy: noop }, 'lucide-svelte': {},
    '../stores/shows.svelte': { getArchivedShows: () => [], isArchivedShowsLoading: () => false,
      loadArchivedShows: async () => true, loadTrackedShows: async () => true, refreshCalendar: async () => true },
    '../stores/movies.svelte': data.api,
    '../stores/showDetail.svelte': {}, '../stores/theme.svelte': { getThemeSettings: () => ({}) },
    '../stores/tiers.svelte': { loadTierListShows: async () => true, loadTierListMovies: async () => true },
    '../utils/tierSearch': {}, './common/EmptyState.svelte': {}, './common/LoadingSpinner.svelte': {},
  }, { $derived: value => value });
  await api.harness.reload();
  assert.equal(api.harness.error(), null);
  assert.equal(data.api.getTrackedMovies()[0].id, 42, 'Reload repairs the same cache used by persistent navigation');
  assert.equal(data.commands.filter(command => command === 'get_tracked_movies').length, 1, 'the movie refresh helper handles the tracked library without a duplicate read');
});

test('Archive blocks overlapping restores and exposes the store failure message inline', async () => {
  let finish;
  const calls = [];
  const source = readFileSync('src/lib/components/ArchiveManager.svelte', 'utf8').match(/<script lang="ts">([\s\S]*?)<\/script>/)[1];
  // The VM harness uses plain state values; start after initial loading to isolate the row guard.
  const readySource = source.replace('let reloading = $state(true);', 'let reloading = $state(false);');
  const ready = load(readySource + '\nexport const harness = { restore, snapshot:()=>({restoring,restoreMessage,restoreFailed}) };', {
    'svelte': { onMount: noop, onDestroy: noop }, 'lucide-svelte': {},
    '../stores/shows.svelte': { getArchivedShows: () => [], isArchivedShowsLoading: () => false, unarchiveShow: (id, onFailure) => {
      calls.push(id); return new Promise(resolve => { finish = () => { onFailure('The show was restored, but some views could not refresh.'); resolve(false); }; });
    } },
    '../stores/movies.svelte': { getArchivedMovies: () => [], isArchivedMoviesLoading: () => false, unarchiveMovie: async id => { calls.push(id); return true; } },
    '../stores/showDetail.svelte': {}, '../stores/theme.svelte': { getThemeSettings: () => ({}) }, '../stores/tiers.svelte': {},
    '../utils/tierSearch': {}, './common/EmptyState.svelte': {}, './common/LoadingSpinner.svelte': {},
  }, { $derived: value => value });
  const pending = ready.harness.restore({ id: 1, name: 'Show', type: 'show' });
  await ready.harness.restore({ id: 2, name: 'Movie', type: 'movie' });
  assert.deepEqual(calls, [1]);
  finish(); await pending;
  assert.equal(ready.harness.snapshot().restoring, null);
  assert.equal(ready.harness.snapshot().restoreFailed, true);
  assert.match(ready.harness.snapshot().restoreMessage, /was restored/);
});
