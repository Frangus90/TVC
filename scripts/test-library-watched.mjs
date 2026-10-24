import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import vm from 'node:vm';
import ts from 'typescript';

// Real stores and handlers run against isolated memory records, never a native database.
function load(source, dependencies, globals = {}) {
  const exports = {};
  const code = ts.transpileModule(source, { compilerOptions: {
    target: ts.ScriptTarget.ES2021, module: ts.ModuleKind.CommonJS,
  } }).outputText;
  vm.runInNewContext(code, {
    exports, $state: value => value, require: name => {
      if (!(name in dependencies)) throw new Error(`Unexpected dependency: ${name}`);
      return dependencies[name];
    }, ...globals,
  });
  return exports;
}
const source = name => readFileSync(`src/lib/stores/${name}.svelte.ts`, 'utf8');
const copy = value => JSON.parse(JSON.stringify(value));
const noop = () => {};
const deferred = () => {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
};

function memoryLibrary() {
  const errors = [], commands = [];
  const movies = [
    { id: 1, title: 'Tracked movie', watched: false, archived: false, tier_only: false, tier_id: 1, rank_order: 4, scheduled_date: '2026-10-24' },
    { id: 2, title: 'Other movie', watched: false, archived: false, tier_only: false, tier_id: 2, rank_order: 3 },
    { id: 3, title: 'Archived movie', watched: false, archived: true, tier_only: false, tier_id: 1, rank_order: 2 },
    { id: 4, title: 'Tier-only movie', watched: false, archived: false, tier_only: true, tier_id: 1, rank_order: 1 },
  ];
  const shows = [
    { id: 11, name: 'First show', archived: false, tier_only: false, tier_id: 1, rank_order: 2 },
    { id: 12, name: 'Second show', archived: false, tier_only: true, tier_id: 2, rank_order: 1 },
  ];
  const episodes = [
    { id: 101, show_id: 11, season_number: 1, watched: false },
    { id: 102, show_id: 11, season_number: 2, watched: false },
    { id: 103, show_id: 12, season_number: 1, watched: false },
  ];
  let beforeWrite = async () => {};
  const showRows = () => shows.map(show => ({ ...show,
    episode_count: episodes.filter(episode => episode.show_id === show.id).length,
    watched_episodes: episodes.filter(episode => episode.show_id === show.id && episode.watched).length,
  }));
  const invoke = async (command, args) => {
    commands.push({ command, args });
    switch (command) {
      case 'get_tracked_movies': return copy(movies.filter(movie => !movie.archived && !movie.tier_only));
      case 'get_archived_movies': return copy(movies.filter(movie => movie.archived));
      case 'get_tier_list_movies': return copy(movies);
      case 'get_movies_for_range': return copy(movies.filter(movie => movie.scheduled_date));
      case 'get_movie_details': return copy(movies.find(movie => movie.id === args.id));
      case 'get_tracked_shows': return copy(showRows().filter(show => !show.archived && !show.tier_only));
      case 'get_archived_shows': return [];
      case 'get_tier_list_shows': return copy(showRows());
      case 'get_episodes_for_range': return copy(episodes);
      case 'sync_show_episodes':
        episodes.push({ id: 104, show_id: args.showId, season_number: 2, watched: false });
        return;
      case 'mark_movie_watched':
        await beforeWrite();
        movies.find(movie => movie.id === args.id).watched = args.watched;
        return;
      case 'mark_show_watched':
        await beforeWrite();
        for (const episode of episodes) if (episode.show_id === args.showId) episode.watched = args.watched;
        return;
      default: throw new Error(`Unexpected IPC: ${command}`);
    }
  };
  const dependencies = {
    '@tauri-apps/api/core': { invoke },
    '../utils/logger': { logger: { error: noop, debug: noop } },
    '../utils/validation': {}, '../utils/requestDedup': {},
    './toast.svelte': { showError: message => errors.push(message) },
    '../utils/database': { getDatabase: async () => ({ select: async (query, [id]) => {
      if (/FROM shows WHERE/.test(query)) return copy(shows.filter(show => show.id === id).map(show => ({ ...show, tier_only: Number(show.tier_only), unmigrated: 0 })));
      if (/FROM episodes/.test(query)) return copy(episodes.filter(episode => episode.show_id === id).map(episode => ({ ...episode, watched: Number(episode.watched) })));
      throw new Error(`Unexpected memory query: ${query}`);
    } }) },
  };
  const tiers = load(source('tiers'), dependencies);
  dependencies['./tiers.svelte'] = tiers;
  const movieStore = load(source('movies'), dependencies);
  const showStore = load(source('shows'), dependencies);
  dependencies['./shows.svelte'] = showStore;
  const detail = load(source('showDetail'), dependencies);
  return { movies: movieStore, shows: showStore, detail, tiers, errors, commands,
    beforeWrite: callback => { beforeWrite = callback; },
  };
}

