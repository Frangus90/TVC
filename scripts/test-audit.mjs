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
function store(name, dependencies = {}) {
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
    ...dependencies,
  });
  return { api, requests, errors };
}

for (const kind of ['shows', 'movies']) {
  test(`${kind}: library refresh failures retain cached titles and report failure to Data actions`, async () => {
    let fail = false;
    const titles = [{ id: 42, name: 'Saved show', title: 'Saved movie', archived: false, tier_only: false }];
    const { api } = store(kind, {
      '@tauri-apps/api/core': { invoke: async () => {
        if (fail) throw new Error('Read failed');
        return titles;
      } },
      '../utils/database': { getDatabase: async () => { throw new Error('Fallback unavailable'); } },
    });
    const suffix = kind === 'shows' ? 'Shows' : 'Movies';
    assert.equal(await api[`loadTracked${suffix}`](), true);
    assert.equal(await api[`loadArchived${suffix}`](), true);
    fail = true;
    assert.equal(await api[`loadTracked${suffix}`](), false);
    assert.equal(await api[`loadArchived${suffix}`](), false);
    assert.equal(api[`getTracked${suffix}`]()[0].id, 42);
    assert.equal(api[`getArchived${suffix}`]()[0].id, 42);
  });
}

test('Plex start failure stays visible and does not persist automatic startup', async () => {
  const { api, requests } = store('plex');
  const toggling = api.toggleServer(true);
  assert.equal(requests[0].command, 'start_plex_server');
  requests[0].reject(new Error('Port is already in use'));
  assert.equal(await toggling, false);
  assert.match(api.getError(), /Port is already in use/);
  assert.equal(api.getConfig().enabled, false);
  assert.equal(requests.length, 1, 'failed start must not erase the error through a config save');
});

test('Plex persists startup only after a successful server transition', async () => {
  const commands = [];
  const { api } = store('plex', { '@tauri-apps/api/core': { invoke: async (command, args) => {
    commands.push(command);
    if (command === 'get_plex_server_status') return { running: true, port: 9876 };
    if (command === 'update_plex_config') assert.equal(args.config.enabled, true);
  } } });
  assert.equal(await api.toggleServer(true), true);
  assert.equal(api.getServerStatus().running, true);
  assert.equal(api.getConfig().enabled, true);
  assert.deepEqual(commands, ['start_plex_server', 'get_plex_server_status', 'update_plex_config']);
});

test('Plex configuration failure retains the running status and exposes unsaved startup preference', async () => {
  const { api } = store('plex', { '@tauri-apps/api/core': { invoke: async command => {
    if (command === 'get_plex_server_status') return { running: true, port: 9876 };
    if (command === 'update_plex_config') throw new Error('Settings could not be saved');
  } } });
  assert.equal(await api.toggleServer(true), false);
  assert.equal(api.getServerStatus().running, true);
  assert.equal(api.getConfig().enabled, false);
  assert.match(api.getError(), /Settings could not be saved/);
});

test('Saved-server connection checks report failures and ignore results after server editing', async () => {
  let resolveCheck;
  const { api } = store('arr', { '@tauri-apps/api/core': { invoke: async command => {
    if (command === 'test_arr_server') return new Promise(resolve => { resolveCheck = resolve; });
    if (command === 'get_arr_servers') return [];
  } } });
  const server = { id: 1, name: 'Home', type: 'sonarr', base_url: 'http://example.invalid', api_key: 'fixture' };
  const pending = api.checkSavedServer(server);
  assert.equal(api.getServerConnectionCheck(1).state, 'checking');
  await api.updateServer(1, { ...server, base_url: 'http://new.example.invalid' });
  resolveCheck({ version: '4.0' });
  await pending;
  assert.equal(api.getServerConnectionCheck(1), undefined, 'old connection result must not describe the newly edited server');
  const failed = store('arr', { '@tauri-apps/api/core': { invoke: async () => { throw new Error('Connection refused'); } } });
  await failed.api.checkSavedServer(server);
  assert.equal(failed.api.getServerConnectionCheck(1).state, 'failed');
  assert.match(failed.api.getServerConnectionCheck(1).message, /Connection refused/);
});

