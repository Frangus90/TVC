<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { Archive, RotateCcw, Tv, Film, Search, RefreshCw, Check, CircleX } from "lucide-svelte";
  import {
    getArchivedShows, loadArchivedShows, unarchiveShow, isArchivedShowsLoading, loadTrackedShows, refreshCalendar,
  } from "../stores/shows.svelte";
  import {
    getArchivedMovies, loadArchivedMovies, unarchiveMovie, isArchivedMoviesLoading, openMovieDetail, refreshMoviesCalendar,
  } from "../stores/movies.svelte";
  import { openShowDetail } from "../stores/showDetail.svelte";
  import { getThemeSettings } from "../stores/theme.svelte";
  import { loadTierListShows, loadTierListMovies } from "../stores/tiers.svelte";
  import { matchesTierSearch } from "../utils/tierSearch";
  import EmptyState from "./common/EmptyState.svelte";
  import LoadingSpinner from "./common/LoadingSpinner.svelte";

  interface ArchiveItem {
    id: number;
    name: string;
    poster_url: string | null;
    type: "show" | "movie";
    color: string | null;
    release_date: string | null;
    tier_only: boolean;
    needs_matching: boolean;
  }

  let filter = $state<"all" | "show" | "movie">("all");
  let query = $state("");
  let reloading = $state(true);
  let loadError = $state<string | null>(null);
  let restoreMessage = $state<string | null>(null);
  let restoreFailed = $state(false);
  let restoring = $state<ArchiveItem | null>(null);
  let loadRequest = 0;
  const theme = $derived(getThemeSettings());

  const items = $derived<ArchiveItem[]>([
    ...getArchivedShows().map(show => ({
      id: show.id, name: show.name, poster_url: show.poster_url, type: "show" as const,
      color: show.color, release_date: show.first_aired ?? null,
      tier_only: show.tier_only, needs_matching: show.unmigrated,
    })),
    ...getArchivedMovies().map(movie => ({
      id: movie.id, name: movie.title, poster_url: movie.poster_url, type: "movie" as const,
      color: movie.color, release_date: movie.release_date,
      tier_only: movie.tier_only, needs_matching: false,
    })),
  ].sort((a, b) => a.name.localeCompare(b.name)));
  const filteredItems = $derived(items.filter(item =>
    (filter === "all" || item.type === filter) && matchesTierSearch(item.name, item.release_date, query),
  ));
  const showCount = $derived(items.filter(item => item.type === "show").length);
  const movieCount = $derived(items.filter(item => item.type === "movie").length);
  const loading = $derived(reloading || (!restoring && (isArchivedShowsLoading() || isArchivedMoviesLoading())));

  async function reload() {
    if (restoring) return;
    const request = ++loadRequest;
    reloading = true; loadError = null;
    try {
      const [showsOk, moviesOk, ...viewResults] = await Promise.all([
        loadArchivedShows(), loadArchivedMovies(), loadTrackedShows(), loadTierListShows(), loadTierListMovies(),
        refreshCalendar(), refreshMoviesCalendar(),
      ]);
      if (request !== loadRequest) return;
      if (showsOk === false || moviesOk === false) {
        loadError = `Could not load archived ${showsOk === false && moviesOk === false ? "shows and movies" : showsOk === false ? "shows" : "movies"}. Reload to try again.`;
      } else if (viewResults.some(result => result === false)) {
        loadError = "Archive loaded, but some library or calendar views could not refresh. Reload to try again.";
      } else {
        restoreMessage = null;
      }
    } catch (error) {
      if (request === loadRequest) loadError = `Could not load Archive: ${String(error)}`;
    } finally { if (request === loadRequest) reloading = false; }
  }

  async function restore(item: ArchiveItem) {
    if (restoring || loading) return;
    restoring = item; restoreMessage = null; restoreFailed = false;
    try {
      const onFailure = (message: string) => { restoreMessage = message; restoreFailed = true; };
      const ok = item.type === "show" ? await unarchiveShow(item.id, onFailure) : await unarchiveMovie(item.id, onFailure);
      if (ok) restoreMessage = `Restored "${item.name}" to ${item.tier_only ? "your tier list" : "your library"}.`;
      else if (!restoreMessage) { restoreMessage = "Restore needs attention. Reload Archive and check the error notification."; restoreFailed = true; }
    } catch (error) {
      restoreMessage = `Restore needs attention: ${String(error)}`; restoreFailed = true;
    } finally { restoring = null; }
  }

  function openDetails(item: ArchiveItem) {
    if (item.type === "show") void openShowDetail(item.id);
    else void openMovieDetail(item.id);
  }
  function year(item: ArchiveItem) { return item.release_date?.match(/^\d{4}/)?.[0] ?? "Year unknown"; }

  onMount(() => { void reload(); });
  onDestroy(() => { loadRequest++; });
</script>

