<script lang="ts">
  import { Tv, Film, Trash2, MoreVertical } from "lucide-svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { getTrackedShows, loadTrackedShows } from "../../stores/shows.svelte";
  import { getTrackedMovies, loadTrackedMovies, openMovieDetail, updateMovieRating } from "../../stores/movies.svelte";
  import { openShowDetail, updateShowRating } from "../../stores/showDetail.svelte";
  import { registerDropZone, startDrag, type DragData, getIsDragging, consumeWasDragging } from "../../stores/dragDrop.svelte";
  import { setSidebarTab } from "../../stores/sidebar.svelte";

  type TierSubTab = "shows" | "movies";
  let subTab = $state<TierSubTab>("shows");

  // Sync sidebar when sub-tab changes
  function switchSubTab(tab: TierSubTab) {
    subTab = tab;
    setSidebarTab(tab);
  }

  // All possible tiers (5 down to 0.5)
  const ALL_TIERS = [5, 4.5, 4, 3.5, 3, 2.5, 2, 1.5, 1, 0.5];

  // Tier labels for descriptive names
  const TIER_LABELS: Record<number, string> = {
    5: "Masterpiece",
    4.5: "Excellent",
    4: "Great",
    3.5: "Good",
    3: "Solid",
    2.5: "Average",
    2: "Below Average",
    1.5: "Poor",
    1: "Bad",
    0.5: "Terrible"
  };

  // Drag state for visual feedback
  let dragOverTier = $state<number | "unrate" | null>(null);

  // Track if currently dragging (for visual hints)
  const isDragging = $derived(getIsDragging());

  // Element references for drop zones
  let tierRefs = $state<Record<number, HTMLElement | null>>({});
  let unrateRef = $state<HTMLElement | null>(null);

  // Rating menu state
  let ratingMenuOpen = $state<{ type: "show" | "movie"; id: number } | null>(null);

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

  // Get items for a specific tier, sorted by rank_order
  function getShowsForTier(tier: number) {
    return rankedShows
      .filter(s => s.rating === tier)
      .sort((a, b) => (a.rank_order ?? 999999) - (b.rank_order ?? 999999) || a.id - b.id);
  }

  function getMoviesForTier(tier: number) {
    return rankedMovies
      .filter(m => m.rating === tier)
      .sort((a, b) => (a.rank_order ?? 999999) - (b.rank_order ?? 999999) || a.id - b.id);
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

  // Handle drop for rating or reordering
  async function handleTierDrop(data: DragData, rating: number | null, dropX?: number, _dropY?: number) {
    if (rating === null) {
      // Unrate zone
      if (data.type === "show" && subTab === "shows") {
        await updateShowRating(data.id, null);
      } else if (data.type === "movie" && subTab === "movies") {
        await updateMovieRating(data.id, null);
      }
      return;
    }

    // Check if this is a within-tier reorder (same rating)
    const isShow = data.type === "show" && subTab === "shows";
    const isMovie = data.type === "movie" && subTab === "movies";

    if (!isShow && !isMovie) return;

    const currentItem = isShow
      ? getTrackedShows().find(s => s.id === data.id)
      : getTrackedMovies().find(m => m.id === data.id);

    const isWithinTier = currentItem?.rating === rating;

    if (isWithinTier && dropX !== undefined) {
      // Within-tier reorder: calculate insertion position
      const tierItems = isShow ? getShowsForTier(rating) : getMoviesForTier(rating);
      if (tierItems.length <= 1) return; // Nothing to reorder

      const tierElement = tierRefs[rating];
      if (!tierElement) return;

      // Find all poster containers in this tier
      const posterContainers = tierElement.querySelectorAll('.rating-menu-container');
      let insertIndex = tierItems.length; // Default: append to end

      for (let i = 0; i < posterContainers.length; i++) {
        const rect = posterContainers[i].getBoundingClientRect();
        const midX = rect.left + rect.width / 2;
        if (dropX < midX) {
          insertIndex = i;
          break;
        }
      }

      // Assign sequential rank_order values
      const reordered = tierItems.filter(item => item.id !== data.id);
      reordered.splice(insertIndex > reordered.length ? reordered.length : insertIndex, 0,
        tierItems.find(item => item.id === data.id)!
      );

      // Update rank_order for all items in this tier
      const command = isShow ? "reorder_show_in_tier" : "reorder_movie_in_tier";
      for (let i = 0; i < reordered.length; i++) {
        await invoke(command, { id: reordered[i].id, newRankOrder: i });
      }

      // Reload data to reflect new order
      if (isShow) await loadTrackedShows();
      else await loadTrackedMovies();
    } else {
      // Cross-tier move: change rating
      if (isShow) {
        await updateShowRating(data.id, rating);
      } else {
        await updateMovieRating(data.id, rating);
      }
    }
  }

  // Handle quick rating change from menu
  async function handleQuickRate(type: "show" | "movie", id: number, rating: number | null) {
    ratingMenuOpen = null;
    if (type === "show") {
      await updateShowRating(id, rating);
    } else {
      await updateMovieRating(id, rating);
    }
  }

  // Toggle rating menu
  function toggleRatingMenu(e: MouseEvent, type: "show" | "movie", id: number) {
    e.stopPropagation();
    e.preventDefault();
    if (ratingMenuOpen?.type === type && ratingMenuOpen?.id === id) {
      ratingMenuOpen = null;
    } else {
      ratingMenuOpen = { type, id };
    }
  }

  // Close menu when clicking outside
  function handleClickOutside(e: MouseEvent) {
    if (ratingMenuOpen && !(e.target as HTMLElement).closest('.rating-menu-container')) {
      ratingMenuOpen = null;
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
          onDrop: (data, dropX, dropY) => handleTierDrop(data, tier, dropX, dropY),
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

    // Global click handler for closing menu
    document.addEventListener('click', handleClickOutside);

    return () => {
      cleanups.forEach(fn => fn());
      document.removeEventListener('click', handleClickOutside);
    };
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="h-full flex flex-col" onclick={handleClickOutside} onkeydown={(e) => { if (e.key === 'Escape') ratingMenuOpen = null; }}>
  <!-- Header with sub-tabs and stats -->
  <div class="flex items-center justify-between mb-4 gap-4">
    <!-- Sub-tabs -->
    <div class="flex gap-1 p-1 bg-surface rounded-lg">
      <button
        type="button"
        onclick={() => switchSubTab("shows")}
        class="flex items-center gap-2 px-4 py-2 text-sm font-medium rounded-md transition-colors
          {subTab === 'shows' ? 'bg-background text-accent' : 'text-text-muted hover:text-text'}"
      >
        <Tv class="w-4 h-4" />
        Shows
      </button>
      <button
        type="button"
        onclick={() => switchSubTab("movies")}
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

  <!-- Drag hint or empty state -->
  {#if stats.count === 0}
    <div class="flex-1 flex flex-col items-center justify-center text-center py-12">
      <div class="w-16 h-16 rounded-full bg-surface flex items-center justify-center mb-4">
        {#if subTab === "shows"}
          <Tv class="w-8 h-8 text-text-muted" />
        {:else}
          <Film class="w-8 h-8 text-text-muted" />
        {/if}
      </div>
      <h3 class="text-lg font-medium text-text mb-2">No rated {subTab} yet</h3>
      <p class="text-text-muted text-sm max-w-md">
        Drag {subTab} from the sidebar and drop them on a tier to rate them,
        or open a {subTab === "shows" ? "show" : "movie"} and use the star rating.
      </p>
    </div>
  {:else}
    <p class="text-xs text-text-muted mb-3">
      {#if isDragging}
        <span class="text-accent font-medium">Release to drop on a tier</span>
      {:else}
        Drag {subTab === "shows" ? "shows" : "movies"} from the sidebar to rate, or click the menu on a poster to change its rating.
      {/if}
    </p>

    <!-- Tier List -->
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
          aria-label="{tier} star tier - {TIER_LABELS[tier]}"
          class="flex items-stretch rounded-lg transition-all
            {dragOverTier === tier ? 'ring-2 ring-accent bg-accent/10' : 'bg-surface'}
            {isEmpty ? 'border-2 border-dashed border-border' : ''}"
        >
          <!-- Tier label with description -->
          <div class="w-28 flex-shrink-0 flex flex-col items-center justify-center py-3 px-2 bg-surface-hover border-r border-border">
            <span class="text-yellow-400 text-base font-bold">{renderStars(tier)}</span>
            <span class="text-[10px] text-text-muted mt-0.5">{tier}</span>
            <span class="text-[10px] text-text-muted font-medium mt-1">{TIER_LABELS[tier]}</span>
          </div>

          <!-- Posters row with names -->
          <div class="flex-1 flex flex-wrap items-start gap-3 p-3 min-h-[100px]">
            {#if isEmpty}
              <span class="text-text-muted text-sm self-center">Drop here for {tier}★</span>
            {:else if subTab === "shows"}
              {#each showItems as show}
                <div class="group flex flex-col items-center w-16 relative rating-menu-container">
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div
                    onmousedown={(e) => { if (!(e.target as HTMLElement).closest('.rating-menu-btn')) startDrag({ type: "show", id: show.id }, e.clientX, e.clientY); }}
                    onclick={() => { if (!consumeWasDragging()) openShowDetail(show.id); }}
                    onkeydown={(e) => { if (e.key === 'Enter') openShowDetail(show.id); }}
                    title={show.name}
                    role="button"
                    tabindex="0"
                    class="flex flex-col items-center w-full transition-transform hover:scale-105 hover:z-10 cursor-grab active:cursor-grabbing"
                  >
                    {#if show.poster_url}
                      <img
                        src={show.poster_url}
                        alt={show.name}
                        class="w-16 h-24 rounded object-cover shadow-lg group-hover:ring-2 group-hover:ring-accent pointer-events-none"
                        loading="lazy"
                        decoding="async"
                      />
                    {:else}
                      <div class="w-16 h-24 rounded bg-border flex items-center justify-center shadow-lg group-hover:ring-2 group-hover:ring-accent">
                        <Tv class="w-6 h-6 text-text-muted" />
                      </div>
                    {/if}
                    <span class="text-xs text-text-muted mt-1 truncate w-full text-center group-hover:text-text pointer-events-none">{show.name}</span>
                  </div>
                  <!-- Rating menu button -->
                  <button
                    type="button"
                    onclick={(e) => toggleRatingMenu(e, "show", show.id)}
                    onmousedown={(e) => e.stopPropagation()}
                    class="rating-menu-btn absolute top-0 right-0 z-20 w-6 h-6 rounded-bl-lg bg-black/70 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity hover:bg-black/90"
                    title="Change rating"
                  >
                    <MoreVertical class="w-3 h-3 text-white pointer-events-none" />
                  </button>
                  <!-- Rating dropdown menu -->
                  {#if ratingMenuOpen?.type === "show" && ratingMenuOpen?.id === show.id}
                    <div class="absolute top-7 right-0 z-50 bg-surface border border-border rounded-lg shadow-xl py-1 min-w-[120px]">
                      {#each ALL_TIERS as ratingOption}
                        <button
                          type="button"
                          onclick={() => handleQuickRate("show", show.id, ratingOption)}
                          class="w-full px-3 py-1.5 text-left text-xs hover:bg-surface-hover flex items-center gap-2
                            {show.rating === ratingOption ? 'text-accent font-medium' : 'text-text'}"
                        >
                          <span class="text-yellow-400">{renderStars(ratingOption)}</span>
                          <span class="text-text-muted">{ratingOption}</span>
                        </button>
                      {/each}
                      <div class="border-t border-border my-1"></div>
                      <button
                        type="button"
                        onclick={() => handleQuickRate("show", show.id, null)}
                        class="w-full px-3 py-1.5 text-left text-xs text-red-400 hover:bg-surface-hover"
                      >
                        Remove rating
                      </button>
                    </div>
                  {/if}
                </div>
              {/each}
            {:else}
              {#each movieItems as movie}
                <div class="group flex flex-col items-center w-16 relative rating-menu-container">
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div
                    onmousedown={(e) => { if (!(e.target as HTMLElement).closest('.rating-menu-btn')) startDrag({ type: "movie", id: movie.id }, e.clientX, e.clientY); }}
                    onclick={() => { if (!consumeWasDragging()) openMovieDetail(movie.id); }}
                    onkeydown={(e) => { if (e.key === 'Enter') openMovieDetail(movie.id); }}
                    title={movie.title}
                    role="button"
                    tabindex="0"
                    class="flex flex-col items-center w-full transition-transform hover:scale-105 hover:z-10 cursor-grab active:cursor-grabbing"
                  >
                    {#if movie.poster_url}
                      <img
                        src={movie.poster_url}
                        alt={movie.title}
                        class="w-16 h-24 rounded object-cover shadow-lg group-hover:ring-2 group-hover:ring-accent pointer-events-none"
                        loading="lazy"
                        decoding="async"
                      />
                    {:else}
                      <div class="w-16 h-24 rounded bg-border flex items-center justify-center shadow-lg group-hover:ring-2 group-hover:ring-accent">
                        <Film class="w-6 h-6 text-text-muted" />
                      </div>
                    {/if}
                    <span class="text-xs text-text-muted mt-1 truncate w-full text-center group-hover:text-text pointer-events-none">{movie.title}</span>
                  </div>
                  <!-- Rating menu button -->
                  <button
                    type="button"
                    onclick={(e) => toggleRatingMenu(e, "movie", movie.id)}
                    onmousedown={(e) => e.stopPropagation()}
                    class="rating-menu-btn absolute top-0 right-0 z-20 w-6 h-6 rounded-bl-lg bg-black/70 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity hover:bg-black/90"
                    title="Change rating"
                  >
                    <MoreVertical class="w-3 h-3 text-white pointer-events-none" />
                  </button>
                  <!-- Rating dropdown menu -->
                  {#if ratingMenuOpen?.type === "movie" && ratingMenuOpen?.id === movie.id}
                    <div class="absolute top-7 right-0 z-50 bg-surface border border-border rounded-lg shadow-xl py-1 min-w-[120px]">
                      {#each ALL_TIERS as ratingOption}
                        <button
                          type="button"
                          onclick={() => handleQuickRate("movie", movie.id, ratingOption)}
                          class="w-full px-3 py-1.5 text-left text-xs hover:bg-surface-hover flex items-center gap-2
                            {movie.rating === ratingOption ? 'text-accent font-medium' : 'text-text'}"
                        >
                          <span class="text-yellow-400">{renderStars(ratingOption)}</span>
                          <span class="text-text-muted">{ratingOption}</span>
                        </button>
                      {/each}
                      <div class="border-t border-border my-1"></div>
                      <button
                        type="button"
                        onclick={() => handleQuickRate("movie", movie.id, null)}
                        class="w-full px-3 py-1.5 text-left text-xs text-red-400 hover:bg-surface-hover"
                      >
                        Remove rating
                      </button>
                    </div>
                  {/if}
                </div>
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
  {/if}
</div>
