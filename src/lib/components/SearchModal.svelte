<script lang="ts">
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

  let searchInput: HTMLInputElement | undefined = $state();

  // Auto-focus when modal opens
  $effect(() => {
    if (isSearchModalOpen() && searchInput) {
      // Small delay to ensure element is in DOM
      setTimeout(() => searchInput?.focus(), 10);
    }
  });

  function handleInput(event: Event) {
    const target = event.target as HTMLInputElement;
    setSearchQuery(target.value);
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
    class="fixed inset-0 bg-black/60 z-50"
    onclick={closeSearchModal}
    aria-label="Close modal"
  ></button>

  <!-- Modal -->
  <div
    class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-full max-w-2xl z-50 bg-surface rounded-xl border border-border shadow-2xl"
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
      />
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
    <div class="max-h-96 overflow-auto">
      {#if isSearchLoading()}
        <div class="flex items-center justify-center py-12">
          <Loader2 class="w-8 h-8 text-accent animate-spin" />
        </div>
      {:else if getSearchResults().length === 0}
        {#if getSearchQuery().trim()}
          <div class="py-12 text-center text-text-muted">
            Press Enter to search for "{getSearchQuery()}"
          </div>
        {:else}
          <div class="py-12 text-center text-text-muted">
            Type a show name and press Enter to search
          </div>
        {/if}
      {:else}
        <ul class="divide-y divide-border">
          {#each getSearchResults() as show}
            <li class="flex items-start gap-4 p-4 hover:bg-surface-hover transition-colors">
              {#if show.image_url}
                <img
                  src={show.image_url}
                  alt=""
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
