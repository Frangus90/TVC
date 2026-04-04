<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import { X, Search, Plus, Loader2, Check, Tv, Film, PenLine } from "lucide-svelte";
  import {
    isTierSearchModalOpen,
    closeTierSearchModal,
    getTiers,
    getTierListShows,
    getTierListMovies,
    addShowTierOnly,
    addMovieTierOnly,
    addManualShow,
    addManualMovie,
  } from "../stores/tiers.svelte";
  import {
    searchShows,
    getSearchResults,
    isSearchLoading,
    type SearchResult,
  } from "../stores/shows.svelte";
  import {
    searchMovies,
    getMovieSearchResults,
    isMovieSearchLoading,
    type MovieSearchResult,
  } from "../stores/movies.svelte";
  import { config } from "../config";
  import { logger } from "../utils/logger";

  type SearchMode = "shows" | "movies" | "manual";

  let searchInput: HTMLInputElement | undefined = $state();
  let debounceTimer: ReturnType<typeof setTimeout> | null = $state(null);
  let searchMode = $state<SearchMode>("shows");
  let query = $state("");
  let addingIds = $state(new Set<string>());
  let addedIds = $state(new Set<string>());
  let selectedTierId = $state<number | null>(null);

  // Manual entry fields
  let manualTitle = $state("");
  let manualPosterUrl = $state("");

  const tiers = $derived(getTiers());

  // Check if already in tier list
  function isShowInTierList(id: number): boolean {
    return addedIds.has(`show-${id}`) || getTierListShows().some(s => s.id === id);
  }

  function isMovieInTierList(id: number): boolean {
    return addedIds.has(`movie-${id}`) || getTierListMovies().some(m => m.id === id);
  }

  // Auto-focus when modal opens
  $effect(() => {
    if (isTierSearchModalOpen() && searchInput) {
      setTimeout(() => searchInput?.focus(), 10);
    }
  });

  // Reset state when modal opens
  $effect(() => {
    if (isTierSearchModalOpen()) {
      query = "";
      addingIds = new Set();
      addedIds = new Set();
      selectedTierId = null;
      manualTitle = "";
      manualPosterUrl = "";
    }
  });

  // Cleanup debounce timer
  $effect(() => {
    return () => {
      if (debounceTimer) clearTimeout(debounceTimer);
    };
  });

  function handleInput(event: Event) {
    const target = event.target as HTMLInputElement;
    query = target.value;

    if (debounceTimer) clearTimeout(debounceTimer);

    if (query.trim()) {
      debounceTimer = setTimeout(() => {
        if (searchMode === "shows") {
          searchShows(query);
        } else if (searchMode === "movies") {
          searchMovies(query);
        }
      }, config.ui.searchDebounceMs);
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      closeTierSearchModal();
    } else if (event.key === "Enter" && searchMode !== "manual") {
      if (query.trim()) {
        if (searchMode === "shows") searchShows(query);
        else searchMovies(query);
      }
    }
  }

  function switchMode(mode: SearchMode) {
    searchMode = mode;
    query = "";
  }

  function getShowId(show: SearchResult): number {
    return parseInt(show.tvdb_id || show.id || "0");
  }

  async function handleAddShow(show: SearchResult) {
    const id = getShowId(show);
    const key = `show-${id}`;
    if (!id || addingIds.has(key) || isShowInTierList(id)) return;

    addingIds = new Set(addingIds).add(key);
    try {
      await addShowTierOnly(id, selectedTierId);
      addedIds = new Set(addedIds).add(key);
    } catch (err) {
      logger.error("Failed to add show to tier list", err);
    } finally {
      const next = new Set(addingIds);
      next.delete(key);
      addingIds = next;
    }
  }

  async function handleAddMovie(movie: MovieSearchResult) {
    const key = `movie-${movie.id}`;
    if (addingIds.has(key) || isMovieInTierList(movie.id)) return;

    addingIds = new Set(addingIds).add(key);
    try {
      await addMovieTierOnly(movie.id, selectedTierId);
      addedIds = new Set(addedIds).add(key);
    } catch (err) {
      logger.error("Failed to add movie to tier list", err);
    } finally {
      const next = new Set(addingIds);
      next.delete(key);
      addingIds = next;
    }
  }

  let manualType = $state<"show" | "movie">("show");

  async function handleAddManualEntry() {
    if (!manualTitle.trim()) return;

    const key = `manual-${manualTitle}`;
    addingIds = new Set(addingIds).add(key);
    try {
      const posterUrl = manualPosterUrl.trim() || null;
      if (manualType === "show") {
        await addManualShow(manualTitle.trim(), posterUrl, selectedTierId);
      } else {
        await addManualMovie(manualTitle.trim(), posterUrl, selectedTierId);
      }
      addedIds = new Set(addedIds).add(key);
      manualTitle = "";
      manualPosterUrl = "";
    } catch (err) {
      logger.error("Failed to add manual entry", err);
    } finally {
      const next = new Set(addingIds);
      next.delete(key);
      addingIds = next;
    }
  }
