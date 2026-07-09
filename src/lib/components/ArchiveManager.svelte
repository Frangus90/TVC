<script lang="ts">
  import { onMount } from "svelte";
  import { Archive, RotateCcw, Tv, Film } from "lucide-svelte";
  import {
    getArchivedShows,
    loadArchivedShows,
    unarchiveShow,
    isArchivedShowsLoading,
  } from "../stores/shows.svelte";
  import {
    getArchivedMovies,
    loadArchivedMovies,
    unarchiveMovie,
    isArchivedMoviesLoading,
  } from "../stores/movies.svelte";
  import { getThemeSettings } from "../stores/theme.svelte";
  import EmptyState from "./common/EmptyState.svelte";
  import LoadingSpinner from "./common/LoadingSpinner.svelte";

  interface ArchiveItem {
    id: number;
    name: string;
    poster_url: string | null;
    type: "show" | "movie";
    color?: string | null;
  }

  const theme = getThemeSettings();

  onMount(() => {
    loadArchivedShows();
    loadArchivedMovies();
  });

  const items = $derived<ArchiveItem[]>(
    [
      ...getArchivedShows().map((s) => ({
        id: s.id,
        name: s.name,
        poster_url: s.poster_url,
        type: "show" as const,
        color: s.color,
      })),
      ...getArchivedMovies().map((m) => ({
        id: m.id,
        name: m.title,
        poster_url: m.poster_url,
        type: "movie" as const,
        color: m.color,
      })),
    ].sort((a, b) => a.name.localeCompare(b.name)),
  );

  const loading = $derived(isArchivedShowsLoading() || isArchivedMoviesLoading());

  function unarchive(item: ArchiveItem) {
    if (item.type === "show") {
      unarchiveShow(item.id);
    } else {
      unarchiveMovie(item.id);
    }
  }
</script>

{#if loading}
  <div class="flex items-center justify-center py-12">
    <LoadingSpinner />
  </div>
{:else if items.length === 0}
  <EmptyState
    icon={Archive}
    title="No archived items"
    message="Archive shows or movies you're done with to keep your list clean."
  />
{:else}
  <p class="text-sm text-text-muted mb-3">
    {items.length} archived item{items.length === 1 ? "" : "s"}. Restore any to move it
    back to your active list.
  </p>
  <ul class="space-y-1">
    {#each items as item (item.type + item.id)}
      <li
        class="group flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-surface-hover transition-colors"
      >
        {#if !theme.hidePosters}
          {#if item.poster_url}
            <img
              src={item.poster_url}
              alt=""
              class="w-8 h-12 rounded object-cover flex-shrink-0"
              loading="lazy"
              decoding="async"
            />
          {:else}
            <div class="w-8 h-12 rounded bg-border flex items-center justify-center flex-shrink-0">
              {#if item.type === "show"}
                <Tv class="w-4 h-4 text-text-muted" />
              {:else}
                <Film class="w-4 h-4 text-text-muted" />
              {/if}
            </div>
          {/if}
        {/if}
        <div class="flex-1 min-w-0 flex items-center gap-2">
          {#if item.color}
            <div
              class="w-3 h-3 rounded-full flex-shrink-0"
              style="background-color: {item.color};"
            ></div>
          {/if}
          <span class="text-sm truncate">{item.name}</span>
          <span class="text-xs text-text-muted flex-shrink-0">
            · {item.type === "show" ? "TV Show" : "Movie"}
          </span>
        </div>
        <button
          type="button"
          onclick={() => unarchive(item)}
          class="opacity-0 group-hover:opacity-100 flex items-center gap-1 px-2 py-1 rounded hover:bg-accent/20 text-accent text-sm transition-all flex-shrink-0"
          title="Restore from archive"
        >
          <RotateCcw class="w-4 h-4" /> Restore
        </button>
      </li>
    {/each}
  </ul>
{/if}