<div class="focus-toolbar flex flex-wrap items-center gap-3 mb-3">
  <div class="flex items-center gap-1" role="group" aria-label="Archived title types">
    {#each [{ id: 'all', label: 'All', count: items.length }, { id: 'show', label: 'TV', count: showCount }, { id: 'movie', label: 'Movies', count: movieCount }] as option}
      <button type="button" onclick={() => { filter = option.id as typeof filter; }} aria-pressed={filter === option.id}
        class="px-3 py-1.5 rounded-lg text-sm transition-colors {filter === option.id ? 'bg-accent/15 text-accent' : 'text-text-muted hover:bg-surface-hover hover:text-text'}">
        {option.label} <span class="text-xs opacity-75">{option.count}</span>
      </button>
    {/each}
  </div>
  <label class="flex items-center gap-2 rounded-lg border border-border px-3 py-2 flex-1 min-w-[180px]">
    <Search size={16} class="text-text-muted shrink-0" />
    <input type="search" bind:value={query} placeholder="Search title or year" aria-label="Search archived titles or years" class="bg-transparent outline-none text-sm w-full min-w-0" />
  </label>
  <button type="button" onclick={() => void reload()} disabled={loading || !!restoring} class="focus-button flex items-center gap-2 disabled:opacity-50" aria-label="Reload Archive">
    <RefreshCw size={16} class={reloading ? 'animate-spin' : ''} /> Reload
  </button>
</div>

{#if restoring}
  <p role="status" class="text-sm text-text-muted mb-3">Restoring "{restoring.name}"…</p>
{/if}
{#if restoreMessage}
  <div role={restoreFailed ? 'alert' : 'status'} class="mb-3 rounded-lg border p-3 text-sm {restoreFailed ? 'border-error/30 bg-error/10 text-error' : 'border-success/30 bg-success/10 text-success'}">
    <p class="flex items-start gap-2">{#if restoreFailed}<CircleX size={16} class="shrink-0 mt-0.5" />{:else}<Check size={16} class="shrink-0 mt-0.5" />{/if}<span>{restoreMessage}</span></p>
  </div>
{/if}

{#if loading}
  <div class="flex items-center justify-center py-8"><LoadingSpinner text="Loading Archive…" /></div>
{:else if loadError}
  <div role="alert" class="focus-panel p-5 text-center">
    <p class="text-error mb-3">{loadError}</p>
    <button type="button" onclick={() => void reload()} class="focus-button">Reload Archive</button>
  </div>
{:else if items.length === 0}
  <EmptyState icon={Archive} title="No archived titles" message="Archive shows or movies you're done with to keep your library tidy." />
{:else if filteredItems.length === 0}
  <EmptyState icon={Search} title="No matching archived titles" message="Try another title or year, or choose All." />
  <div class="text-center"><button type="button" onclick={() => { query = ''; filter = 'all'; }} class="focus-button">Clear filters</button></div>
{:else}
  <ul class="space-y-2">
    {#each filteredItems as item (item.type + item.id)}
      <li class="focus-panel flex items-center gap-3 px-4 border border-border rounded-xl {theme.compactSpacing ? 'py-2' : 'py-3'}">
        {#if !theme.hidePosters}
          {#if item.poster_url}
            <img src={item.poster_url} alt="" class="w-8 h-12 rounded object-cover shrink-0" loading="lazy" decoding="async" />
          {:else}
            <div class="w-8 h-12 rounded bg-border flex items-center justify-center shrink-0">
              {#if item.type === 'show'}<Tv size={16} class="text-text-muted" />{:else}<Film size={16} class="text-text-muted" />{/if}
            </div>
          {/if}
        {/if}
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-2">
            {#if item.color}<span class="w-2.5 h-2.5 rounded-full shrink-0" style:background-color={item.color}></span>{/if}
            <button type="button" onclick={() => openDetails(item)} class="text-sm text-left font-medium hover:text-accent hover:underline break-words min-w-0" aria-label={`Open details for ${item.name}`}>{item.name}</button>
          </div>
          <p class="text-xs text-text-muted mt-1 flex flex-wrap gap-x-2 gap-y-1">
            <span>{item.type === 'show' ? 'TV show' : 'Movie'}</span><span>· {year(item)}</span>
            <span>· {item.tier_only ? 'Tier only' : 'Tracking'}</span>
            {#if item.needs_matching}<span class="text-warning">· Needs matching</span>{/if}
          </p>
        </div>
        <button type="button" onclick={() => void restore(item)} disabled={!!restoring}
          class="flex items-center gap-1.5 px-3 py-2 rounded-lg border border-border hover:bg-accent/10 text-accent text-sm shrink-0 disabled:opacity-50"
          aria-label={`Restore ${item.name} from archive`}>
          <RotateCcw size={16} class={restoring?.id === item.id && restoring?.type === item.type ? 'animate-spin' : ''} />
          {restoring?.id === item.id && restoring?.type === item.type ? 'Restoring…' : 'Restore'}
        </button>
      </li>
    {/each}
  </ul>
{/if}