async function loadMovies(data) {
  await Promise.all([data.movies.loadTrackedMovies(), data.movies.loadArchivedMovies(), data.tiers.loadTierListMovies(), data.movies.loadMoviesForRange('2026-10-01', '2026-10-31')]);
}

test('movie watched actions update every matching view, including tier-only titles, while preserving ranks and schedules', async () => {
  const data = memoryLibrary();
  await loadMovies(data);
  const before = copy(data.tiers.getTierListMovies());
  for (const id of [1, 3, 4]) {
    await data.movies.openMovieDetail(id);
    assert.equal(await data.movies.markMovieWatched(id, true), true);
    assert.equal(data.movies.getCurrentMovie().watched, true);
    const title = data.tiers.getTierListMovies().find(movie => movie.id === id);
    assert.deepEqual(copy(title), { ...before.find(movie => movie.id === id), watched: true });
    for (const rows of [data.movies.getTrackedMovies(), data.movies.getArchivedMovies(), data.movies.getCalendarMovies()]) {
      const item = rows.find(movie => movie.id === id);
      if (item) assert.equal(item.watched, true);
    }
  }
  assert.deepEqual(copy(data.tiers.getTierListMovies().find(movie => movie.id === 2)), before.find(movie => movie.id === 2));
  assert.equal(await data.movies.markMovieWatched(1, false), true);
  assert.equal(data.movies.getTrackedMovies().find(movie => movie.id === 1).watched, false);
  assert.equal(data.movies.getCalendarMovies()[0].watched, false);
  assert.equal(data.tiers.getTierListMovies().find(movie => movie.id === 1).watched, false);
  assert.equal(data.movies.getCurrentMovie().id, 4, 'updating another title must not navigate the open modal');
  assert.equal(data.movies.getCurrentMovie().watched, true);
  assert.equal(data.errors.length, 0);
});

test('a rejected movie watched write retains all cached watched states and reports failure', async () => {
  const data = memoryLibrary();
  await loadMovies(data);
  await data.movies.openMovieDetail(1);
  const before = copy([data.movies.getTrackedMovies(), data.movies.getArchivedMovies(), data.movies.getCalendarMovies(), data.movies.getCurrentMovie(), data.tiers.getTierListMovies()]);
  data.beforeWrite(async () => { throw new Error('Write rejected'); });
  assert.equal(await data.movies.markMovieWatched(1, true), false);
  assert.deepEqual(copy([data.movies.getTrackedMovies(), data.movies.getArchivedMovies(), data.movies.getCalendarMovies(), data.movies.getCurrentMovie(), data.tiers.getTierListMovies()]), before);
  assert.match(data.errors[0], /Write rejected/);
});

test('marking a whole show updates all its seasons, calendar, picker and library counts without changing tier assignments', async () => {
  const data = memoryLibrary();
  await Promise.all([data.shows.loadTrackedShows(), data.tiers.loadTierListShows(), data.shows.loadEpisodesForRange('2026-10-01', '2026-10-31')]);
  await data.detail.openShowDetail(11);
  await data.shows.openEpisodePicker({ id: 11 }, '2026-10-24');
  const ranks = copy(data.tiers.getTierListShows()).map(({ id, tier_id, rank_order }) => ({ id, tier_id, rank_order }));
  assert.equal(await data.detail.markShowWatched(11, true), true);
  assert.equal(data.detail.getShowEpisodes().filter(episode => episode.watched).length, 2);
  assert.equal(data.shows.getEpisodePickerEpisodes().filter(episode => episode.watched).length, 2);
  assert.equal(data.shows.getCalendarEpisodes().find(episode => episode.show_id === 12).watched, false);
  assert.equal(data.shows.getTrackedShows()[0].watched_episodes, 2);
  assert.equal(data.tiers.getTierListShows()[0].watched_episodes, 2);
  assert.deepEqual(copy(data.tiers.getTierListShows()).map(({ id, tier_id, rank_order }) => ({ id, tier_id, rank_order })), ranks);
  assert.equal(data.errors.length, 0);
});

