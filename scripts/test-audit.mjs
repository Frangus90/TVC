import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import vm from 'node:vm';
import { createRequire } from 'node:module';
import ts from 'typescript';

const require = createRequire(import.meta.url);
// Exercise the real store functions with controlled IPC completion order. State values
// are plain values here; browser smoke tests separately cover Svelte rendering/focus.
function load(path, dependencies = {}) {
  const code = ts.transpileModule(readFileSync(path, 'utf8'), { compilerOptions: {
    target: ts.ScriptTarget.ES2021, module: ts.ModuleKind.CommonJS,
  } }).outputText;
  const exports = {};
  vm.runInNewContext(code, { exports, $state: value => value, console, setTimeout, clearTimeout,
    require: name => name in dependencies ? dependencies[name] : require(name),
  }, { filename: path });
  return exports;
}
function store(name) {
  const requests = [], errors = [];
  const invoke = (command, args) => new Promise((resolve, reject) => requests.push({ command, args, resolve, reject }));
  const api = load(`src/lib/stores/${name}.svelte.ts`, {
    '@tauri-apps/api/core': { invoke },
    '../utils/logger': { logger: { error() {}, debug() {}, warn() {} } },
    '../utils/validation': { validateSearchQuery: query => ({ valid: query.trim().length > 0 }) },
    '../utils/requestDedup': { requestDeduplicator: { deduplicate: (_, request) => request() } },
    '../utils/database': { getDatabase: async () => ({ select: async () => [], execute: async () => ({}) }) },
    './toast.svelte': { showError: message => errors.push(message) },
    './shows.svelte': { loadTrackedShows: async () => {} },
    './movies.svelte': { loadTrackedMovies: async () => {} },
  });
  return { api, requests, errors };
}
const tick = async () => { for (let i = 0; i < 8; i++) await Promise.resolve(); };

for (const kind of ['shows', 'movies']) {
  test(`${kind}: newest search wins; clearing and closing invalidate pending results`, async () => {
    const { api, requests, errors } = store(kind);
    const search = api[kind === 'shows' ? 'searchShows' : 'searchMovies'];
    const get = api[kind === 'shows' ? 'getSearchResults' : 'getMovieSearchResults'];
    const loading = api[kind === 'shows' ? 'isSearchLoading' : 'isMovieSearchLoading'];
    const close = api[kind === 'shows' ? 'closeSearchModal' : 'closeMovieSearchModal'];
    const first = search('old'), second = search('new');
    requests[1].resolve([{ id: 2, name: 'new', title: 'new' }]); await second;
    requests[0].reject(new Error('old failure')); await first;
    assert.equal(get()[0].id, 2); assert.equal(errors.length, 0); assert.equal(loading(), false);
    const third = search('pending'); await search('');
    requests[2].resolve([{ id: 3 }]); await third;
    assert.equal(get().length, 0); assert.equal(loading(), false);
    const fourth = search('closing'); close(); requests[3].resolve([{ id: 4 }]); await fourth;
    assert.equal(get().length, 0);
  });
  test(`${kind}: an old calendar response cannot replace the current range`, async () => {
    const { api, requests } = store(kind);
    const fetch = api[kind === 'shows' ? 'loadEpisodesForRange' : 'loadMoviesForRange'];
    const get = api[kind === 'shows' ? 'getCalendarEpisodes' : 'getCalendarMovies'];
    const first = fetch('2026-09-01', '2026-09-30'), second = fetch('2026-10-01', '2026-10-31');
    requests[1].resolve([{ id: 2 }]); await second; requests[0].resolve([{ id: 1 }]); await first;
    assert.equal(get()[0].id, 2);
  });
  test(`${kind}: failed additions reject so the caller cannot show Added`, async () => {
    const { api, requests } = store(kind);
    const result = kind === 'shows' ? api.addShow({ tmdb_id: 1 }) : api.addMovie({ id: 1 });
    requests[0].reject(new Error('write failed'));
    await assert.rejects(result, /write failed/);
  });
}

