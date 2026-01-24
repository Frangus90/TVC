<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import { X, Search, Plus, Loader2, Film, Star } from "lucide-svelte";
  import {
    isMovieSearchModalOpen,
    closeMovieSearchModal,
    getMovieSearchQuery,
    setMovieSearchQuery,
    searchMovies,
    getMovieSearchResults,
    isMovieSearchLoading,
    addMovie,
    type MovieSearchResult,
  } from "../stores/movies.svelte";
  import { config } from "../config";

  let searchInput: HTMLInputElement | undefined = $state();
  let debounceTimer: ReturnType<typeof setTimeout> | null = $state(null);

  // Auto-focus when modal opens
  $effect(() => {
    if (isMovieSearchModalOpen() && searchInput) {
      setTimeout(() => searchInput?.focus(), 10);
    }
  });

  // Cleanup debounce timer
  $effect(() => {
    return () => {
      if (debounceTimer) {
        clearTimeout(debounceTimer);
      }
    };
  });

  function handleInput(event: Event) {
    const target = event.target as HTMLInputElement;
    const value = target.value;
    setMovieSearchQuery(value);

    // Clear existing timer
    if (debounceTimer) {
      clearTimeout(debounceTimer);
    }

    // Debounce search - auto-search after configured delay
    if (value.trim()) {
      debounceTimer = setTimeout(() => {
        searchMovies(value);
      }, config.ui.searchDebounceMs);
    } else {
      // Clear results immediately if query is empty
      searchMovies("");
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      closeMovieSearchModal();
    } else if (event.key === "Enter") {
      const query = getMovieSearchQuery();
      if (query.trim()) {
        searchMovies(query);
      }
    }
  }

  function handleSubmit(event: Event) {
    event.preventDefault();
    const query = getMovieSearchQuery();
    if (query.trim()) {
      searchMovies(query);
    }
  }

  async function handleAddMovie(movie: MovieSearchResult) {
    await addMovie(movie);
  }

  function formatYear(date: string | null): string {
    if (!date) return "";
    return date.split("-")[0];
  }

  function formatRating(rating: number | null): string {
    if (!rating) return "";
    return rating.toFixed(1);
  }
</script>

<svelte:window on:keydown={handleKeyDown} />

{#if isMovieSearchModalOpen()}
  <!-- Backdrop -->
  <button
    transition:fade={{ duration: 150 }}
    class="fixed inset-0 bg-black/60 z-50"
    onclick={closeMovieSearchModal}
    aria-label="Close modal"
  ></button>

  {@const resultCount = getMovieSearchResults().length}
  {@const hasResults = resultCount > 0}
  {@const estimatedItemHeight = 120}
  {@const baseHeight = 200}
  {@const minHeight = 400}
  {@const maxHeight = 800}
  {@const calculatedHeight = hasResults ? Math.min(Math.max(resultCount * estimatedItemHeight + baseHeight, minHeight), maxHeight) : minHeight}

  <!-- Modal -->
  <div
    transition:scale={{ duration: 200, start: 0.95, opacity: 0 }}
    class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-full max-w-5xl z-50 bg-surface rounded-xl border border-border shadow-2xl flex flex-col"
    style="max-height: {calculatedHeight}px;"
  >
    <!-- Header -->
    <form onsubmit={handleSubmit} class="flex items-center gap-3 p-4 border-b border-border">
      <Search class="w-5 h-5 text-text-muted" />
      <input
        type="text"
        placeholder="Search for a movie... (press Enter)"
        class="flex-1 bg-transparent text-text text-lg outline-none placeholder:text-text-muted"
        value={getMovieSearchQuery()}
        oninput={handleInput}
        bind:this={searchInput}
      />
      <button
        type="button"
        onclick={closeMovieSearchModal}
        class="p-1.5 rounded-lg hover:bg-surface-hover transition-colors"
        aria-label="Close"
      >
        <X class="w-5 h-5 text-text-muted" />
      </button>
    </form>

    <!-- Results -->
    <div class="flex-1 overflow-auto min-h-0">
      {#if isMovieSearchLoading()}
        <div class="flex items-center justify-center py-12">
          <Loader2 class="w-8 h-8 text-accent animate-spin" />
          <span class="ml-3 text-sm text-text-muted">Searching...</span>
        </div>
      {:else if getMovieSearchResults().length === 0}
        {#if getMovieSearchQuery().trim()}
          <div class="py-12 text-center">
            <p class="text-text-muted mb-2">No results found for "{getMovieSearchQuery()}"</p>
            <p class="text-xs text-text-muted">Try a different search term</p>
          </div>
        {:else}
          <div class="py-12 text-center text-text-muted">
            <p class="mb-2">Type a movie name to search</p>
            <p class="text-xs">Search will start automatically as you type</p>
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
                <img
                  src={movie.poster_url}
                  alt=""
                  class="w-16 h-24 rounded object-cover flex-shrink-0"
                />
              {:else}
                <div class="w-16 h-24 rounded bg-border flex items-center justify-center flex-shrink-0">
                  <Film class="w-6 h-6 text-text-muted" />
                </div>
              {/if}
              <div class="flex-1 min-w-0">
                <h3 class="font-semibold text-text truncate">
                  {movie.title || "Unknown"}
                </h3>
                <div class="flex items-center gap-2 mt-0.5">
                  {#if movie.release_date}
                    <span class="text-sm text-text-muted">
                      {formatYear(movie.release_date)}
                    </span>
                  {/if}
                  {#if movie.vote_average && movie.vote_average > 0}
                    <span class="flex items-center gap-1 text-sm text-amber-400">
                      <Star class="w-3.5 h-3.5 fill-current" />
                      {formatRating(movie.vote_average)}
                    </span>
                  {/if}
                </div>
                {#if movie.overview}
                  <p class="text-sm text-text-muted mt-2 line-clamp-2">
                    {movie.overview}
                  </p>
                {/if}
              </div>
              <button
                onclick={() => handleAddMovie(movie)}
                class="flex-shrink-0 p-2 bg-accent hover:bg-accent-hover text-white rounded-lg transition-colors"
                aria-label="Add movie"
              >
                <Plus class="w-5 h-5" />
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <!-- Footer -->
    <div class="p-4 border-t border-border">
      <button
        type="button"
        onclick={closeMovieSearchModal}
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
