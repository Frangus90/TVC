<script lang="ts">
  import { Tv, Film, Trophy } from "lucide-svelte";
  import { getTrackedShows } from "../../stores/shows.svelte";
  import { getTrackedMovies, openMovieDetail } from "../../stores/movies.svelte";
  import { openShowDetail } from "../../stores/showDetail.svelte";

  type TierSubTab = "shows" | "movies";
  let subTab = $state<TierSubTab>("shows");

  // Ranked items filtered and sorted
  const rankedShows = $derived.by(() => {
    return getTrackedShows()
      .filter(s => s.rating !== null)
      .sort((a, b) => (b.rating ?? 0) - (a.rating ?? 0));
  });

  const rankedMovies = $derived.by(() => {
    return getTrackedMovies()
      .filter(m => m.rating !== null)
      .sort((a, b) => (b.rating ?? 0) - (a.rating ?? 0));
  });

  // Group items by rating tier
  function groupByRating<T extends { rating: number | null }>(items: T[]): [number, T[]][] {
    const groups = new Map<number, T[]>();
    for (const item of items) {
      if (item.rating === null) continue;
      if (!groups.has(item.rating)) groups.set(item.rating, []);
      groups.get(item.rating)!.push(item);
    }
    return Array.from(groups.entries()).sort((a, b) => b[0] - a[0]);
  }

  // Render star display for tier label
  function renderStars(rating: number): string {
    const fullStars = Math.floor(rating);
    const hasHalf = rating % 1 >= 0.5;
    return "★".repeat(fullStars) + (hasHalf ? "½" : "");
  }

  // Calculate stats
  const stats = $derived.by(() => {
    const items = subTab === "shows" ? rankedShows : rankedMovies;
    if (items.length === 0) return { avg: 0, count: 0 };
    const avg = items.reduce((sum, item) => sum + (item.rating ?? 0), 0) / items.length;
    return { avg, count: items.length };
  });

  // Get grouped items based on sub-tab - use separate deriveds for type safety
  const groupedShows = $derived(groupByRating(rankedShows));
  const groupedMovies = $derived(groupByRating(rankedMovies));
  const currentCount = $derived(subTab === "shows" ? rankedShows.length : rankedMovies.length);
</script>

<div class="h-full flex flex-col">
  <!-- Header with sub-tabs and stats -->
  <div class="flex items-center justify-between mb-6">
    <!-- Sub-tabs -->
    <div class="flex gap-1 p-1 bg-surface rounded-lg">
      <button
        type="button"
        onclick={() => subTab = "shows"}
        class="flex items-center gap-2 px-4 py-2 text-sm font-medium rounded-md transition-colors
          {subTab === 'shows' ? 'bg-background text-accent' : 'text-text-muted hover:text-text'}"
      >
        <Tv class="w-4 h-4" />
        Shows
      </button>
      <button
        type="button"
        onclick={() => subTab = "movies"}
        class="flex items-center gap-2 px-4 py-2 text-sm font-medium rounded-md transition-colors
          {subTab === 'movies' ? 'bg-background text-accent' : 'text-text-muted hover:text-text'}"
      >
        <Film class="w-4 h-4" />
        Movies
      </button>
    </div>

    <!-- Stats -->
    {#if stats.count > 0}
      <div class="flex items-center gap-4 text-sm">
        <span class="text-text-muted">
          Average: <span class="text-yellow-400 font-medium">{stats.avg.toFixed(1)}★</span>
        </span>
        <span class="text-text-muted">
          Rated: <span class="text-text font-medium">{stats.count}</span>
        </span>
      </div>
    {/if}
  </div>

  <!-- Tier List -->
  {#if currentCount === 0}
    <div class="flex-1 flex flex-col items-center justify-center text-center">
      <Trophy class="w-16 h-16 text-text-muted mb-4" />
      <h3 class="text-lg font-medium text-text mb-2">No rated {subTab === 'shows' ? 'shows' : 'movies'}</h3>
      <p class="text-text-muted max-w-md">
        Rate {subTab === 'shows' ? 'shows' : 'movies'} from their detail page to see them in the tier list.
      </p>
    </div>
  {:else if subTab === "shows"}
    <div class="flex-1 overflow-auto space-y-2">
      {#each groupedShows as [rating, items]}
        <div class="flex items-stretch bg-surface rounded-lg overflow-hidden">
          <div class="w-20 flex-shrink-0 flex flex-col items-center justify-center py-3 px-2 bg-surface-hover border-r border-border">
            <span class="text-yellow-400 text-lg font-bold">{renderStars(rating)}</span>
            <span class="text-xs text-text-muted mt-1">{rating}</span>
          </div>
          <div class="flex-1 flex flex-wrap items-center gap-2 p-3 min-h-[100px]">
            {#each items as show}
              <button
                type="button"
                onclick={() => openShowDetail(show.id)}
                class="group relative flex-shrink-0 transition-transform hover:scale-105 hover:z-10"
                title={show.name}
              >
                {#if show.poster_url}
                  <img
                    src={show.poster_url}
                    alt=""
                    class="w-16 h-24 rounded object-cover shadow-lg group-hover:ring-2 group-hover:ring-accent"
                    loading="lazy"
                    decoding="async"
                  />
                {:else}
                  <div class="w-16 h-24 rounded bg-border flex items-center justify-center shadow-lg group-hover:ring-2 group-hover:ring-accent">
                    <Tv class="w-6 h-6 text-text-muted" />
                  </div>
                {/if}
              </button>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <div class="flex-1 overflow-auto space-y-2">
      {#each groupedMovies as [rating, items]}
        <div class="flex items-stretch bg-surface rounded-lg overflow-hidden">
          <div class="w-20 flex-shrink-0 flex flex-col items-center justify-center py-3 px-2 bg-surface-hover border-r border-border">
            <span class="text-yellow-400 text-lg font-bold">{renderStars(rating)}</span>
            <span class="text-xs text-text-muted mt-1">{rating}</span>
          </div>
          <div class="flex-1 flex flex-wrap items-center gap-2 p-3 min-h-[100px]">
            {#each items as movie}
              <button
                type="button"
                onclick={() => openMovieDetail(movie.id)}
                class="group relative flex-shrink-0 transition-transform hover:scale-105 hover:z-10"
                title={movie.title}
              >
                {#if movie.poster_url}
                  <img
                    src={movie.poster_url}
                    alt=""
                    class="w-16 h-24 rounded object-cover shadow-lg group-hover:ring-2 group-hover:ring-accent"
                    loading="lazy"
                    decoding="async"
                  />
                {:else}
                  <div class="w-16 h-24 rounded bg-border flex items-center justify-center shadow-lg group-hover:ring-2 group-hover:ring-accent">
                    <Film class="w-6 h-6 text-text-muted" />
                  </div>
                {/if}
              </button>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
