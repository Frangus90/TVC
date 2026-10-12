<script lang="ts">
  import { untrack } from "svelte";
  import {
    Archive, CalendarX, Check, Film, Grid2X2, Layers, List, Loader2,
    Plus, Search, Trash2, Tv, AlertTriangle,
  } from "lucide-svelte";
  import {
    getTrackedShows, getArchivedShows, loadTrackedShows, loadArchivedShows,
    openSearchModal, removeShow, archiveShow,
  } from "../stores/shows.svelte";
  import {
    getTrackedMovies, getArchivedMovies, loadTrackedMovies, loadArchivedMovies,
    openMovieSearchModal, openMovieDetail, removeMovie, archiveMovie, unscheduleMovie,
  } from "../stores/movies.svelte";
  import { openShowDetail } from "../stores/showDetail.svelte";
  import {
    getTierListShows, getTierListMovies, loadTierListShows, loadTierListMovies,
    loadTiers, getTierById,
  } from "../stores/tiers.svelte";
  import { getThemeSettings } from "../stores/theme.svelte";
  import { setSidebarTab } from "../stores/sidebar.svelte";
  import { openConfirmDialog } from "../stores/confirmDialog.svelte";
  import { formatDate } from "../utils/dateFormat";
  import UnmigratedShowsResolver from "./UnmigratedShowsResolver.svelte";

  interface Props { mediaType: "shows" | "movies" }
  interface LibraryItem {
    id: number;
    title: string;
    poster: string | null;
    color: string | null;
    status: string | null;
    tierId: number | null;
    tierOnly: boolean;
    releaseDate: string | null;
    runtime: number | null;
    scheduledDate: string | null;
    watched: boolean;
    unmigrated: boolean;
  }

  let { mediaType }: Props = $props();
  let query = $state("");
  let filter = $state<"all" | "tracking" | "tier-only">("all");
  let layout = $state<"posters" | "list">("posters");
  let bulkMode = $state(false);
  let selected = $state<Set<number>>(new Set());
  let loading = $state(true);
  let busy = $state(false);
  let resolverOpen = $state(false);
  const theme = $derived(getThemeSettings());
  const isList = $derived(theme.hidePosters || layout === "list");
  const noun = $derived(mediaType === "shows" ? "show" : "movie");
  const title = $derived(mediaType === "shows" ? "TV shows" : "Movies");

  // Tier-list records include archived titles; those stay in Data's Archive.
  const items = $derived.by((): LibraryItem[] => {
    const result = new Map<number, LibraryItem>();
    if (mediaType === "shows") {
      const archived = new Set(getArchivedShows().map((show) => show.id));
      const tierShows = getTierListShows();
      const tiersByShow = new Map(tierShows.map((show) => [show.id, show]));
      for (const show of tierShows) {
        if (!show.tier_only || archived.has(show.id)) continue;
        result.set(show.id, {
          id: show.id, title: show.name, poster: show.poster_url, color: null,
          status: null, tierId: show.tier_id, tierOnly: true,
          releaseDate: show.first_aired, runtime: null, scheduledDate: null,
          watched: false, unmigrated: false,
        });
      }
      for (const show of getTrackedShows()) {
        if (archived.has(show.id)) continue;
        const tierShow = tiersByShow.get(show.id);
        result.set(show.id, {
          id: show.id, title: show.name, poster: show.poster_url, color: show.color,
          status: show.status, tierId: tierShow?.tier_id ?? null, tierOnly: show.tier_only,
          releaseDate: tierShow?.first_aired ?? null, runtime: null, scheduledDate: null,
          watched: false, unmigrated: show.unmigrated,
        });
      }
    } else {
      const archived = new Set(getArchivedMovies().map((movie) => movie.id));
      const tierMovies = getTierListMovies();
      const tiersByMovie = new Map(tierMovies.map((movie) => [movie.id, movie]));
      for (const movie of tierMovies) {
        if (!movie.tier_only || archived.has(movie.id)) continue;
        result.set(movie.id, {
          id: movie.id, title: movie.title, poster: movie.poster_url, color: null,
          status: null, tierId: movie.tier_id, tierOnly: true,
          releaseDate: movie.release_date, runtime: null, scheduledDate: null,
          watched: false, unmigrated: false,
        });
      }
      for (const movie of getTrackedMovies()) {
        if (archived.has(movie.id)) continue;
        result.set(movie.id, {
          id: movie.id, title: movie.title, poster: movie.poster_url, color: movie.color,
          status: movie.status,
          tierId: tiersByMovie.get(movie.id)?.tier_id ?? null,
          tierOnly: movie.tier_only, releaseDate: movie.release_date, runtime: movie.runtime,
          scheduledDate: movie.scheduled_date, watched: movie.watched, unmigrated: false,
        });
      }
    }
    return [...result.values()].sort((a, b) => a.title.localeCompare(b.title));
  });
  const trackingCount = $derived(items.filter((item) => !item.tierOnly).length);
  const tierOnlyCount = $derived(items.length - trackingCount);
  const unmappedCount = $derived(items.filter((item) => item.unmigrated).length);
  const visibleItems = $derived(items.filter((item) =>
    (filter === "all" || item.tierOnly === (filter === "tier-only")) &&
    item.title.toLowerCase().includes(query.trim().toLowerCase())
  ));
  const allShownSelected = $derived(visibleItems.length > 0 && visibleItems.every((item) => selected.has(item.id)));

  $effect(() => {
    const kind = mediaType;
    let active = true;
    untrack(() => {
      query = "";
      filter = "all";
      selected = new Set();
      bulkMode = false;
      loading = true;
      loadLibrary(kind).finally(() => { if (active) loading = false; });
    });
    return () => { active = false; };
  });

  async function loadLibrary(kind: Props["mediaType"]) {
    await Promise.all(kind === "shows"
      ? [loadTrackedShows(), loadArchivedShows(), loadTierListShows(), loadTiers()]
      : [loadTrackedMovies(), loadArchivedMovies(), loadTierListMovies(), loadTiers()]);
  }

  function addTitle() {
    if (mediaType === "shows") openSearchModal();
    else openMovieSearchModal();
  }

  function openItem(item: LibraryItem) {
    if (bulkMode) toggleSelection(item.id);
    else if (mediaType === "shows") openShowDetail(item.id);
    else openMovieDetail(item.id);
  }

  function toggleSelection(id: number) {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    selected = next;
  }

  function selectShown() {
    const next = new Set(selected);
    for (const item of visibleItems) {
      if (allShownSelected) next.delete(item.id);
      else next.add(item.id);
    }
    selected = next;
  }

  function cancelSelection() {
    bulkMode = false;
    selected = new Set();
  }

  async function changeItems(action: "archive" | "remove", ids: number[]) {
    if (!ids.length || busy) return;
    const kind = mediaType;
    const itemLabel = kind === "shows" ? "show" : "movie";
    const label = action === "archive" ? "Archive" : "Remove";
    const subject = ids.length === 1
      ? `“${items.find((item) => item.id === ids[0])?.title ?? itemLabel}”`
      : `${ids.length} ${itemLabel}s`;
    busy = true;
    try {
      const confirmed = await openConfirmDialog({
        title: `${label} ${ids.length === 1 ? itemLabel : "titles"}`,
        message: action === "archive"
          ? `Archive ${subject}? Their history and tiers will be kept. You can restore them from Data → Archive.`
          : `Remove ${subject} from TVC? Their saved history, schedules and tier assignments will be deleted. This cannot be undone.`,
        type: action === "remove" ? "danger" : "info",
        confirmLabel: label,
      });
      if (!confirmed) return;
      for (const id of ids) {
        if (kind === "shows") await (action === "archive" ? archiveShow(id) : removeShow(id));
        else await (action === "archive" ? archiveMovie(id) : removeMovie(id));
      }
      // Existing action stores report failures. Reload the tier-only records too,
      // keeping failed titles selected rather than reporting a false success.
      await (kind === "shows" ? loadTierListShows() : loadTierListMovies());
      if (kind === mediaType) {
        selected = new Set([...selected].filter((id) => items.some((item) => item.id === id)));
        if (!selected.size) bulkMode = false;
      }
    } finally {
      busy = false;
    }
  }

  async function unschedule(item: LibraryItem) {
    if (busy) return;
    busy = true;
    try { await unscheduleMovie(item.id); }
    finally { busy = false; }
  }

  function metadata(item: LibraryItem) {
    const parts = [item.releaseDate?.slice(0, 4), item.runtime ? `${item.runtime} min` : null];
    if (item.status && mediaType === "shows") parts.push(item.status);
    return parts.filter(Boolean).join(" · ");
  }