test('refreshing a fully watched show reloads library summaries when TMDB adds an unwatched episode', async () => {
  const data = memoryLibrary();
  await data.shows.loadEpisodesForRange('2026-10-01', '2026-10-31');
  await data.detail.markShowWatched(11, true);
  assert.equal(data.shows.getTrackedShows()[0].watched_episodes, 2);
  assert.equal(data.shows.getTrackedShows()[0].episode_count, 2);

  await data.shows.syncShowEpisodes(11);

  for (const rows of [data.shows.getTrackedShows(), data.tiers.getTierListShows()]) {
    const show = rows.find(show => show.id === 11);
    assert.equal(show.episode_count, 3);
    assert.equal(show.watched_episodes, 2);
    assert.equal(show.tier_id, 1);
    assert.equal(show.rank_order, 2);
  }
  const calendar = data.shows.getCalendarEpisodes().filter(episode => episode.show_id === 11);
  assert.equal(calendar.filter(episode => episode.watched).length, 2);
  assert.equal(calendar.find(episode => episode.id === 104).watched, false);
  assert.equal(data.errors.length, 0);
});

test('an old show watched completion leaves another open show and picker untouched', async () => {
  const data = memoryLibrary(), pending = deferred();
  await data.shows.loadEpisodesForRange('2026-10-01', '2026-10-31');
  await data.detail.openShowDetail(11);
  data.beforeWrite(() => pending.promise);
  const marking = data.detail.markShowWatched(11, true);
  await data.detail.openShowDetail(12);
  await data.shows.openEpisodePicker({ id: 12 }, '2026-10-24');
  pending.resolve();
  assert.equal(await marking, true);
  assert.equal(data.detail.getCurrentShow().id, 12);
  assert.equal(data.detail.getShowEpisodes()[0].watched, false);
  assert.equal(data.shows.getEpisodePickerEpisodes()[0].watched, false);
  assert.equal(data.shows.getCalendarEpisodes().filter(episode => episode.show_id === 11).every(episode => episode.watched), true);
  assert.equal(data.shows.getCalendarEpisodes().find(episode => episode.show_id === 12).watched, false);
});

test('a rejected show watched write leaves episodes unchanged and does not assign the error to a newer detail', async () => {
  const data = memoryLibrary(), pending = deferred();
  await data.shows.loadEpisodesForRange('2026-10-01', '2026-10-31');
  await data.detail.openShowDetail(11);
  await data.shows.openEpisodePicker({ id: 11 }, '2026-10-24');
  data.beforeWrite(() => pending.promise);
  const marking = data.detail.markShowWatched(11, true);
  await data.detail.openShowDetail(12);
  pending.reject(new Error('Write rejected'));
  assert.equal(await marking, false);
  assert.equal(data.detail.getError(), null);
  assert.equal(data.detail.getShowEpisodes()[0].watched, false);
  assert.equal(data.shows.getCalendarEpisodes().every(episode => !episode.watched), true);
  assert.equal(data.shows.getEpisodePickerEpisodes().every(episode => !episode.watched), true);
  assert.match(data.errors[0], /Write rejected/);
});

function libraryHandler(kind, mark, records = {}) {
  const script = readFileSync('src/lib/components/LibraryView.svelte', 'utf8').match(/<script[^>]*>([\s\S]*?)<\/script>/)[1];
  const successes = [], opened = [];
  const derived = value => value;
  derived.by = callback => callback();
  const api = load(script + '\nexport const harness = { markWatched, items, switchMedia: value => mediaType = value, snapshot: () => ({ busy, marking, selected: [...selected], bulkMode }) };', {
    'svelte': {},
    '../stores/shows.svelte': { getTrackedShows: () => records.trackedShows ?? [], getArchivedShows: () => [] },
    '../stores/movies.svelte': { getTrackedMovies: () => records.trackedMovies ?? [], getArchivedMovies: () => [], markMovieWatched: (id, watched) => mark('movies', id, watched), openMovieDetail: id => opened.push(id) },
    '../stores/showDetail.svelte': { markShowWatched: (id, watched) => mark('shows', id, watched), openShowDetail: id => opened.push(id) },
    '../stores/tiers.svelte': { getTierListShows: () => records.tierShows ?? [], getTierListMovies: () => records.tierMovies ?? [] },
    '../stores/theme.svelte': { getThemeSettings: () => ({}) },
    '../stores/toast.svelte': { showSuccess: message => successes.push(message) },
    '../stores/sidebar.svelte': {}, '../stores/confirmDialog.svelte': {}, '../utils/dateFormat': {},
  }, { $props: () => ({ mediaType: kind }), $derived: derived, $effect: noop });
  return { ...api.harness, successes, opened };
}