</script>

<svelte:window on:keydown={handleKeyDown} />

{#if isTierSearchModalOpen()}
  <!-- Backdrop -->
  <button
    transition:fade={{ duration: 150 }}
    class="fixed inset-0 bg-black/60 z-50"
    onclick={closeTierSearchModal}
    aria-label="Close modal"
  ></button>

  <!-- Modal -->
  <div
    transition:scale={{ duration: 200, start: 0.95, opacity: 0 }}
    class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-full max-w-5xl z-50 bg-surface rounded-xl border border-border shadow-2xl flex flex-col"
    style="max-height: 700px;"
  >
    <!-- Header -->
    <div class="p-4 border-b border-border">
      <!-- Mode tabs -->
      <div class="flex items-center gap-2 mb-3">
        <div class="flex gap-1 p-1 bg-background rounded-lg">
          <button
            type="button"
            onclick={() => switchMode("shows")}
            class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md transition-colors
              {searchMode === 'shows' ? 'bg-surface text-accent' : 'text-text-muted hover:text-text'}"
          >
            <Tv class="w-3.5 h-3.5" />
            Shows
          </button>
          <button
            type="button"
            onclick={() => switchMode("movies")}
            class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md transition-colors
              {searchMode === 'movies' ? 'bg-surface text-accent' : 'text-text-muted hover:text-text'}"
          >
            <Film class="w-3.5 h-3.5" />
            Movies
          </button>
          <button
            type="button"
            onclick={() => switchMode("manual")}
            class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md transition-colors
              {searchMode === 'manual' ? 'bg-surface text-accent' : 'text-text-muted hover:text-text'}"
          >
            <PenLine class="w-3.5 h-3.5" />
            Manual
          </button>
        </div>

        <!-- Tier selector -->
        <div class="ml-auto flex items-center gap-2">
          <span class="text-xs text-text-muted">Add to:</span>
          <select
            bind:value={selectedTierId}
            class="bg-background border border-border rounded-md px-2 py-1 text-xs text-text"
          >
            <option value={null}>No tier (untiered)</option>
            {#each tiers as tier (tier.id)}
              <option value={tier.id}>
                {tier.name}
              </option>
            {/each}
          </select>
        </div>

        <button
          type="button"
          onclick={closeTierSearchModal}
          class="p-1.5 rounded-lg hover:bg-surface-hover transition-colors"
          aria-label="Close"
        >
          <X class="w-5 h-5 text-text-muted" />
        </button>
      </div>

      <!-- Search input (for shows/movies modes) -->
      {#if searchMode !== "manual"}
        <div class="flex items-center gap-3">
          <Search class="w-5 h-5 text-text-muted" />
          <input
            type="text"
            placeholder="Search for a {searchMode === 'shows' ? 'TV show' : 'movie'} to add to tier list..."
            class="flex-1 bg-transparent text-text text-lg outline-none placeholder:text-text-muted"
            value={query}
            oninput={handleInput}
            bind:this={searchInput}
            autocomplete="off"
          />
        </div>
      {/if}
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-auto min-h-0">
      {#if searchMode === "manual"}
        <!-- Manual entry form -->
        <div class="p-6 space-y-4">
          <p class="text-sm text-text-muted">
            Add a show or movie that isn't in TVDB/TMDB. These entries exist only in your tier list.
          </p>

          <div class="flex gap-2">
            <button
              type="button"
              onclick={() => manualType = "show"}
              class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md transition-colors
                {manualType === 'show' ? 'bg-accent text-white' : 'bg-background text-text-muted hover:text-text'}"
            >
              <Tv class="w-3.5 h-3.5" />
              Show
            </button>
            <button
              type="button"
              onclick={() => manualType = "movie"}
              class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md transition-colors
                {manualType === 'movie' ? 'bg-accent text-white' : 'bg-background text-text-muted hover:text-text'}"
            >
              <Film class="w-3.5 h-3.5" />
              Movie
            </button>
          </div>

          <div>
            <label for="manual-title" class="block text-sm font-medium text-text mb-1">Title</label>
            <input
              id="manual-title"
              type="text"
              bind:value={manualTitle}
              placeholder="Enter title..."
              class="w-full bg-background border border-border rounded-lg px-3 py-2 text-text outline-none focus:ring-2 focus:ring-accent"
            />
          </div>

          <div>
            <label for="manual-poster" class="block text-sm font-medium text-text mb-1">Poster URL (optional)</label>
            <input
              id="manual-poster"
              type="text"
              bind:value={manualPosterUrl}
              placeholder="https://..."
              class="w-full bg-background border border-border rounded-lg px-3 py-2 text-text outline-none focus:ring-2 focus:ring-accent"
            />
          </div>

          <button
            type="button"
            onclick={handleAddManualEntry}
            disabled={!manualTitle.trim()}
            class="flex items-center gap-2 px-4 py-2 bg-accent text-white rounded-lg hover:bg-accent/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <Plus class="w-4 h-4" />
            Add {manualType === "show" ? "Show" : "Movie"}
          </button>

          {#if addedIds.size > 0}
            <p class="text-sm text-green-400">Added successfully!</p>
          {/if}
        </div>
      {:else if searchMode === "shows"}
        {#if isSearchLoading()}
          <div class="flex items-center justify-center py-12">
            <Loader2 class="w-8 h-8 text-accent animate-spin" />
            <span class="ml-3 text-sm text-text-muted">Searching...</span>
          </div>
        {:else if getSearchResults().length === 0}
          {#if query.trim()}
            <div class="py-12 text-center">
              <p class="text-text-muted mb-2">No results found for "{query}"</p>
              <p class="text-xs text-text-muted">Try a different search term, or use Manual to add custom entries</p>
            </div>
          {:else}
            <div class="py-12 text-center text-text-muted">
              <p class="mb-2">Search for a TV show to add to your tier list</p>
              <p class="text-xs">Results appear as you type</p>
            </div>
          {/if}
        {:else}
          <div class="px-4 py-2 border-b border-border">
            <p class="text-xs text-text-muted">
              Found {getSearchResults().length} result{getSearchResults().length !== 1 ? 's' : ''}
            </p>
          </div>
          <ul class="divide-y divide-border">
            {#each getSearchResults() as show}
              {@const showId = getShowId(show)}
              <li class="flex items-start gap-4 p-4 hover:bg-surface-hover transition-colors">
                {#if show.image_url}
                  <img src={show.image_url} alt={show.name || "TV show"} class="w-16 h-24 rounded object-cover flex-shrink-0" />
                {:else}
                  <div class="w-16 h-24 rounded bg-border flex items-center justify-center flex-shrink-0">
                    <Tv class="w-6 h-6 text-text-muted" />
                  </div>
                {/if}
                <div class="flex-1 min-w-0">
                  <h3 class="font-semibold text-text truncate">{show.name || "Unknown"}</h3>
                  {#if show.year || show.network}
                    <p class="text-sm text-text-muted mt-0.5">{[show.year, show.network].filter(Boolean).join(" - ")}</p>
                  {/if}
                  {#if show.overview}
                    <p class="text-sm text-text-muted mt-2 line-clamp-2">{show.overview}</p>
                  {/if}
                </div>
                {#if addingIds.has(`show-${showId}`)}
                  <button disabled class="flex-shrink-0 p-2 bg-accent text-white rounded-lg opacity-70">
                    <Loader2 class="w-5 h-5 animate-spin" />
                  </button>
                {:else if isShowInTierList(showId)}
                  <button disabled class="flex-shrink-0 p-2 bg-green-600 text-white rounded-lg">
                    <Check class="w-5 h-5" />
                  </button>
                {:else}
                  <button
                    onclick={() => handleAddShow(show)}
                    class="flex-shrink-0 p-2 bg-accent hover:bg-accent/90 text-white rounded-lg transition-colors"
                    aria-label="Add to tier list"
                  >
                    <Plus class="w-5 h-5" />
                  </button>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
      {:else if searchMode === "movies"}
        {#if isMovieSearchLoading()}
          <div class="flex items-center justify-center py-12">
            <Loader2 class="w-8 h-8 text-accent animate-spin" />
            <span class="ml-3 text-sm text-text-muted">Searching...</span>
          </div>
        {:else if getMovieSearchResults().length === 0}
          {#if query.trim()}
            <div class="py-12 text-center">
              <p class="text-text-muted mb-2">No results found for "{query}"</p>
              <p class="text-xs text-text-muted">Try a different search term, or use Manual to add custom entries</p>
            </div>
          {:else}
            <div class="py-12 text-center text-text-muted">
              <p class="mb-2">Search for a movie to add to your tier list</p>
              <p class="text-xs">Results appear as you type</p>
            </div>
          {/if}
        {:else}
          <div class="px-4 py-2 border-b border-border">
            <p class="text-xs text-text-muted">
              Found {getMovieSearchResults().length} result{getMovieSearchResults().length !== 1 ? 's' : ''}
            </p>
          </div>
          <ul class="divide-y divide-border">
            {#each getMovieSearchResults() as movie}
              <li class="flex items-start gap-4 p-4 hover:bg-surface-hover transition-colors">
                {#if movie.poster_url}
                  <img src={movie.poster_url} alt={movie.title} class="w-16 h-24 rounded object-cover flex-shrink-0" />
                {:else}
                  <div class="w-16 h-24 rounded bg-border flex items-center justify-center flex-shrink-0">
                    <Film class="w-6 h-6 text-text-muted" />
                  </div>
                {/if}
                <div class="flex-1 min-w-0">
                  <h3 class="font-semibold text-text truncate">{movie.title}</h3>
                  {#if movie.release_date}
                    <p class="text-sm text-text-muted mt-0.5">{movie.release_date.slice(0, 4)}</p>
                  {/if}
                  {#if movie.overview}
                    <p class="text-sm text-text-muted mt-2 line-clamp-2">{movie.overview}</p>
                  {/if}
                </div>
                {#if addingIds.has(`movie-${movie.id}`)}
                  <button disabled class="flex-shrink-0 p-2 bg-accent text-white rounded-lg opacity-70">
                    <Loader2 class="w-5 h-5 animate-spin" />
                  </button>
                {:else if isMovieInTierList(movie.id)}
                  <button disabled class="flex-shrink-0 p-2 bg-green-600 text-white rounded-lg">
                    <Check class="w-5 h-5" />
                  </button>
                {:else}
                  <button
                    onclick={() => handleAddMovie(movie)}
                    class="flex-shrink-0 p-2 bg-accent hover:bg-accent/90 text-white rounded-lg transition-colors"
                    aria-label="Add to tier list"
                  >
                    <Plus class="w-5 h-5" />
                  </button>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
      {/if}
    </div>

    <!-- Footer -->
    <div class="p-4 border-t border-border">
      <button
        type="button"
        onclick={closeTierSearchModal}
        class="w-full py-2.5 text-text-muted hover:bg-surface-hover rounded-lg transition-colors"
      >
        Close
      </button>
    </div>
  </div>
{/if}

<style>
  .line-clamp-2 {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
