<script lang="ts">
  import { onMount } from "svelte";
  import { Tv, Film, Trash2, MoreVertical, Plus, ArrowUpCircle, ArrowDownCircle, ArrowRight, Search, X } from "lucide-svelte";
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
  import { setSidebarTab } from "../../stores/sidebar.svelte";
  import { removeShow } from "../../stores/shows.svelte";
  import { removeMovie } from "../../stores/movies.svelte";

  type TierSubTab = "shows" | "movies";
  let subTab = $state<TierSubTab>("shows");

  // Sync sidebar when sub-tab changes
  function switchSubTab(tab: TierSubTab) {
    subTab = tab;
    searchQuery = "";
    setSidebarTab(tab);
  }

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

  // Search/filter state
  let searchQuery = $state("");
  const isSearching = $derived(searchQuery.trim().length > 0);

  function matchesSearch(name: string): boolean {
    if (!isSearching) return true;
    return name.toLowerCase().includes(searchQuery.trim().toLowerCase());
  }

  // Hover preview state
  let hoverPreview = $state<{ url: string; name: string; x: number; y: number } | null>(null);
  let hoverTimeout: ReturnType<typeof setTimeout> | null = null;

  function showPreview(e: MouseEvent, url: string | null, name: string) {
    if (!url) return;
    if (hoverTimeout) clearTimeout(hoverTimeout);
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    hoverPreview = {
      url,
      name,
      x: rect.left + rect.width / 2,
      y: rect.top,
    };
  }

  function hidePreview() {
    if (hoverTimeout) clearTimeout(hoverTimeout);
    hoverTimeout = setTimeout(() => { hoverPreview = null; }, 50);
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
      <!-- Search input -->
      {#if stats.total > 0}
        <div class="relative">
          <Search class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-text-muted pointer-events-none" />
          <input
            type="text"
            bind:value={searchQuery}
            placeholder="Find in list..."
            class="w-44 pl-8 pr-7 py-1.5 text-sm bg-surface border border-border rounded-lg text-text placeholder:text-text-muted/50 focus:outline-none focus:ring-1 focus:ring-accent focus:border-accent transition-colors"
          />
          {#if isSearching}
            <button
              type="button"
              onclick={() => { searchQuery = ""; }}
              class="absolute right-2 top-1/2 -translate-y-1/2 text-text-muted hover:text-text transition-colors"
            >
              <X class="w-3.5 h-3.5" />
            </button>
          {/if}
        </div>
      {/if}

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
        onclick={() => openTierSearchModal(subTab)}
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
      Click the menu on a poster to move it between tiers or remove it.
    </p>

    <!-- Tier List -->
    <div class="flex-1 overflow-auto space-y-2">
      {#each tiers as tier (tier.id)}
        {@const showItems = subTab === "shows" ? getShowsForTier(tier.id) : []}
        {@const movieItems = subTab === "movies" ? getMoviesForTier(tier.id) : []}
        {@const items = subTab === "shows" ? showItems : movieItems}
        {@const isEmpty = items.length === 0}

        <div
          role="listbox"
          aria-label="{tier.name} tier"
          class="flex items-stretch rounded-lg transition-all bg-surface
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
              <span class="text-text-muted text-sm self-center">Empty</span>
            {:else if subTab === "shows"}
              {#each showItems as show (show.id)}
                {@const isMatch = matchesSearch(show.name)}
                <div class="group flex flex-col items-center w-16 relative context-menu-container transition-opacity {isSearching && !isMatch ? 'opacity-20' : ''}">
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div
                    onclick={() => openShowDetail(show.id)}
                    onkeydown={(e) => { if (e.key === 'Enter') openShowDetail(show.id); }}
                    onmouseenter={(e) => showPreview(e, show.poster_url, show.name)}
                    onmouseleave={hidePreview}
                    title={show.name}
                    role="button"
                    tabindex="0"
                    class="flex flex-col items-center w-full transition-transform hover:scale-105 hover:z-10 cursor-pointer"
                  >
                    {#if show.poster_url}
                      <img
                        src={show.poster_url}
                        alt={show.name}
                        class="w-16 h-24 rounded object-cover shadow-lg pointer-events-none
                          {isSearching && isMatch ? 'ring-2 ring-accent shadow-accent/30' : 'group-hover:ring-2 group-hover:ring-accent'}"
                        loading="lazy"
                        decoding="async"
                      />
                    {:else}
                      <div class="w-16 h-24 rounded bg-border flex items-center justify-center shadow-lg
                        {isSearching && isMatch ? 'ring-2 ring-accent shadow-accent/30' : 'group-hover:ring-2 group-hover:ring-accent'}">
                        <Tv class="w-6 h-6 text-text-muted" />
                      </div>
                    {/if}
                    <span class="text-xs mt-1 truncate w-full text-center pointer-events-none
                      {isSearching && isMatch ? 'text-accent font-medium' : 'text-text-muted group-hover:text-text'}">{show.name}</span>
                  </div>
                  <!-- Context menu button -->
                  <button
                    type="button"
                    onclick={(e) => toggleContextMenu(e, "show", show.id)}
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
                {@const isMatch = matchesSearch(movie.title)}
                <div class="group flex flex-col items-center w-16 relative context-menu-container transition-opacity {isSearching && !isMatch ? 'opacity-20' : ''}">
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div
                    onclick={() => openMovieDetail(movie.id)}
                    onkeydown={(e) => { if (e.key === 'Enter') openMovieDetail(movie.id); }}
                    onmouseenter={(e) => showPreview(e, movie.poster_url, movie.title)}
                    onmouseleave={hidePreview}
                    title={movie.title}
                    role="button"
                    tabindex="0"
                    class="flex flex-col items-center w-full transition-transform hover:scale-105 hover:z-10 cursor-pointer"
                  >
                    {#if movie.poster_url}
                      <img
                        src={movie.poster_url}
                        alt={movie.title}
                        class="w-16 h-24 rounded object-cover shadow-lg pointer-events-none
                          {isSearching && isMatch ? 'ring-2 ring-accent shadow-accent/30' : 'group-hover:ring-2 group-hover:ring-accent'}"
                        loading="lazy"
                        decoding="async"
                      />
                    {:else}
                      <div class="w-16 h-24 rounded bg-border flex items-center justify-center shadow-lg
                        {isSearching && isMatch ? 'ring-2 ring-accent shadow-accent/30' : 'group-hover:ring-2 group-hover:ring-accent'}">
                        <Film class="w-6 h-6 text-text-muted" />
                      </div>
                    {/if}
                    <span class="text-xs mt-1 truncate w-full text-center pointer-events-none
                      {isSearching && isMatch ? 'text-accent font-medium' : 'text-text-muted group-hover:text-text'}">{movie.title}</span>
                  </div>
                  <!-- Context menu button -->
                  <button
                    type="button"
                    onclick={(e) => toggleContextMenu(e, "movie", movie.id)}
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
                {@const isMatch = matchesSearch(show.name)}
                <div class="group flex flex-col items-center w-16 relative context-menu-container transition-opacity {isSearching && !isMatch ? 'opacity-20' : ''}">
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div
                    onclick={() => openShowDetail(show.id)}
                    onkeydown={(e) => { if (e.key === 'Enter') openShowDetail(show.id); }}
                    onmouseenter={(e) => showPreview(e, show.poster_url, show.name)}
                    onmouseleave={hidePreview}
                    title={show.name}
                    role="button"
                    tabindex="0"
                    class="flex flex-col items-center w-full transition-transform hover:scale-105 hover:z-10 cursor-pointer"
                  >
                    {#if show.poster_url}
                      <img src={show.poster_url} alt={show.name} class="w-16 h-24 rounded object-cover shadow-lg pointer-events-none {isSearching && isMatch ? 'ring-2 ring-accent shadow-accent/30' : 'group-hover:ring-2 group-hover:ring-accent'}" loading="lazy" decoding="async" />
                    {:else}
                      <div class="w-16 h-24 rounded bg-border flex items-center justify-center shadow-lg {isSearching && isMatch ? 'ring-2 ring-accent shadow-accent/30' : 'group-hover:ring-2 group-hover:ring-accent'}">
                        <Tv class="w-6 h-6 text-text-muted" />
                      </div>
                    {/if}
                    <span class="text-xs mt-1 truncate w-full text-center pointer-events-none {isSearching && isMatch ? 'text-accent font-medium' : 'text-text-muted group-hover:text-text'}">{show.name}</span>
                  </div>
                  <button
                    type="button"
                    onclick={(e) => toggleContextMenu(e, "show", show.id)}
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
                {@const isMatch = matchesSearch(movie.title)}
                <div class="group flex flex-col items-center w-16 relative context-menu-container transition-opacity {isSearching && !isMatch ? 'opacity-20' : ''}">
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div
                    onclick={() => openMovieDetail(movie.id)}
                    onkeydown={(e) => { if (e.key === 'Enter') openMovieDetail(movie.id); }}
                    onmouseenter={(e) => showPreview(e, movie.poster_url, movie.title)}
                    onmouseleave={hidePreview}
                    title={movie.title}
                    role="button"
                    tabindex="0"
                    class="flex flex-col items-center w-full transition-transform hover:scale-105 hover:z-10 cursor-pointer"
                  >
                    {#if movie.poster_url}
                      <img src={movie.poster_url} alt={movie.title} class="w-16 h-24 rounded object-cover shadow-lg pointer-events-none {isSearching && isMatch ? 'ring-2 ring-accent shadow-accent/30' : 'group-hover:ring-2 group-hover:ring-accent'}" loading="lazy" decoding="async" />
                    {:else}
                      <div class="w-16 h-24 rounded bg-border flex items-center justify-center shadow-lg {isSearching && isMatch ? 'ring-2 ring-accent shadow-accent/30' : 'group-hover:ring-2 group-hover:ring-accent'}">
                        <Film class="w-6 h-6 text-text-muted" />
                      </div>
                    {/if}
                    <span class="text-xs mt-1 truncate w-full text-center pointer-events-none {isSearching && isMatch ? 'text-accent font-medium' : 'text-text-muted group-hover:text-text'}">{movie.title}</span>
                  </div>
                  <button
                    type="button"
                    onclick={(e) => toggleContextMenu(e, "movie", movie.id)}
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

    </div>
  {/if}

  <!-- Hover poster preview -->
  {#if hoverPreview}
    {@const previewW = 128}
    {@const previewH = 192}
    {@const left = Math.max(8, Math.min(hoverPreview.x - previewW / 2, window.innerWidth - previewW - 8))}
    {@const top = hoverPreview.y - previewH - 12 > 0 ? hoverPreview.y - previewH - 12 : hoverPreview.y + 100 + 8}
    <div
      class="fixed z-[9999] pointer-events-none"
      style="left: {left}px; top: {top}px;"
    >
      <img
        src={hoverPreview.url}
        alt={hoverPreview.name}
        class="w-32 h-48 rounded-lg object-cover shadow-2xl ring-1 ring-border"
      />
    </div>
  {/if}
</div>