for (const kind of ['shows', 'movies']) {
  test(`${kind}: library eye toggles watched titles off and unwatched or partially watched titles on`, async () => {
    const calls = [];
    const records = kind === 'shows' ? {
      trackedShows: [
        { id: 41, name: 'Fully watched show', episode_count: 2, watched_episodes: 2 },
        { id: 42, name: 'Partially watched show', episode_count: 2, watched_episodes: 1 },
      ],
      tierShows: [{ id: 43, name: 'Watched tier-only show', tier_only: true, episode_count: 1, watched_episodes: 1 }],
    } : {
      trackedMovies: [
        { id: 41, title: 'Watched movie', watched: true },
        { id: 42, title: 'Unwatched movie', watched: false },
      ],
      tierMovies: [{ id: 43, title: 'Watched tier-only movie', tier_only: true, watched: true }],
    };
    const api = libraryHandler(kind, async (...args) => { calls.push(args); return true; }, records);
    for (const id of [41, 42, 43]) await api.markWatched(api.items.find(item => item.id === id));
    assert.deepEqual(calls, [[kind, 41, false], [kind, 42, true], [kind, 43, false]]);
    assert.equal(api.successes.length, 3);
    assert.match(api.successes[0], /marked as unwatched/);
    assert.match(api.successes[1], /marked as watched/);
    assert.match(api.successes[2], /marked as unwatched/);
    assert.equal(api.opened.length, 0, 'toggling the eye must not open a detail dialog');
  });

  test(`${kind}: library watched buttons report only saved changes and retain the original media during an overlapping click`, async () => {
    const calls = [], pending = deferred();
    let result = false;
    const api = libraryHandler(kind, async (...args) => {
      calls.push(args);
      return result ? pending.promise : false;
    });
    const item = { id: 42, title: 'Saved title', watched: true };
    await api.markWatched(item);
    assert.deepEqual(calls[0], [kind, 42, false]);
    assert.equal(api.successes.length, 0, 'a rejected or empty-show write must not claim success');
    assert.equal(api.snapshot().busy, false);
    result = true;
    const marking = api.markWatched(item);
    assert.equal(api.snapshot().busy, true);
    assert.equal(api.snapshot().marking, `${kind}:42`);
    api.switchMedia(kind === 'shows' ? 'movies' : 'shows');
    await api.markWatched({ id: 43, title: 'Other title', watched: false });
    assert.equal(calls.length, 2, 'overlapping clicks cannot issue another write');
    assert.deepEqual(calls[1], [kind, 42, false]);
    pending.resolve(true);
    await marking;
    assert.equal(api.successes.length, 1);
    assert.match(api.successes[0], kind === 'shows' ? /All saved episodes of Saved title/ : /^Saved title marked/);
    assert.match(api.successes[0], /marked as unwatched/);
    assert.deepEqual(copy(api.snapshot()), { busy: false, marking: null, selected: [], bulkMode: false });
    assert.equal(api.opened.length, 0, 'the eye action must not open a detail dialog');
  });
}

for (const kind of ['shows', 'movies']) {
  test(`${kind}: detail watched controls retain their target while pending and allow another attempt after failure`, async () => {
    const component = kind === 'shows' ? 'ShowDetail' : 'MovieDetail';
    const script = readFileSync(`src/lib/components/${component}.svelte`, 'utf8').match(/<script[^>]*>([\s\S]*?)<\/script>/)[1];
    const handler = kind === 'shows' ? 'handleMarkShowWatched' : 'handleToggleWatched';
    let current = { id: 42, watched: false }, episodes = [{ watched: false }];
    const calls = [], pending = deferred();
    let wait = true;
    const mark = async (id, watched) => { calls.push([id, watched]); return wait ? pending.promise : false; };
    const api = load(script + `\nexport const harness = { mark: ${handler}, pending: () => watchPendingId };`, {
      '../stores/movies.svelte': { getCurrentMovie: () => current, markMovieWatched: mark },
      '../stores/showDetail.svelte': { getCurrentShow: () => current, getShowEpisodes: () => episodes, markShowWatched: mark },
      '../stores/shows.svelte': {}, '../stores/tiers.svelte': {}, '../stores/confirmDialog.svelte': {},
      '@tauri-apps/api/core': {}, '@tauri-apps/plugin-opener': {}, '../utils/logger': {}, '../utils/dateFormat': {},
    }, { $effect: noop }).harness;
    const marking = api.mark();
    assert.equal(api.pending(), 42);
    current = { id: 43, watched: true };
    episodes = [{ watched: true }];
    await api.mark();
    assert.deepEqual(calls, [[42, true]], 'a newer detail cannot overwrite an unfinished watched action');
    pending.resolve(false);
    await marking;
    assert.equal(api.pending(), null);
    assert.equal(current.watched, true, 'failure never changes the newer detail locally');
    wait = false;
    await api.mark();
    assert.deepEqual(calls, [[42, true], [43, false]], 'a completed failure must clear the guard so another action can run');
    assert.equal(api.pending(), null);
    current = null;
    await api.mark();
    assert.equal(calls.length, 2, 'closed details have no watched target');
  });
}