for (const kind of ['shows', 'movies']) {
  test(`${kind}: removing a tier-only title clears the shared library cache; failed removal keeps it`, async () => {
    const title = kind === 'shows' ? { name: 'Tier-only show' } : { title: 'Tier-only movie' };
    let saved = [{ id: 42, tier_only: true, tier_id: 1, ...title }];
    let failRemoval = false;
    const invoke = async (command) => {
      if (command === `get_tier_list_${kind}`) return structuredClone(saved);
      if (command === `get_tracked_${kind}`) return [];
      if (command === (kind === 'shows' ? 'remove_show' : 'remove_movie')) {
        if (failRemoval) throw new Error('write failed');
        saved = [];
      }
    };
    const tiers = load('src/lib/stores/tiers.svelte.ts', {
      '@tauri-apps/api/core': { invoke },
      '../utils/logger': { logger: { error() {} } },
      './toast.svelte': { showError() {} },
    });
    const refreshTiers = tiers[kind === 'shows' ? 'loadTierListShows' : 'loadTierListMovies'];
    const getTiers = tiers[kind === 'shows' ? 'getTierListShows' : 'getTierListMovies'];
    const { api, errors } = store(kind, { '@tauri-apps/api/core': { invoke }, './tiers.svelte': tiers });
    const remove = api[kind === 'shows' ? 'removeShow' : 'removeMovie'];
    await refreshTiers();
    assert.equal(getTiers()[0].id, 42);
    await remove(42);
    assert.equal(saved.length, 0);
    assert.equal(getTiers().length, 0, 'combined library/global search must not retain a deleted title');
    saved = [{ id: 42, tier_only: true, tier_id: 1, ...title }];
    await refreshTiers();
    failRemoval = true;
    await remove(42);
    assert.equal(getTiers()[0].id, 42);
    assert.equal(errors.length, 1);
  });
}

test('unmapped show remap, deletion and completed migration refresh every shared show-ID cache', async () => {
  let saved = [{ id: 42, name: 'Legacy show', tier_only: true, tier_id: 1, archived: true }];
  let archived = [], tracked = [];
  const listeners = new Map();
  const invoke = async (command, args) => {
    if (command === 'get_tier_list_shows') return structuredClone(saved);
    if (command === 'get_unmigrated_shows') return [];
    if (command === 'resolve_unmigrated_show') {
      saved = saved.map(show => ({ ...show, id: args.newTmdbId }));
      return { episodes_orphaned: 0, merged_with: null };
    }
    if (command === 'delete_unmigrated_show') saved = [];
  };
  const dependencies = {
    '@tauri-apps/api/core': { invoke },
    '../utils/logger': { logger: { error() {} } },
    './toast.svelte': { showError() {} },
  };
  const tiers = load('src/lib/stores/tiers.svelte.ts', dependencies);
  const migration = load('src/lib/stores/migration.svelte.ts', {
    ...dependencies,
    '@tauri-apps/api/event': { listen: async (name, handler) => { listeners.set(name, handler); return () => listeners.delete(name); } },
    './tiers.svelte': tiers,
    './shows.svelte': {
      loadTrackedShows: async () => { tracked = saved.filter(show => !show.archived); },
      loadArchivedShows: async () => { archived = saved.filter(show => show.archived); },
      refreshCalendar: async () => {},
    },
  });
  await tiers.loadTierListShows();
  await migration.resolveUnmigratedShow(42, 99);
  assert.equal(tiers.getTierListShows()[0].id, 99);
  assert.equal(archived[0].id, 99, 'archived-ID exclusion must use the remapped ID');
  assert.equal(tracked.length, 0);
  await migration.deleteUnmigratedShow(99);
  assert.equal(tiers.getTierListShows().length, 0);
  assert.equal(archived.length, 0);
  saved = [{ id: 101, name: 'Migrated show', tier_only: true, tier_id: 1, archived: false }];
  const stop = migration.setupMigrationListener();
  listeners.get('tvdb_migration_finished')({ payload: { mapped: 1, quarantined: 0, errors: [], per_show: [] } });
  await tick();
  assert.equal(tiers.getTierListShows()[0].id, 101);
  assert.equal(tracked[0].id, 101);
  stop();
});
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
