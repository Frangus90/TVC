<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import { X, Search, Plus, Loader2 } from "lucide-svelte";
  import {
    isSearchModalOpen,
    closeSearchModal,
    getSearchQuery,
    setSearchQuery,
    searchShows,
    getSearchResults,
    isSearchLoading,
    addShow,
    type SearchResult,
  } from "../stores/shows.svelte";
  import { config } from "../config";

  let searchInput: HTMLInputElement | undefined = $state();
  let debounceTimer: ReturnType<typeof setTimeout> | null = $state(null);

  // Auto-focus when modal opens
  $effect(() => {
    if (isSearchModalOpen() && searchInput) {
      // Small delay to ensure element is in DOM
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
    setSearchQuery(value);

    // Clear existing timer
    if (debounceTimer) {
      clearTimeout(debounceTimer);
    }

    // Debounce search - auto-search after debounce delay
    if (value.trim()) {
      debounceTimer = setTimeout(() => {
        searchShows(value);
      }, config.ui.searchDebounceMs);
    } else {
      // Clear results immediately if query is empty
      searchShows("");
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      closeSearchModal();
    } else if (event.key === "Enter") {
      const query = getSearchQuery();
      if (query.trim()) {
        searchShows(query);
      }
    }
  }

  function handleSubmit(event: Event) {
    event.preventDefault();
    const query = getSearchQuery();
    if (query.trim()) {
      searchShows(query);
    }
  }

  async function handleAddShow(show: SearchResult) {
    await addShow(show);
  }
</script>

<svelte:window on:keydown={handleKeyDown} />

{#if isSearchModalOpen()}
  <!-- Backdrop -->
  <button
    transition:fade={{ duration: 150 }}
    class="fixed inset-0 bg-black/60 z-50"
    onclick={closeSearchModal}
    aria-label="Close modal"
  ></button>

  {@const resultCount = getSearchResults().length}
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
        placeholder="Search for a TV show... (press Enter)"
        class="flex-1 bg-transparent text-text text-lg outline-none placeholder:text-text-muted"
        value={getSearchQuery()}
        oninput={handleInput}
        bind:this={searchInput}
        aria-label="Search for TV shows"
        aria-describedby="search-help"
        autocomplete="off"
      />
      <span id="search-help" class="sr-only">Type a show name and press Enter to search</span>
      <button
        type="button"
        onclick={closeSearchModal}
        class="p-1.5 rounded-lg hover:bg-surface-hover transition-colors"
        aria-label="Close"
      >
        <X class="w-5 h-5 text-text-muted" />
      </button>
    </form>

    <!-- Results -->
    <div class="flex-1 overflow-auto min-h-0">
      {#if isSearchLoading()}
        <div class="flex items-center justify-center py-12">
          <Loader2 class="w-8 h-8 text-accent animate-spin" />
          <span class="ml-3 text-sm text-text-muted">Searching...</span>
        </div>
      {:else if getSearchResults().length === 0}
        {#if getSearchQuery().trim()}
          <div class="py-12 text-center">
            <p class="text-text-muted mb-2">No results found for "{getSearchQuery()}"</p>
            <p class="text-xs text-text-muted">Try a different search term</p>
          </div>
        {:else}
          <div class="py-12 text-center text-text-muted">
            <p class="mb-2">Type a show name to search</p>
            <p class="text-xs">Search will start automatically as you type</p>
          </div>
        {/if}
      {:else}
        <div class="px-4 py-2 border-b border-border">
          <p class="text-xs text-text-muted">
            Found {getSearchResults().length} result{getSearchResults().length !== 1 ? 's' : ''}
          </p>
        </div>
        <ul class="divide-y divide-border" role="listbox" aria-label="Search results">
          {#each getSearchResults() as show}
            <li class="flex items-start gap-4 p-4 hover:bg-surface-hover transition-colors" role="option" tabindex="0">
              {#if show.image_url}
                <img
                  src={show.image_url}
                  alt={`Poster for ${show.name || "TV show"}`}
                  class="w-16 h-24 rounded object-cover flex-shrink-0"
                />
              {:else}
                <div class="w-16 h-24 rounded bg-border flex items-center justify-center flex-shrink-0">
                  <span class="text-xs text-text-muted">No Image</span>
                </div>
              {/if}
              <div class="flex-1 min-w-0">
                <h3 class="font-semibold text-text truncate">
                  {show.name || "Unknown"}
                </h3>
                {#if show.year || show.network}
                  <p class="text-sm text-text-muted mt-0.5">
                    {[show.year, show.network].filter(Boolean).join(" - ")}
                  </p>
                {/if}
                {#if show.overview}
                  <p class="text-sm text-text-muted mt-2 line-clamp-2">
                    {show.overview}
                  </p>
                {/if}
              </div>
              <button
                onclick={() => handleAddShow(show)}
                class="flex-shrink-0 p-2 bg-accent hover:bg-accent-hover text-white rounded-lg transition-colors"
                aria-label="Add show"
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
        onclick={closeSearchModal}
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
