<script lang="ts">
  import { Tv, Film, Trash2 } from "lucide-svelte";
  import { onMount } from "svelte";
  import { getTrackedShows } from "../../stores/shows.svelte";
  import { getTrackedMovies, openMovieDetail, updateMovieRating } from "../../stores/movies.svelte";
  import { openShowDetail, updateShowRating } from "../../stores/showDetail.svelte";
  import { registerDropZone, startDrag, type DragData, getIsDragging } from "../../stores/dragDrop.svelte";

  type TierSubTab = "shows" | "movies";
  let subTab = $state<TierSubTab>("shows");

  // All possible tiers (5 down to 0.5)
  const ALL_TIERS = [5, 4.5, 4, 3.5, 3, 2.5, 2, 1.5, 1, 0.5];

  // Drag state for visual feedback
  let dragOverTier = $state<number | "unrate" | null>(null);

  // Track if currently dragging (for visual hints)
  const isDragging = $derived(getIsDragging());

  // Element references for drop zones
  let tierRefs = $state<Record<number, HTMLElement | null>>({});
  let unrateRef = $state<HTMLElement | null>(null);

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

  // Get items for a specific tier
  function getShowsForTier(tier: number) {
    return rankedShows.filter(s => s.rating === tier);
  }

  function getMoviesForTier(tier: number) {
    return rankedMovies.filter(m => m.rating === tier);
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

  // Handle drop for rating
  async function handleTierDrop(data: DragData, rating: number | null) {
    if (data.type === "show" && subTab === "shows") {
      await updateShowRating(data.id, rating);
    } else if (data.type === "movie" && subTab === "movies") {
      await updateMovieRating(data.id, rating);
    }
  }

  // Register drop zones
  onMount(() => {
    const cleanups: (() => void)[] = [];

    // Register tier drop zones
    for (const tier of ALL_TIERS) {
      const element = tierRefs[tier];
      if (element) {
        const cleanup = registerDropZone(`tier-${tier}`, element, {
          onDrop: (data) => handleTierDrop(data, tier),
          onDragEnter: () => { dragOverTier = tier; },
          onDragLeave: () => { dragOverTier = null; }
        });
        cleanups.push(cleanup);
      }
    }

    // Register unrate zone
    if (unrateRef) {
      const cleanup = registerDropZone("unrate", unrateRef, {
        onDrop: (data) => handleTierDrop(data, null),
        onDragEnter: () => { dragOverTier = "unrate"; },
        onDragLeave: () => { dragOverTier = null; }
      });
      cleanups.push(cleanup);
    }

    return () => {
      cleanups.forEach(fn => fn());
    };
  });
</script>

<div class="h-full flex flex-col">
  <!-- Header with sub-tabs and stats -->
  <div class="flex items-center justify-between mb-4">
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

  <!-- Drag hint -->
  <p class="text-xs text-text-muted mb-3">
    {#if isDragging}
      <span class="text-accent font-medium">Release to drop on a tier</span>
    {:else}
      Drag {subTab === "shows" ? "shows" : "movies"} from the sidebar and drop them here to rate.
    {/if}
  </p>

  <!-- Tier List - Always show all tiers -->
  <div class="flex-1 overflow-auto space-y-2">
    {#each ALL_TIERS as tier}
      {@const showItems = subTab === "shows" ? getShowsForTier(tier) : []}
      {@const movieItems = subTab === "movies" ? getMoviesForTier(tier) : []}
      {@const items = subTab === "shows" ? showItems : movieItems}
      {@const isEmpty = items.length === 0}

      <div
        bind:this={tierRefs[tier]}
        data-drop-zone="tier-{tier}"
        role="listbox"
        aria-label="{tier} star tier"
        class="flex items-stretch rounded-lg overflow-hidden transition-all
          {dragOverTier === tier ? 'ring-2 ring-accent bg-accent/10' : 'bg-surface'}
          {isEmpty ? 'border-2 border-dashed border-border' : ''}"
      >
        <!-- Tier label -->
        <div class="w-20 flex-shrink-0 flex flex-col items-center justify-center py-3 px-2 bg-surface-hover border-r border-border">
          <span class="text-yellow-400 text-lg font-bold">{renderStars(tier)}</span>
          <span class="text-xs text-text-muted mt-1">{tier}</span>
        </div>

        <!-- Posters row -->
        <div class="flex-1 flex flex-wrap items-center gap-2 p-3 min-h-[80px]">
          {#if isEmpty}
            <span class="text-text-muted text-sm">Drop here for {tier}★</span>
          {:else if subTab === "shows"}
            {#each showItems as show}
              <button
                type="button"
                onclick={() => openShowDetail(show.id)}
                onmousedown={(e) => startDrag({ type: "show", id: show.id }, e.clientX, e.clientY)}
                class="group relative flex-shrink-0 transition-transform hover:scale-105 hover:z-10 cursor-grab active:cursor-grabbing"
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
          {:else}
            {#each movieItems as movie}
              <button
                type="button"
                onclick={() => openMovieDetail(movie.id)}
                onmousedown={(e) => startDrag({ type: "movie", id: movie.id }, e.clientX, e.clientY)}
                class="group relative flex-shrink-0 transition-transform hover:scale-105 hover:z-10 cursor-grab active:cursor-grabbing"
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
          {/if}
        </div>
      </div>
    {/each}

    <!-- Unrate Zone -->
    <div
      bind:this={unrateRef}
      data-drop-zone="unrate"
      role="listbox"
      aria-label="Remove rating"
      class="flex items-center gap-3 p-4 rounded-lg border-2 border-dashed transition-all
        {dragOverTier === 'unrate' ? 'border-red-500 bg-red-500/10 ring-2 ring-red-500' : 'border-border'}"
    >
      <Trash2 class="w-5 h-5 {dragOverTier === 'unrate' ? 'text-red-500' : 'text-text-muted'}" />
      <span class="{dragOverTier === 'unrate' ? 'text-red-500' : 'text-text-muted'} text-sm">
        Drop here to remove rating
      </span>
    </div>
  </div>
</div>
