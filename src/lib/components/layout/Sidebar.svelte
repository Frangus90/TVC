<script lang="ts">
  import { Plus, Tv, Trash2 } from "lucide-svelte";
  import { onMount } from "svelte";
  import {
    getTrackedShows,
    loadTrackedShows,
    openSearchModal,
    removeShow,
  } from "../../stores/shows.svelte";

  onMount(() => {
    loadTrackedShows();
  });

  async function handleRemoveShow(event: MouseEvent, showId: number) {
    event.stopPropagation();
    await removeShow(showId);
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
          <li>
            <div
              class="group w-full flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-surface-hover transition-colors"
            >
              {#if show.poster_url}
                <img src={show.poster_url} alt="" class="w-8 h-12 rounded object-cover" />
              {:else}
                <div class="w-8 h-12 rounded bg-border flex items-center justify-center">
                  <Tv class="w-4 h-4 text-text-muted" />
                </div>
              {/if}
              <span class="flex-1 text-sm truncate">{show.name}</span>
              <button
                onclick={(e) => handleRemoveShow(e, show.id)}
                class="opacity-0 group-hover:opacity-100 p-1.5 rounded hover:bg-red-500/20 text-red-400 transition-all"
                aria-label="Remove show"
              >
                <Trash2 class="w-4 h-4" />
              </button>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <div class="p-3 border-t border-border">
    <button
      onclick={openSearchModal}
      class="w-full flex items-center justify-center gap-2 px-4 py-2.5 bg-accent hover:bg-accent-hover text-white rounded-lg transition-colors font-medium"
    >
      <Plus class="w-4 h-4" />
      Add Show
    </button>
    <p class="text-xs text-text-muted text-center mt-3">v0.4.3</p>
  </div>
</aside>
