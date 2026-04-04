<script lang="ts">
  import { onMount } from "svelte";
  import { Tv, Film, Trash2, MoreVertical, Plus, ArrowUpCircle, ArrowDownCircle, ArrowRight } from "lucide-svelte";
  import { invoke } from "@tauri-apps/api/core";
  import {
    getTiers,
    getTierListShows,
    getTierListMovies,
    loadTiers,
    loadTierListShows,
    loadTierListMovies,
    updateShowTier,
    updateMovieTier,
    openTierSearchModal,
    promoteShowToTracked,
    promoteMovieToTracked,
    demoteShowToTierOnly,
    demoteMovieToTierOnly,
    type Tier,
    type TierListShow,
    type TierListMovie,
  } from "../../stores/tiers.svelte";
  import { openShowDetail } from "../../stores/showDetail.svelte";
  import { openMovieDetail } from "../../stores/movies.svelte";
  import { registerDropZone, startDrag, type DragData, getIsDragging, consumeWasDragging } from "../../stores/dragDrop.svelte";
  import { setSidebarTab } from "../../stores/sidebar.svelte";
  import { removeShow } from "../../stores/shows.svelte";
  import { removeMovie } from "../../stores/movies.svelte";

  type TierSubTab = "shows" | "movies";
  let subTab = $state<TierSubTab>("shows");

  // Sync sidebar when sub-tab changes
  function switchSubTab(tab: TierSubTab) {
    subTab = tab;
    setSidebarTab(tab);
  }

  // Drag state for visual feedback
  let dragOverTier = $state<number | "unrate" | null>(null);

  // Track if currently dragging (for visual hints)
  const isDragging = $derived(getIsDragging());

  // Element references for drop zones
  let tierRefs = $state<Record<number, HTMLElement | null>>({});
  let unrateRef = $state<HTMLElement | null>(null);

  // Context menu state
  let contextMenuOpen = $state<{ type: "show" | "movie"; id: number } | null>(null);

  // Dynamic tiers from DB
  const tiers = $derived(getTiers());
  const tierListShows = $derived(getTierListShows());
  const tierListMovies = $derived(getTierListMovies());

  // Load data on init (onMount to avoid re-fetching on every reactive cycle)
  onMount(() => {
    loadTiers();
    loadTierListShows();
    loadTierListMovies();
  });

  // Register drop zones reactively when tiers change
  let dropZoneCleanups: (() => void)[] = [];

  $effect(() => {
    // Clean up old drop zones
    dropZoneCleanups.forEach(fn => fn());
    dropZoneCleanups = [];

    // Re-register for current tiers
    // Use a microtask to ensure DOM refs are bound
    const tiersCopy = [...tiers];
    queueMicrotask(() => {
      for (const tier of tiersCopy) {
        const element = tierRefs[tier.id];
        if (element) {
          const cleanup = registerDropZone(`tier-${tier.id}`, element, {
            onDrop: (data, dropX, dropY) => handleTierDrop(data, tier.id, dropX, dropY),
            onDragEnter: () => { dragOverTier = tier.id; },
            onDragLeave: () => { dragOverTier = null; }
          });
          dropZoneCleanups.push(cleanup);
        }
      }

      // Register unrate zone
      if (unrateRef) {
        const cleanup = registerDropZone("unrate", unrateRef, {
          onDrop: (data) => handleTierDrop(data, null),
          onDragEnter: () => { dragOverTier = "unrate"; },
          onDragLeave: () => { dragOverTier = null; }
        });
        dropZoneCleanups.push(cleanup);
      }
    });

    return () => {
      dropZoneCleanups.forEach(fn => fn());
      dropZoneCleanups = [];
    };
  });

  // Close menu on click outside
  $effect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (contextMenuOpen && !(e.target as HTMLElement).closest('.context-menu-container')) {
        contextMenuOpen = null;
      }
    }
    document.addEventListener('click', handleClickOutside);
    return () => document.removeEventListener('click', handleClickOutside);
  });

  // Get items for a specific tier
  function getShowsForTier(tierId: number): TierListShow[] {
    return tierListShows
      .filter(s => s.tier_id === tierId)
      .sort((a, b) => (a.rank_order ?? 999999) - (b.rank_order ?? 999999) || a.id - b.id);
  }

  function getMoviesForTier(tierId: number): TierListMovie[] {
    return tierListMovies
      .filter(m => m.tier_id === tierId)
      .sort((a, b) => (a.rank_order ?? 999999) - (b.rank_order ?? 999999) || a.id - b.id);
  }

  // Untiered items (tier_id is null but in the tier list)
  const untieredShows = $derived(tierListShows.filter(s => s.tier_id === null));
  const untieredMovies = $derived(tierListMovies.filter(m => m.tier_id === null));

  // Stats
  const stats = $derived.by(() => {
    const items = subTab === "shows" ? tierListShows : tierListMovies;
    const tiered = items.filter(i => i.tier_id !== null);
    return { tiered: tiered.length, total: items.length };
  });

  // Handle drop for tier assignment or reordering
  async function handleTierDrop(data: DragData, tierId: number | null, dropX?: number, _dropY?: number) {
    if (tierId === null) {
      // Remove from tier (set tier_id to null)
      if (data.type === "show" && subTab === "shows") {
        await updateShowTier(data.id, null);
      } else if (data.type === "movie" && subTab === "movies") {
        await updateMovieTier(data.id, null);
      }
      return;
    }

    const isShow = data.type === "show" && subTab === "shows";
    const isMovie = data.type === "movie" && subTab === "movies";
    if (!isShow && !isMovie) return;

    // Check if within-tier reorder
    const currentItem = isShow
      ? tierListShows.find(s => s.id === data.id)
      : tierListMovies.find(m => m.id === data.id);

    const isWithinTier = currentItem?.tier_id === tierId;

    if (isWithinTier && dropX !== undefined) {
      // Within-tier reorder
      const tierItems = isShow ? getShowsForTier(tierId) : getMoviesForTier(tierId);
      if (tierItems.length <= 1) return;

      const tierElement = tierRefs[tierId];
      if (!tierElement) return;

      const posterContainers = tierElement.querySelectorAll('.context-menu-container');
      let insertIndex = tierItems.length;

      for (let i = 0; i < posterContainers.length; i++) {
        const rect = posterContainers[i].getBoundingClientRect();
        const midX = rect.left + rect.width / 2;
        if (dropX < midX) {
          insertIndex = i;
          break;
        }
      }

      const reordered = tierItems.filter(item => item.id !== data.id);
      reordered.splice(insertIndex > reordered.length ? reordered.length : insertIndex, 0,
        tierItems.find(item => item.id === data.id)!
      );

      const command = isShow ? "reorder_show_in_tier" : "reorder_movie_in_tier";
      for (let i = 0; i < reordered.length; i++) {
        await invoke(command, { id: reordered[i].id, newRankOrder: i });
      }

      if (isShow) await loadTierListShows();
      else await loadTierListMovies();
    } else {
      // Cross-tier move
      if (isShow) {
        await updateShowTier(data.id, tierId);
      } else {
        await updateMovieTier(data.id, tierId);
      }
    }
  }

  // Context menu actions
  function toggleContextMenu(e: MouseEvent, type: "show" | "movie", id: number) {
    e.stopPropagation();
    e.preventDefault();
    if (contextMenuOpen?.type === type && contextMenuOpen?.id === id) {
      contextMenuOpen = null;
    } else {
      contextMenuOpen = { type, id };
    }
  }

  async function handleMoveTier(type: "show" | "movie", id: number, tierId: number | null) {
    contextMenuOpen = null;
    if (type === "show") {
      await updateShowTier(id, tierId);
    } else {
      await updateMovieTier(id, tierId);
    }
  }

  async function handlePromote(type: "show" | "movie", id: number) {
    contextMenuOpen = null;
    if (type === "show") {
      await promoteShowToTracked(id);
    } else {
      await promoteMovieToTracked(id);
    }
  }

  async function handleDemote(type: "show" | "movie", id: number) {
    contextMenuOpen = null;
    if (type === "show") {
      await demoteShowToTierOnly(id);
    } else {
      await demoteMovieToTierOnly(id);
    }
  }

  async function handleRemoveFromTierList(type: "show" | "movie", id: number) {
    contextMenuOpen = null;
    if (type === "show") {
      await removeShow(id);
      await loadTierListShows();
    } else {
      await removeMovie(id);
      await loadTierListMovies();
    }
  }

  function getTierLabelStyle(tier: Tier): string {
    if (tier.color) {
      return `color: ${tier.color};`;
    }
    return '';
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="h-full flex flex-col" onkeydown={(e) => { if (e.key === 'Escape') contextMenuOpen = null; }}>
  <!-- Header with sub-tabs, stats, and add button -->
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

    <div class="flex items-center gap-4">
      <!-- Stats -->
      {#if stats.total > 0}
        <div class="flex items-center gap-4 text-sm">
          <span class="text-text-muted">
            Tiered: <span class="text-accent font-medium">{stats.tiered}</span> / {stats.total}
          </span>
        </div>
      {/if}

      <!-- Add to Tier List button -->
      <button
        type="button"
        onclick={() => openTierSearchModal()}
        class="flex items-center gap-2 px-3 py-2 text-sm font-medium rounded-lg bg-accent text-white hover:bg-accent/90 transition-colors"
      >
        <Plus class="w-4 h-4" />
        Add to Tier List
      </button>
    </div>
  </div>

  <!-- Drag hint or empty state -->
  {#if tiers.length === 0}
    <div class="flex-1 flex flex-col items-center justify-center text-center py-12">
      <div class="w-16 h-16 rounded-full bg-surface flex items-center justify-center mb-4">
        <Tv class="w-8 h-8 text-text-muted" />
      </div>
      <h3 class="text-lg font-medium text-text mb-2">No tiers configured</h3>
      <p class="text-text-muted text-sm max-w-md">
        Go to Settings &gt; Tier List to set up your tier system.
      </p>
    </div>
  {:else if stats.total === 0}
    <div class="flex-1 flex flex-col items-center justify-center text-center py-12">
      <div class="w-16 h-16 rounded-full bg-surface flex items-center justify-center mb-4">
        {#if subTab === "shows"}
          <Tv class="w-8 h-8 text-text-muted" />
        {:else}
          <Film class="w-8 h-8 text-text-muted" />
        {/if}
      </div>
      <h3 class="text-lg font-medium text-text mb-2">No {subTab} in tier list yet</h3>
      <p class="text-text-muted text-sm max-w-md mb-4">
        Click "Add to Tier List" to search and add {subTab}, or drag them from the sidebar.
      </p>
    </div>
  {:else}
    <p class="text-xs text-text-muted mb-3">
      {#if isDragging}
        <span class="text-accent font-medium">Release to drop on a tier</span>
      {:else}
        Drag {subTab} between tiers to reorder, or click the menu on a poster for more options.
      {/if}
    </p>

    <!-- Tier List -->
    <div class="flex-1 overflow-auto space-y-2">
      {#each tiers as tier (tier.id)}
        {@const showItems = subTab === "shows" ? getShowsForTier(tier.id) : []}
        {@const movieItems = subTab === "movies" ? getMoviesForTier(tier.id) : []}
        {@const items = subTab === "shows" ? showItems : movieItems}
        {@const isEmpty = items.length === 0}

        <div
          bind:this={tierRefs[tier.id]}
          data-drop-zone="tier-{tier.id}"
          role="listbox"
          aria-label="{tier.name} tier"
          class="flex items-stretch rounded-lg transition-all
            {dragOverTier === tier.id ? 'ring-2 ring-accent bg-accent/10' : 'bg-surface'}
            {isEmpty ? 'border-2 border-dashed border-border' : ''}"
          style={tier.color ? `border-left: 4px solid ${tier.color};` : ''}
        >
          <!-- Tier label -->
          <div class="w-28 flex-shrink-0 flex flex-col items-center justify-center py-3 px-2 bg-surface-hover border-r border-border rounded-l-lg">
            <span class="text-sm font-bold" style={getTierLabelStyle(tier)}>{tier.name}</span>
            <span class="text-[10px] text-text-muted mt-0.5">#{tier.position}</span>
          </div>

          <!-- Posters row -->
          <div class="flex-1 flex flex-wrap items-start gap-3 p-3 min-h-[100px]">
            {#if isEmpty}
              <span class="text-text-muted text-sm self-center">Drop here</span>
            {:else if subTab === "shows"}
              {#each showItems as show (show.id)}
                <div class="group flex flex-col items-center w-16 relative context-menu-container">
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div
                    onmousedown={(e) => { if (!(e.target as HTMLElement).closest('.context-menu-btn')) startDrag({ type: "show", id: show.id }, e.clientX, e.clientY); }}
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
                  <!-- Context menu button -->
                  <button
                    type="button"
                    onclick={(e) => toggleContextMenu(e, "show", show.id)}
                    onmousedown={(e) => e.stopPropagation()}
                    class="context-menu-btn absolute top-0 right-0 z-20 w-6 h-6 rounded-bl-lg bg-black/70 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity hover:bg-black/90"
                    title="Options"
                  >
                    <MoreVertical class="w-3 h-3 text-white pointer-events-none" />
                  </button>
                  <!-- Context menu dropdown -->
                  {#if contextMenuOpen?.type === "show" && contextMenuOpen?.id === show.id}
                    <div class="absolute top-7 right-0 z-50 bg-surface border border-border rounded-lg shadow-xl py-1 min-w-[160px]">
                      <!-- Move to tier options -->
                      <div class="px-3 py-1 text-[10px] text-text-muted uppercase tracking-wider">Move to tier</div>
                      {#each tiers as targetTier (targetTier.id)}
                        {#if targetTier.id !== show.tier_id}
                          <button
                            type="button"
                            onclick={() => handleMoveTier("show", show.id, targetTier.id)}
                            class="w-full px-3 py-1.5 text-left text-xs hover:bg-surface-hover flex items-center gap-2 text-text"
                          >
                            <ArrowRight class="w-3 h-3" />
                            {#if targetTier.color}
                              <span class="w-2 h-2 rounded-full" style="background-color: {targetTier.color};"></span>
                            {/if}
                            {targetTier.name}
                          </button>
                        {/if}
                      {/each}
                      <div class="border-t border-border my-1"></div>
                      <!-- Promote/Demote -->
                      {#if show.tier_only}
                        <button
                          type="button"
                          onclick={() => handlePromote("show", show.id)}
                          class="w-full px-3 py-1.5 text-left text-xs hover:bg-surface-hover flex items-center gap-2 text-green-400"
                        >
                          <ArrowUpCircle class="w-3 h-3" />
                          Start tracking
                        </button>
                      {:else}
                        <button
                          type="button"
                          onclick={() => handleDemote("show", show.id)}
                          class="w-full px-3 py-1.5 text-left text-xs hover:bg-surface-hover flex items-center gap-2 text-yellow-400"
                        >
                          <ArrowDownCircle class="w-3 h-3" />
                          Tier only (stop tracking)
                        </button>
                      {/if}
                      <div class="border-t border-border my-1"></div>
                      <!-- Remove from tier -->
                      <button
                        type="button"
                        onclick={() => handleMoveTier("show", show.id, null)}
                        class="w-full px-3 py-1.5 text-left text-xs text-text-muted hover:bg-surface-hover flex items-center gap-2"
                      >
                        <Trash2 class="w-3 h-3" />
                        Remove from tier
                      </button>
                      <!-- Remove entirely -->
                      {#if show.tier_only}
                        <button
                          type="button"
                          onclick={() => handleRemoveFromTierList("show", show.id)}
                          class="w-full px-3 py-1.5 text-left text-xs text-red-400 hover:bg-surface-hover flex items-center gap-2"
                        >
                          <Trash2 class="w-3 h-3" />
                          Remove from tier list
                        </button>
                      {/if}
                    </div>
                  {/if}
                </div>
              {/each}
            {:else}
              {#each movieItems as movie (movie.id)}
                <div class="group flex flex-col items-center w-16 relative context-menu-container">
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div
                    onmousedown={(e) => { if (!(e.target as HTMLElement).closest('.context-menu-btn')) startDrag({ type: "movie", id: movie.id }, e.clientX, e.clientY); }}
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
                  <!-- Context menu button -->
                  <button
                    type="button"
                    onclick={(e) => toggleContextMenu(e, "movie", movie.id)}
                    onmousedown={(e) => e.stopPropagation()}
                    class="context-menu-btn absolute top-0 right-0 z-20 w-6 h-6 rounded-bl-lg bg-black/70 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity hover:bg-black/90"
                    title="Options"
                  >
                    <MoreVertical class="w-3 h-3 text-white pointer-events-none" />
                  </button>
                  <!-- Context menu dropdown -->
                  {#if contextMenuOpen?.type === "movie" && contextMenuOpen?.id === movie.id}
                    <div class="absolute top-7 right-0 z-50 bg-surface border border-border rounded-lg shadow-xl py-1 min-w-[160px]">
                      <div class="px-3 py-1 text-[10px] text-text-muted uppercase tracking-wider">Move to tier</div>
                      {#each tiers as targetTier (targetTier.id)}
                        {#if targetTier.id !== movie.tier_id}
                          <button
                            type="button"
                            onclick={() => handleMoveTier("movie", movie.id, targetTier.id)}
                            class="w-full px-3 py-1.5 text-left text-xs hover:bg-surface-hover flex items-center gap-2 text-text"
                          >
                            <ArrowRight class="w-3 h-3" />
                            {#if targetTier.color}
                              <span class="w-2 h-2 rounded-full" style="background-color: {targetTier.color};"></span>
                            {/if}
                            {targetTier.name}
                          </button>
                        {/if}
                      {/each}
                      <div class="border-t border-border my-1"></div>
                      {#if movie.tier_only}
                        <button
                          type="button"
                          onclick={() => handlePromote("movie", movie.id)}
                          class="w-full px-3 py-1.5 text-left text-xs hover:bg-surface-hover flex items-center gap-2 text-green-400"
                        >
                          <ArrowUpCircle class="w-3 h-3" />
                          Start tracking
                        </button>
                      {:else}
                        <button
                          type="button"
                          onclick={() => handleDemote("movie", movie.id)}
                          class="w-full px-3 py-1.5 text-left text-xs hover:bg-surface-hover flex items-center gap-2 text-yellow-400"
                        >
                          <ArrowDownCircle class="w-3 h-3" />
                          Tier only (stop tracking)
                        </button>
                      {/if}
                      <div class="border-t border-border my-1"></div>
                      <button
                        type="button"
                        onclick={() => handleMoveTier("movie", movie.id, null)}
                        class="w-full px-3 py-1.5 text-left text-xs text-text-muted hover:bg-surface-hover flex items-center gap-2"
                      >
                        <Trash2 class="w-3 h-3" />
                        Remove from tier
                      </button>
                      {#if movie.tier_only}
                        <button
                          type="button"
                          onclick={() => handleRemoveFromTierList("movie", movie.id)}
                          class="w-full px-3 py-1.5 text-left text-xs text-red-400 hover:bg-surface-hover flex items-center gap-2"
                        >
                          <Trash2 class="w-3 h-3" />
                          Remove from tier list
                        </button>
                      {/if}
                    </div>
                  {/if}
                </div>
              {/each}
            {/if}
          </div>
        </div>
      {/each}

      <!-- Untiered items -->
      {#if (subTab === "shows" ? untieredShows : untieredMovies).length > 0}
        <div class="mt-4 pt-4 border-t border-border">
          <h4 class="text-xs text-text-muted uppercase tracking-wider mb-2">Untiered</h4>
          <div class="flex flex-wrap gap-3">
            {#if subTab === "shows"}
              {#each untieredShows as show (show.id)}
                <div class="group flex flex-col items-center w-16 relative context-menu-container">
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div
                    onmousedown={(e) => { if (!(e.target as HTMLElement).closest('.context-menu-btn')) startDrag({ type: "show", id: show.id }, e.clientX, e.clientY); }}
                    onclick={() => { if (!consumeWasDragging()) openShowDetail(show.id); }}
                    onkeydown={(e) => { if (e.key === 'Enter') openShowDetail(show.id); }}
                    title={show.name}
                    role="button"
                    tabindex="0"
                    class="flex flex-col items-center w-full transition-transform hover:scale-105 hover:z-10 cursor-grab active:cursor-grabbing"
                  >
                    {#if show.poster_url}
                      <img src={show.poster_url} alt={show.name} class="w-16 h-24 rounded object-cover shadow-lg group-hover:ring-2 group-hover:ring-accent pointer-events-none" loading="lazy" decoding="async" />
                    {:else}
                      <div class="w-16 h-24 rounded bg-border flex items-center justify-center shadow-lg group-hover:ring-2 group-hover:ring-accent">
                        <Tv class="w-6 h-6 text-text-muted" />
                      </div>
                    {/if}
                    <span class="text-xs text-text-muted mt-1 truncate w-full text-center group-hover:text-text pointer-events-none">{show.name}</span>
                  </div>
                  <button
                    type="button"
                    onclick={(e) => toggleContextMenu(e, "show", show.id)}
                    onmousedown={(e) => e.stopPropagation()}
                    class="context-menu-btn absolute top-0 right-0 z-20 w-6 h-6 rounded-bl-lg bg-black/70 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity hover:bg-black/90"
                    title="Options"
                  >
                    <MoreVertical class="w-3 h-3 text-white pointer-events-none" />
                  </button>
                  {#if contextMenuOpen?.type === "show" && contextMenuOpen?.id === show.id}
                    <div class="absolute top-7 right-0 z-50 bg-surface border border-border rounded-lg shadow-xl py-1 min-w-[160px]">
                      <div class="px-3 py-1 text-[10px] text-text-muted uppercase tracking-wider">Move to tier</div>
                      {#each tiers as targetTier (targetTier.id)}
                        <button
                          type="button"
                          onclick={() => handleMoveTier("show", show.id, targetTier.id)}
                          class="w-full px-3 py-1.5 text-left text-xs hover:bg-surface-hover flex items-center gap-2 text-text"
                        >
                          <ArrowRight class="w-3 h-3" />
                          {#if targetTier.color}
                            <span class="w-2 h-2 rounded-full" style="background-color: {targetTier.color};"></span>
                          {/if}
                          {targetTier.name}
                        </button>
                      {/each}
                      <div class="border-t border-border my-1"></div>
                      {#if show.tier_only}
                        <button type="button" onclick={() => handleRemoveFromTierList("show", show.id)} class="w-full px-3 py-1.5 text-left text-xs text-red-400 hover:bg-surface-hover flex items-center gap-2">
                          <Trash2 class="w-3 h-3" />
                          Remove from tier list
                        </button>
                      {/if}
                    </div>
                  {/if}
                </div>
              {/each}
            {:else}
              {#each untieredMovies as movie (movie.id)}
                <div class="group flex flex-col items-center w-16 relative context-menu-container">
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div
                    onmousedown={(e) => { if (!(e.target as HTMLElement).closest('.context-menu-btn')) startDrag({ type: "movie", id: movie.id }, e.clientX, e.clientY); }}
                    onclick={() => { if (!consumeWasDragging()) openMovieDetail(movie.id); }}
                    onkeydown={(e) => { if (e.key === 'Enter') openMovieDetail(movie.id); }}
                    title={movie.title}
                    role="button"
                    tabindex="0"
                    class="flex flex-col items-center w-full transition-transform hover:scale-105 hover:z-10 cursor-grab active:cursor-grabbing"
                  >
                    {#if movie.poster_url}
                      <img src={movie.poster_url} alt={movie.title} class="w-16 h-24 rounded object-cover shadow-lg group-hover:ring-2 group-hover:ring-accent pointer-events-none" loading="lazy" decoding="async" />
                    {:else}
                      <div class="w-16 h-24 rounded bg-border flex items-center justify-center shadow-lg group-hover:ring-2 group-hover:ring-accent">
                        <Film class="w-6 h-6 text-text-muted" />
                      </div>
                    {/if}
                    <span class="text-xs text-text-muted mt-1 truncate w-full text-center group-hover:text-text pointer-events-none">{movie.title}</span>
                  </div>
                  <button
                    type="button"
                    onclick={(e) => toggleContextMenu(e, "movie", movie.id)}
                    onmousedown={(e) => e.stopPropagation()}
                    class="context-menu-btn absolute top-0 right-0 z-20 w-6 h-6 rounded-bl-lg bg-black/70 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity hover:bg-black/90"
                    title="Options"
                  >
                    <MoreVertical class="w-3 h-3 text-white pointer-events-none" />
                  </button>
                  {#if contextMenuOpen?.type === "movie" && contextMenuOpen?.id === movie.id}
                    <div class="absolute top-7 right-0 z-50 bg-surface border border-border rounded-lg shadow-xl py-1 min-w-[160px]">
                      <div class="px-3 py-1 text-[10px] text-text-muted uppercase tracking-wider">Move to tier</div>
                      {#each tiers as targetTier (targetTier.id)}
                        <button
                          type="button"
                          onclick={() => handleMoveTier("movie", movie.id, targetTier.id)}
                          class="w-full px-3 py-1.5 text-left text-xs hover:bg-surface-hover flex items-center gap-2 text-text"
                        >
                          <ArrowRight class="w-3 h-3" />
                          {#if targetTier.color}
                            <span class="w-2 h-2 rounded-full" style="background-color: {targetTier.color};"></span>
                          {/if}
                          {targetTier.name}
                        </button>
                      {/each}
                      <div class="border-t border-border my-1"></div>
                      {#if movie.tier_only}
                        <button type="button" onclick={() => handleRemoveFromTierList("movie", movie.id)} class="w-full px-3 py-1.5 text-left text-xs text-red-400 hover:bg-surface-hover flex items-center gap-2">
                          <Trash2 class="w-3 h-3" />
                          Remove from tier list
                        </button>
                      {/if}
                    </div>
                  {/if}
                </div>
              {/each}
            {/if}
          </div>
        </div>
      {/if}

      <!-- Unrate/Remove Zone (visible when dragging) -->
      <div
        bind:this={unrateRef}
        data-drop-zone="unrate"
        role="listbox"
        aria-label="Remove from tier"
        class="flex items-center gap-3 p-4 rounded-lg border-2 border-dashed transition-all mt-2
          {dragOverTier === 'unrate' ? 'border-red-500 bg-red-500/10 ring-2 ring-red-500' : 'border-border'}"
      >
        <Trash2 class="w-5 h-5 {dragOverTier === 'unrate' ? 'text-red-500' : 'text-text-muted'}" />
        <span class="{dragOverTier === 'unrate' ? 'text-red-500' : 'text-text-muted'} text-sm">
          Drop here to remove from tier
        </span>
      </div>
    </div>
  {/if}
</div>
