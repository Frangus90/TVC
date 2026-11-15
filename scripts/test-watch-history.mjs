import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import vm from 'node:vm';
import ts from 'typescript';

function loadStore(invoke, showError = () => {}) {
  const exports = {};
  const code = ts.transpileModule(readFileSync('src/lib/stores/statistics.svelte.ts', 'utf8'), {
    compilerOptions: { target: ts.ScriptTarget.ES2021, module: ts.ModuleKind.CommonJS },
  }).outputText;
  vm.runInNewContext(code, {
    exports,
    $state: value => value,
    require: name => {
      if (name === '@tauri-apps/api/core') return { invoke };
      if (name === '../utils/logger') return { logger: { error() {} } };
      if (name === './toast.svelte') return { showError };
      if (name === '../utils/dateFormat') return {
        formatDateKey: value => value,
        formatDateHeader: value => value,
        parseDateTime: value => new Date(value),
      };
      throw new Error(`Unexpected dependency: ${name}`);
    },
  }, { filename: 'statistics.svelte.ts' });
  return exports;
}

const item = (name, item_type = 'movie') => ({
  source: 'plex', item_type, id: 1, detail_id: null, name,
  show_name: item_type === 'episode' ? 'Untracked Show' : null,
  season_number: item_type === 'episode' ? 1 : null,
  episode_number: item_type === 'episode' ? 2 : null,
  year: null, watched_at: '2026-11-14T20:00:00Z', poster_url: null, runtime: null,
});

test('Watch History loads the Plex and TVC results and keeps the selected media tab', async () => {
  const history = [item('Untracked Movie'), item('Untracked Episode', 'episode')];
  const calls = [];
  const api = loadStore(async (command, args) => {
    calls.push([command, args]);
    return history;
  });

  await api.loadWatchHistory();

  assert.equal(calls.length, 1);
  assert.equal(calls[0][0], 'get_watch_history');
  assert.equal(calls[0][1].limit, undefined);
  assert.deepEqual(Array.from(api.getWatchHistory(), entry => entry.name), ['Untracked Movie', 'Untracked Episode']);
  assert.equal(api.getHistoryMediaTab(), 'tv');
  api.setHistoryMediaTab('movies');
  assert.equal(api.getHistoryMediaTab(), 'movies');
});

test('a late older history response cannot overwrite a newer Plex refresh', async () => {
  const pending = [];
  const api = loadStore(() => new Promise(resolve => pending.push(resolve)));
  const older = api.loadWatchHistory();
  const newer = api.loadWatchHistory();

  pending[1]([item('Newest Plex play')]);
  await newer;
  pending[0]([item('Stale result')]);
  await older;

  assert.deepEqual(Array.from(api.getWatchHistory(), entry => entry.name), ['Newest Plex play']);
});

test('matching a Plex history item saves the selected title and reloads history', async () => {
  const history = [item('Matched title')];
  const calls = [];
  const api = loadStore(async (command, args) => {
    calls.push([command, args]);
    return command === 'get_watch_history' ? history : undefined;
  });

  assert.equal(await api.matchPlexHistoryItem(17, 'episode', 42), true);
  assert.deepEqual(JSON.parse(JSON.stringify(calls)), [
    ['resolve_plex_history_item', { historyId: 17, mediaType: 'episode', entityId: 42 }],
    ['get_watch_history', {}],
  ]);
  assert.equal(api.getWatchHistory()[0].name, 'Matched title');
});

test('a failed Plex history match reports the error and leaves the picker result usable', async () => {
  const errors = [];
  const api = loadStore(async () => { throw new Error('database unavailable'); }, message => errors.push(message));

  assert.equal(await api.matchPlexHistoryItem(17, 'movie', 42), false);
  assert.equal(errors.length, 1);
  assert.match(errors[0], /could not be linked/);
});