test('movie details and cast ignore old selections and closed dialogs', async () => {
  const { api, requests } = store('movies');
  const first = api.openMovieDetail(1), second = api.openMovieDetail(2);
  requests[1].resolve({ id: 2 }); await second;
  requests[0].reject(new Error('old failure')); await first;
  assert.equal(api.getCurrentMovie().id, 2); assert.equal(api.getMovieDetailError(), null);
  const cast = api.fetchMovieCastCrew(2); api.closeMovieDetail();
  requests[2].resolve({ cast: [{ id: 2 }], crew: [] }); await cast;
  assert.equal(api.getMovieCast().length, 0);
  const detail = api.openMovieDetail(3); api.closeMovieDetail(); requests[3].resolve({ id: 3 }); await detail;
  assert.equal(api.getCurrentMovie(), null);
});

test('awards selection publishes only the selected ceremony and its score', async () => {
  const { api, requests } = store('awards');
  const first = api.selectCeremony(1), second = api.selectCeremony(2);
  requests[1].resolve({ id: 2, categories: [] }); await tick();
  requests[2].resolve({ picks: [{ category_id: 20, nominee_id: 200 }], correct: 1, total: 1 }); await second;
  requests[0].resolve({ id: 1, categories: [] }); await first;
  assert.equal(api.getSelectedCeremony().id, 2); assert.equal(api.getPrediction(20), 200);
  const third = api.selectCeremony(3); api.clearSelectedCeremony(); requests[3].resolve({ id: 3 }); await third;
  assert.equal(api.getSelectedCeremony(), null); assert.equal(api.getScore(), null);
});

test('Arr switching during import keeps the new server and its library', async () => {
  const { api, requests } = store('arr');
  const first = { id: 1, type: 'sonarr' }, second = { id: 2, type: 'radarr' };
  api.setSelectedServer(first); requests[0].resolve([{ arr_id: 11, tmdb_id: 1 }]); await tick();
  api.toggleItemSelection(11); const imported = api.importSelected();
  assert.equal(requests[1].args.request.server_id, 1);
  api.setSelectedServer(second); requests[2].resolve([{ arr_id: 22, tmdb_id: 2 }]); await tick();
  requests[1].resolve({ imported: 1 }); await imported;
  assert.equal(api.getSelectedServer().id, 2); assert.equal(api.getLibraryItems()[0].arr_id, 22);
  assert.equal(api.getImportResult(), null);
});

test('release note HTML is inert while limited formatting survives', () => {
  const { inlineReleaseNotes } = load('src/lib/utils/releaseNotes.ts');
  const html = inlineReleaseNotes('**Hello** `<img src=x onerror=alert(1)>` & <svg onload=alert(2)>');
  assert.match(html, /<strong[^>]*>Hello<\/strong>/); assert.match(html, /&lt;img/);
  assert.doesNotMatch(html, /<(?:img|svg)\b/); assert.match(html, /&amp;/);
});

test('Oslo calendar boundaries and grouping follow local days including DST', () => {
  const old = process.env.TZ; process.env.TZ = 'Europe/Oslo';
  try {
    const { localDayRange, formatDateKey, formatDateTime } = load('src/lib/utils/dateFormat.ts');
    const spring = localDayRange('2026-03-29', '2026-03-29');
    assert.equal((Date.parse(spring.end) - Date.parse(spring.start)) / 3600000, 23);
    const autumn = localDayRange('2026-10-25', '2026-10-25');
    assert.equal((Date.parse(autumn.end) - Date.parse(autumn.start)) / 3600000, 25);
    assert.equal(formatDateKey('2026-09-17T23:00:00Z'), '2026-09-18');
    assert.equal(formatDateTime('2026-09-17T23:00:00Z'), '18.09.2026 01:00');
  } finally { if (old === undefined) delete process.env.TZ; else process.env.TZ = old; }
});