</script>

<section class="focus-page library-page" class:library-compact={theme.compactSpacing} aria-labelledby="library-heading">
  <header class="focus-page-heading focus-page-bar library-toolbar">
    <h1 id="library-heading">{title}</h1>
    <div class="library-filters" aria-label="Library filter">
      {#each [["all", "All", items.length], ["tracking", "Tracking", trackingCount], ["tier-only", "Tier only", tierOnlyCount]] as [value, label, count]}
        <button type="button" aria-pressed={filter === value} class:active={filter === value}
          onclick={() => filter = value as typeof filter}>{label} <span>{count}</span></button>
      {/each}
    </div>
    <div class="library-tools">
      <label class="library-search">
        <Search size={16} aria-hidden="true" />
        <input type="search" bind:value={query} aria-label={`Search your ${title.toLowerCase()}`} placeholder={`Search ${title.toLowerCase()}`} />
      </label>
      {#if !theme.hidePosters}
        <div class="library-layout" aria-label="Library layout">
          <button type="button" aria-label="Poster view" title="Poster view" aria-pressed={!isList} onclick={() => layout = "posters"}><Grid2X2 size={17} /></button>
          <button type="button" aria-label="List view" title="List view" aria-pressed={isList} onclick={() => layout = "list"}><List size={17} /></button>
        </div>
      {/if}
      <button type="button" class="library-button" onclick={() => setSidebarTab("tiers")}><Layers size={16} /> Tier list</button>
      {#if !bulkMode}
        <button type="button" class="library-button" disabled={loading || !items.length} onclick={() => bulkMode = true}>Select</button>
      {/if}
      <button type="button" onclick={addTitle} class="library-button library-primary"><Plus size={17} /> Add a {noun}</button>
    </div>
  </header>

  {#if mediaType === "shows" && unmappedCount > 0}
    <div class="focus-panel library-notice">
      <AlertTriangle size={18} class="text-amber-400 shrink-0" />
      <p>{unmappedCount} {unmappedCount === 1 ? "show needs" : "shows need"} a TMDB match before their episodes can sync.</p>
      <button type="button" class="library-button" onclick={() => resolverOpen = true}>Resolve shows</button>
    </div>
  {/if}

  {#if bulkMode}
    <div class="focus-panel library-selection" role="group" aria-label="Selected titles">
      <span class="font-medium" aria-live="polite">{selected.size} selected</span>
      <button type="button" class="library-button" disabled={busy || !visibleItems.length} onclick={selectShown}>{allShownSelected ? "Deselect shown" : "Select shown"}</button>
      <span class="library-selection-spacer"></span>
      <button type="button" class="library-button" disabled={busy || !selected.size} onclick={() => changeItems("archive", [...selected])}><Archive size={16} /> Archive</button>
      <button type="button" class="library-button library-danger" disabled={busy || !selected.size} onclick={() => changeItems("remove", [...selected])}><Trash2 size={16} /> Remove</button>
      <button type="button" class="library-button" disabled={busy} onclick={cancelSelection}>Cancel</button>
    </div>
  {/if}

  {#if loading}
    <div class="library-loading" role="status"><Loader2 size={24} class="animate-spin" /> Loading your {title.toLowerCase()}…</div>
  {:else if !items.length}
    <div class="focus-panel library-empty">
      {#if mediaType === "shows"}<Tv size={36} />{:else}<Film size={36} />{/if}
      <h2>No {title.toLowerCase()} yet</h2>
      <p class="text-text-muted">Add a {noun} to track it, or save titles in your tier list.</p>
      <button type="button" class="library-button library-primary" onclick={addTitle}><Plus size={16} /> Add a {noun}</button>
    </div>
  {:else if !visibleItems.length}
    <div class="focus-panel library-empty">
      <Search size={32} />
      <h2>No matching titles</h2>
      <p class="text-text-muted">Try another name or change the filter.</p>
      <button type="button" class="library-button" onclick={() => { query = ""; filter = "all"; }}>Show all titles</button>
    </div>
  {:else}
    <ul class="library-items" class:library-list={isList} aria-label={title}>
      {#each visibleItems as item (item.id)}
        {@const tier = item.tierId !== null ? getTierById(item.tierId) : undefined}
        <li class="library-card" class:selected={bulkMode && selected.has(item.id)}>
          <button type="button" class="library-open" disabled={busy}
            aria-label={bulkMode ? `${selected.has(item.id) ? "Deselect" : "Select"} ${item.title}` : `Open ${item.title}`}
            aria-pressed={bulkMode ? selected.has(item.id) : undefined} onclick={() => openItem(item)}>
            {#if bulkMode}
              <span class="library-checkbox" class:checked={selected.has(item.id)} aria-hidden="true">{#if selected.has(item.id)}<Check size={14} />{/if}</span>
            {/if}
            {#if !theme.hidePosters}
              <span class="library-poster">
                {#if item.poster}<img src={item.poster} alt="" loading="lazy" decoding="async" />
                {:else if mediaType === "shows"}<Tv size={30} />{:else}<Film size={30} />{/if}
              </span>
            {/if}
            <span class="library-copy">
              <span class="library-title">{#if item.color}<span class="library-color" style:background={item.color}></span>{/if}{item.title}</span>
              {#if metadata(item)}<span class="library-metadata">{metadata(item)}</span>{/if}
              <span class="library-state">
                <span>{item.tierOnly ? "Tier only" : mediaType === "movies" && item.watched ? "Watched" : "Tracking"}</span>
                {#if item.unmigrated}<span class="library-unmapped">Needs matching</span>{/if}
                {#if tier}<span class="library-tier" style:--tier-color={tier.color}>{tier.name}</span>{/if}
              </span>
              {#if item.scheduledDate}<span class="library-schedule">Scheduled · {formatDate(item.scheduledDate)}</span>{/if}
            </span>
          </button>
          {#if !bulkMode}
            <div class="library-actions">
              {#if mediaType === "movies" && item.scheduledDate}
                <button type="button" disabled={busy} onclick={() => unschedule(item)} aria-label={`Remove ${item.title} from schedule`} title="Remove from schedule"><CalendarX size={16} /></button>
              {/if}
              <button type="button" disabled={busy} onclick={() => changeItems("archive", [item.id])} aria-label={`Archive ${item.title}`} title="Archive"><Archive size={16} /></button>
              <button type="button" disabled={busy} class="library-danger" onclick={() => changeItems("remove", [item.id])} aria-label={`Remove ${item.title}`} title="Remove"><Trash2 size={16} /></button>
            </div>
          {/if}
        </li>
      {/each}
    </ul>
    <p class="library-count">Showing {visibleItems.length} of {items.length} titles · Archived titles are in Data → Archive</p>
  {/if}
</section>

<UnmigratedShowsResolver open={resolverOpen} onClose={() => resolverOpen = false} />

<style>
  .library-page { min-width: 0; }
  .library-button { display: inline-flex; align-items: center; justify-content: center; gap: .5rem; padding: .6rem .85rem; border: 1px solid var(--color-border); border-radius: .6rem; background: var(--color-surface); font-size: .82em; font-weight: 500; white-space: nowrap; }
  .library-button:hover { background: var(--color-surface-hover); }
  .library-button:disabled, .library-actions button:disabled { opacity: .45; cursor: default; }
  .library-primary { color: var(--accent-foreground); background: var(--color-accent); border-color: transparent; }
  .library-primary:hover { background: var(--color-accent-hover); }
  .library-danger { color: #f28b94; }
  .library-notice { display: flex; align-items: center; flex-wrap: wrap; gap: .75rem; margin-bottom: 1.25rem; padding: .85rem 1rem; font-size: .85em; }
  .library-notice p { flex: 1; }
  .library-toolbar { justify-content: flex-start; }
  .library-toolbar h1 { flex: none; }
  .library-filters, .library-tools, .library-layout { display: flex; align-items: center; gap: .4rem; }
  .library-tools { flex-wrap: wrap; margin-left: auto; }
  .library-filters button { display: flex; align-items: center; gap: .45rem; padding: .55rem .8rem; border-radius: .55rem; font-size: .8em; color: var(--color-text-muted); }
  .library-filters button:hover, .library-layout button:hover { background: var(--color-surface-hover); color: var(--color-text); }
  .library-filters button.active { background: color-mix(in srgb, var(--color-accent) 13%, transparent); color: var(--color-accent); }
  .library-filters span { opacity: .8; font-size: .9em; }
  .library-search { display: flex; align-items: center; gap: .6rem; color: var(--color-text-muted); padding: .6rem .8rem; background: var(--color-surface); border: 1px solid var(--color-border); border-radius: .6rem; }
  .library-search:focus-within { border-color: var(--color-accent); }
  .library-search input { background: transparent; border: none; outline: none; color: var(--color-text); font-size: .8em; width: 12rem; min-width: 0; }
  .library-layout { border: 1px solid var(--color-border); border-radius: .6rem; padding: .2rem; gap: .15rem; }
  .library-layout button { padding: .4rem; border-radius: .4rem; color: var(--color-text-muted); }
  .library-layout button[aria-pressed="true"] { color: var(--color-accent); background: var(--color-surface-hover); }
  .library-selection { display: flex; flex-wrap: wrap; align-items: center; gap: .6rem; padding: .7rem 1rem; margin-bottom: 1.2rem; font-size: .85em; }
  .library-selection-spacer { flex: 1; }
  .library-loading, .library-empty { display: flex; align-items: center; justify-content: center; gap: 1rem; padding: 5rem 1rem; color: var(--color-text-muted); }
  .library-empty { flex-direction: column; text-align: center; }
  .library-empty h2 { color: var(--color-text); font-weight: 600; font-size: 1.15em; }
  .library-items { display: grid; grid-template-columns: repeat(auto-fill, minmax(165px, 1fr)); gap: 1.6rem 1.25rem; list-style: none; margin: 0; padding: 0; }
  .library-card { position: relative; min-width: 0; padding: .45rem; border: 1px solid transparent; border-radius: .85rem; }
  .library-card:hover { background: var(--color-surface); }
  .library-card.selected { background: color-mix(in srgb, var(--color-accent) 8%, var(--color-surface)); border-color: var(--color-accent); }
  .library-open { display: flex; flex-direction: column; width: 100%; text-align: left; border-radius: .65rem; }
  .library-poster { display: flex; width: 100%; aspect-ratio: 2 / 3; align-items: center; justify-content: center; overflow: hidden; border: 1px solid var(--color-border); border-radius: .65rem; background: var(--color-surface-hover); color: var(--color-text-muted); }
  .library-poster img { width: 100%; height: 100%; object-fit: cover; }
  .library-copy { display: flex; flex-direction: column; min-width: 0; width: 100%; padding-top: .75rem; gap: .3rem; }
  .library-title { display: block; font-size: .85em; font-weight: 600; line-height: 1.45; overflow-wrap: anywhere; }
  .library-color { display: inline-block; width: .55rem; height: .55rem; border-radius: 50%; margin-right: .4rem; }
  .library-metadata, .library-schedule { color: var(--color-text-muted); font-size: .72em; }
  .library-schedule { color: var(--color-accent); }
  .library-state { display: flex; align-items: center; flex-wrap: wrap; gap: .5rem; font-size: .7em; color: var(--color-text-muted); margin-top: .2rem; }
  .library-tier { border: 1px solid color-mix(in srgb, var(--tier-color) 30%, transparent); background: color-mix(in srgb, var(--tier-color) 10%, transparent); color: var(--tier-color); padding: .15rem .45rem; border-radius: .3rem; }
  .library-unmapped { color: #f3c478; }
  .library-actions { display: flex; justify-content: flex-end; gap: .15rem; padding-top: .4rem; }
  .library-actions button { padding: .35rem; border-radius: .4rem; color: var(--color-text-muted); }
  .library-actions button:hover { background: var(--color-surface-hover); color: var(--color-accent); }
  .library-actions button.library-danger:hover { color: #f28b94; }
  .library-checkbox { position: absolute; top: .9rem; left: .9rem; display: flex; align-items: center; justify-content: center; width: 1.3rem; height: 1.3rem; border: 1px solid var(--color-text-muted); border-radius: .3rem; background: var(--color-surface); }
  .library-checkbox.checked { background: var(--color-accent); color: white; border-color: var(--color-accent); }
  .library-count { margin-top: 1.75rem; font-size: .75em; color: var(--color-text-muted); }
  .library-list { grid-template-columns: 1fr; gap: .5rem; }
  .library-list .library-card { display: flex; align-items: center; gap: 1rem; padding: .8rem 1rem; border-color: var(--color-border); background: var(--color-surface); }
  .library-list .library-open { flex: 1; min-width: 0; flex-direction: row; align-items: center; gap: 1rem; }
  .library-list .library-poster { width: 3rem; height: 4.5rem; flex-shrink: 0; }
  .library-list .library-copy { padding-top: 0; }
  .library-list .library-actions { flex-shrink: 0; padding-top: 0; }
  .library-list .library-checkbox { position: static; flex-shrink: 0; }
  .library-compact .library-items:not(.library-list) { grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: 1rem .85rem; }
  .library-compact .library-list .library-card { padding: .5rem .75rem; }
  .library-compact .library-list .library-poster { width: 2rem; height: 3rem; }
  @media (min-width: 2400px) {
    .library-items:not(.library-list) { grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); }
    .library-compact .library-items:not(.library-list) { grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); }
  }
  @media (max-width: 700px) {
    .library-tools { width: 100%; }
    .library-search { flex: 1; }
    .library-search input { width: 100%; }
    .library-items:not(.library-list) { grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: 1rem; }
    .library-list .library-card { gap: .5rem; padding: .65rem; }
    .library-list .library-open { gap: .65rem; }
    .library-list .library-poster { width: 2.5rem; height: 3.75rem; }
  }
</style>
