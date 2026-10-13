import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';
import { test } from 'node:test';

function load(source, dependencies, globals = {}) {
  const exports = {};
  const code = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2021, module: ts.ModuleKind.CommonJS } }).outputText;
  vm.runInNewContext(code, { exports, require: name => {
    if (!(name in dependencies)) throw new Error(`Unexpected dependency: ${name}`);
    return dependencies[name];
  }, $state: value => value, $derived: value => value, console, ...globals });
  return exports;
}
const tierSource = readFileSync('src/lib/stores/tiers.svelte.ts', 'utf8');
const noop = () => {};
const tick = async () => { for (let index = 0; index < 30; index++) await Promise.resolve(); };

test('tier drafts stay in session memory without changing saved tiers or calling IPC', async () => {
  const commands = [];
  const api = load(tierSource, {
    '@tauri-apps/api/core': { invoke: async command => { commands.push(command); return [{ id: 1, name: 'Saved tier', color: '', position: 1 }]; } },
    '../utils/logger': { logger: { error: noop } }, './toast.svelte': { showError: noop },
  });
  await api.loadTiers();
  commands.length = 0;
  const edits = [{ name: 'Draft tier', color: '#8b5cf6' }];
  api.saveTierDraft('custom', edits);
  edits[0].name = 'Mutated caller';
  // A newly mounted editor reads the same module's session draft.
  const restored = api.getTierDraft();
  assert.equal(restored.tiers[0].name, 'Draft tier');
  assert.equal(api.getTiers()[0].name, 'Saved tier');
  assert.equal(commands.length, 0);
  api.clearTierDraft();
  assert.equal(api.getTierDraft(), null);
  assert.equal(commands.length, 0);
});

test('tier Apply failure keeps the draft; successful Apply refreshes both libraries and open details', async () => {
  const calls = [];
  let fail = true;
  let show = { id: 11, tier_id: 1 }, movie = { id: 21, tier_id: 1 };
  const reload = name => async () => { calls.push(name); };
  const api = load(tierSource, {
    '@tauri-apps/api/core': { invoke: async (command, args) => {
      calls.push(command);
      if (command === 'apply_tier_configuration') {
        assert.equal(args.preset, 'custom');
        if (fail) throw new Error('transaction rejected');
        return { tiers: [{ id: 2, name: 'New tier', color: '', position: 1 }], cleared_shows: 1, cleared_movies: 1 };
      }
      return [];
    } },
    '../utils/logger': { logger: { error: noop } }, './toast.svelte': { showError: noop },
    './shows.svelte': { loadTrackedShows: reload('tracked shows'), loadArchivedShows: reload('archived shows') },
    './movies.svelte': { loadTrackedMovies: reload('tracked movies'), loadArchivedMovies: reload('archived movies'), isMovieDetailOpen: () => true, getCurrentMovie: () => movie, openMovieDetail: async id => { calls.push('movie detail'); assert.equal(id, 21); movie = { id, tier_id: null }; } },
    './showDetail.svelte': { isShowDetailOpen: () => true, getCurrentShow: () => show, openShowDetail: async id => { calls.push('show detail'); assert.equal(id, 11); show = { id, tier_id: null }; } },
  });
  api.saveTierDraft('custom', [{ name: 'New tier', color: '' }]);
  await assert.rejects(api.applyTierConfiguration('custom', api.getTierDraft().tiers), /transaction rejected/);
  assert.equal(api.getTierDraft().tiers[0].name, 'New tier');
  assert.equal(calls.length, 1);
  calls.length = 0;
  fail = false;
  await api.applyTierConfiguration('custom', api.getTierDraft().tiers);
  assert.equal(calls.filter(name => name === 'apply_tier_configuration').length, 1);
  for (const name of ['get_tier_list_shows', 'get_tier_list_movies', 'tracked shows', 'archived shows', 'tracked movies', 'archived movies', 'show detail', 'movie detail']) assert.ok(calls.includes(name), name);
  assert.equal(show.tier_id, null);
  assert.equal(movie.tier_id, null);
  assert.equal(api.getTierDraft(), null);
});

