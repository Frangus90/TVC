<script lang="ts">
  import { Plus, Tv, Trash2, RefreshCw, Check } from "lucide-svelte";
  import { onMount } from "svelte";
  import {
    getTrackedShows,
    loadTrackedShows,
    openSearchModal,
    removeShow,
  } from "../../stores/shows.svelte";
  import { openShowDetail } from "../../stores/showDetail.svelte";
  import {
    triggerUpdateCheck,
    isCheckingForUpdates,
  } from "../../stores/updates.svelte";

  let selectedShows = $state<Set<number>>(new Set());
  let bulkMode = $state(false);

  onMount(() => {
    loadTrackedShows();
  });

  async function handleRemoveShow(event: MouseEvent, showId: number) {
    event.stopPropagation();
    await removeShow(showId);
  }

  function toggleShowSelection(showId: number) {
    const newSelected = new Set(selectedShows);
    if (newSelected.has(showId)) {
      newSelected.delete(showId);
    } else {
      newSelected.add(showId);
    }
    selectedShows = newSelected;
  }

  async function handleBulkRemove() {
    if (selectedShows.size === 0) return;

    if (confirm(`Are you sure you want to remove ${selectedShows.size} show${selectedShows.size > 1 ? 's' : ''}?`)) {
      for (const showId of selectedShows) {
        await removeShow(showId);
      }
      selectedShows = new Set();
      bulkMode = false;
    }
  }
</script>

<aside class="w-64 bg-surface border-r border-border flex flex-col">
  <div class="p-4 border-b border-border">
    <div class="flex items-center gap-2 text-xl font-semibold">
      <Tv class="w-6 h-6 text-accent" />
      <span>TVC</span>
    </div>
  </div>

  <div class="flex-1 overflow-auto p-3">
    <div class="flex items-center justify-between mb-3">
      <span class="text-sm font-medium text-text-muted uppercase tracking-wide">Tracked Shows</span>
      <div class="flex items-center gap-2">
        {#if bulkMode}
          <button
            type="button"
            onclick={() => { bulkMode = false; selectedShows = new Set(); }}
            class="text-xs text-text-muted hover:text-text"
          >
            Cancel
          </button>
          {#if selectedShows.size > 0}
            <button
              type="button"
              onclick={handleBulkRemove}
              class="text-xs text-red-400 hover:text-red-300"
            >
              Remove ({selectedShows.size})
            </button>
          {/if}
        {:else}
          <button
            type="button"
            onclick={() => bulkMode = true}
            class="text-xs text-accent hover:underline"
          >
            Select
          </button>
        {/if}
      </div>
    </div>

    {#if getTrackedShows().length === 0}
      <p class="text-sm text-text-muted py-4 text-center">
        No shows tracked yet.
        <br />
        Click "Add Show" to get started.
      </p>
    {:else}
      <ul class="space-y-1">
        {#each getTrackedShows() as show}
          {@const isSelected = selectedShows.has(show.id)}
          <li>
            {#if bulkMode}
              <button
                type="button"
                onclick={() => toggleShowSelection(show.id)}
                class="group w-full flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-surface-hover transition-colors text-left"
              >
                <span
                  class="w-5 h-5 rounded border flex-shrink-0 flex items-center justify-center
                    {isSelected ? 'bg-accent border-accent' : 'border-border'}"
                >
                  {#if isSelected}
                    <Check class="w-3 h-3 text-white" />
                  {/if}
                </span>
                {#if show.poster_url}
                  <img src={show.poster_url} alt="" class="w-8 h-12 rounded object-cover" loading="lazy" decoding="async" />
                {:else}
                  <div class="w-8 h-12 rounded bg-border flex items-center justify-center">
                    <Tv class="w-4 h-4 text-text-muted" />
                  </div>
                {/if}
                <span class="flex-1 text-sm truncate">{show.name}</span>
              </button>
            {:else}
              <div class="group w-full flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-surface-hover transition-colors">
                <button
                  type="button"
                  class="flex-1 flex items-center gap-3 text-left"
                  onclick={() => openShowDetail(show.id)}
                >
                  {#if show.poster_url}
                    <img src={show.poster_url} alt="" class="w-8 h-12 rounded object-cover" loading="lazy" decoding="async" />
                  {:else}
                    <div class="w-8 h-12 rounded bg-border flex items-center justify-center">
                      <Tv class="w-4 h-4 text-text-muted" />
                    </div>
                  {/if}
                  <div class="flex-1 flex items-center gap-2 min-w-0">
                    {#if show.color}
                      <div
                        class="w-3 h-3 rounded-full flex-shrink-0"
                        style="background-color: {show.color};"
                        title="Show color"
                      ></div>
                    {/if}
                    <span class="text-sm truncate">{show.name}</span>
                  </div>
                </button>
                <button
                  type="button"
                  onclick={(e) => { e.stopPropagation(); handleRemoveShow(e, show.id); }}
                  class="opacity-0 group-hover:opacity-100 p-1.5 rounded hover:bg-red-500/20 text-red-400 transition-all"
                  aria-label="Remove show"
                >
                  <Trash2 class="w-4 h-4" />
                </button>
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <div class="p-3 border-t border-border">
    <button
      type="button"
      onclick={openSearchModal}
      class="w-full flex items-center justify-center gap-2 px-4 py-2.5 bg-accent hover:bg-accent-hover text-white rounded-lg transition-colors font-medium"
    >
      <Plus class="w-4 h-4" />
      Add Show
    </button>
    <button
      type="button"
      onclick={triggerUpdateCheck}
      disabled={isCheckingForUpdates()}
      class="w-full flex items-center justify-center gap-2 mt-2 px-3 py-1.5 text-xs text-text-muted hover:text-text hover:bg-surface-hover rounded transition-colors disabled:opacity-50"
    >
      <RefreshCw class="w-3 h-3 {isCheckingForUpdates() ? 'animate-spin' : ''}" />
      {isCheckingForUpdates() ? "Checking..." : "Check for Updates"}
    </button>
    <p class="text-xs text-text-muted text-center mt-2">v0.5.4</p>
  </div>
</aside>