test('Tiermaker matching and cancelled import never create tiers; confirmed import maps preview IDs to saved tiers', async () => {
  const mutations = [];
  let confirmed = false;
  let saved = [{ id: 1, name: 'Existing other-list tier', color: '', position: 1 }];
  const destroys = [];
  const component = readFileSync('src/lib/components/settings/TiermakerImport.svelte', 'utf8').match(/<script lang="ts">([\s\S]*?)<\/script>/)[1];
  const source = component + '\nexport const harness = { parse: parseHtml, importTitles: startImport, setInput: (value: string) => htmlInput = value, snapshot: () => ({step,isSearching,matchedItems,plannedTiers}) };';
  const api = load(source, {
    'svelte': { onDestroy: callback => destroys.push(callback) },
    '@tauri-apps/api/core': { invoke: async command => {
      assert.equal(command, 'search_movies');
      return [{ id: 101, title: 'Film One', release_date: '2001-01-01', poster_url: null }];
    } },
    '../../stores/confirmDialog.svelte': { openConfirmDialog: async () => confirmed },
    '../../stores/tiers.svelte': {
      getTiers: () => saved,
      createTier: async (name, color) => { mutations.push('create tier'); const tier = { id: 2, name, color, position: 0 }; saved = [...saved, tier]; return tier; },
      reorderTiers: async ids => { mutations.push('reorder tiers'); assert.equal(ids.join(','), '2,1'); },
      addMovieTierOnly: async (id, tierId) => { mutations.push('import movie'); assert.equal(id, 101); assert.equal(tierId, 2); },
      addShowTierOnly: () => assert.fail('movie preview must not import TV show IDs'),
      addManualMovie: noop, addManualShow: noop, loadTierListMovies: async () => {}, loadTierListShows: async () => {}, loadTiers: async () => {},
    },
    'lucide-svelte': {},
  }, { DOMParser: class {
    parseFromString() { return { querySelectorAll: selector => selector === '.tier-row' ? [{
      querySelector: () => ({ textContent: 'Imported tier' }),
      querySelectorAll: () => [{ querySelector: () => ({ getAttribute: name => name === 'alt' ? 'Film One' : '' }) }],
    }] : [] }; }
  } });
  api.harness.setInput('controlled Tiermaker fixture');
  api.harness.parse();
  await tick();
  assert.equal(api.harness.snapshot().step, 'review');
  assert.equal(api.harness.snapshot().plannedTiers.length, 1);
  assert.equal(mutations.length, 0);
  assert.equal(saved.length, 1);
  await api.harness.importTitles();
  assert.equal(api.harness.snapshot().step, 'review');
  assert.equal(mutations.length, 0);
  confirmed = true;
  await api.harness.importTitles();
  assert.equal(api.harness.snapshot().step, 'done');
  assert.equal(mutations.join(','), 'create tier,reorder tiers,import movie');
  assert.equal(saved[0].name, 'Existing other-list tier');
  destroys.forEach(destroy => destroy());
});

test('Racing settings includes returned categories and reports central reminder blocking', async () => {
  const rows = [
    { slug: 'dtm', category: 'touring', enabled: true },
    { slug: 'extreme-e', category: 'other', enabled: false },
    { slug: 'future-series', category: 'new-category', enabled: true },
  ];
  let refreshed = 0;
  const component = readFileSync('src/lib/components/settings/RacingTab.svelte', 'utf8').match(/<script lang="ts">([\s\S]*?)<\/script>/)[1].replace('import.meta.env.DEV', 'false');
  const source = component + '\nexport const harness = { grouped: getSeriesByCategory, setFilter: (value: "all" | "followed") => seriesFilter = value, status: () => reminderStatus, toggle: handleToggle };';
  const derived = value => value;
  derived.by = callback => callback();
  const api = load(source, {
    'lucide-svelte': {},
    '../../stores/racing.svelte': { getRacingSeries: () => rows, getRacingConfig: () => ({ notifications_enabled: true }), toggleSeries: async () => false, refreshSingleSeries: async () => { refreshed++; } },
    '../../stores/notifications.svelte': { getNotificationSettings: () => ({ enabled: false, racing_enabled: true }) },
    '../../stores/settings.svelte': {}, '../../utils/dateFormat': {},
  }, { $derived: derived, $effect: noop, });
  api.harness.setFilter('all');
  assert.equal(api.harness.grouped().touring[0].slug, 'dtm');
  assert.equal(api.harness.grouped().other[0].slug, 'extreme-e');
  assert.equal(api.harness.grouped()['new-category'][0].slug, 'future-series');
  api.harness.setFilter('followed');
  assert.equal(api.harness.grouped().other.length, 0);
  assert.match(api.harness.status(), /paused in Notifications/);
  await api.harness.toggle('dtm', true);
  assert.equal(refreshed, 0, 'failed following changes must not trigger a feed refresh');
});
