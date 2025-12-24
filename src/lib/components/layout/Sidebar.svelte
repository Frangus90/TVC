<script lang="ts">
  import { Plus, Tv, Trash2, RefreshCw, Check, Film, Archive, RotateCcw, CalendarX, BarChart3, Database, PanelLeftClose, PanelLeft, Trophy } from "lucide-svelte";
  import { onMount } from "svelte";
  import {
    getTrackedShows,
    loadTrackedShows,
    openSearchModal,
    removeShow,
    getArchivedShows,
    loadArchivedShows,
    unarchiveShow,
    isShowsLoading,
    isArchivedShowsLoading,
  } from "../../stores/shows.svelte";
  import {
    getTrackedMovies,
    loadTrackedMovies,
    openMovieSearchModal,
    removeMovie,
    getArchivedMovies,
    loadArchivedMovies,
    unarchiveMovie,
    unscheduleMovie,
    openMovieDetail,
    isMoviesLoading,
    isArchivedMoviesLoading,
  } from "../../stores/movies.svelte";
  import { openShowDetail } from "../../stores/showDetail.svelte";
  import {
    triggerUpdateCheck,
    isCheckingForUpdates,
  } from "../../stores/updates.svelte";
  import { openStatisticsModal } from "../../stores/statistics.svelte";
  import { openDataManagement } from "../../stores/dataManagement.svelte";
  import { isSidebarCollapsed, toggleSidebar } from "../../stores/sidebar.svelte";
  import { getThemeSettings } from "../../stores/theme.svelte";
  import { openConfirmDialog } from "../../stores/confirmDialog.svelte";
  import SkeletonLoader from "../common/SkeletonLoader.svelte";
  import EmptyState from "../common/EmptyState.svelte";

  type SidebarTab = "shows" | "movies" | "rankings" | "archive";
  type RankingsSubTab = "shows" | "movies";
  let activeTab = $state<SidebarTab>("shows");
  let rankingsSubTab = $state<RankingsSubTab>("shows");
  let selectedItems = $state<Set<number>>(new Set());
  let bulkMode = $state(false);

  const theme = $derived(getThemeSettings());
  const isCompactList = $derived(theme.compactSpacing && theme.hidePosters);

  // Rankings derived state
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

  function groupByRating<T extends { rating: number | null }>(items: T[]): [number, T[]][] {
    const groups = new Map<number, T[]>();
    for (const item of items) {
      if (item.rating === null) continue;
      if (!groups.has(item.rating)) groups.set(item.rating, []);
      groups.get(item.rating)!.push(item);
    }
    return Array.from(groups.entries()).sort((a, b) => b[0] - a[0]);
  }

  const showStats = $derived.by(() => {
    const rated = rankedShows;
    if (rated.length === 0) return { avg: 0, count: 0, distribution: new Map<number, number>() };
    const avg = rated.reduce((sum, s) => sum + (s.rating ?? 0), 0) / rated.length;
    const distribution = new Map<number, number>();
    for (const s of rated) {
      const r = s.rating ?? 0;
      distribution.set(r, (distribution.get(r) ?? 0) + 1);
    }
    return { avg, count: rated.length, distribution };
  });

  const movieStats = $derived.by(() => {
    const rated = rankedMovies;
    if (rated.length === 0) return { avg: 0, count: 0, distribution: new Map<number, number>() };
    const avg = rated.reduce((sum, m) => sum + (m.rating ?? 0), 0) / rated.length;
    const distribution = new Map<number, number>();
    for (const m of rated) {
      const r = m.rating ?? 0;
      distribution.set(r, (distribution.get(r) ?? 0) + 1);
    }
    return { avg, count: rated.length, distribution };
  });

  function renderStars(rating: number): string {
    const fullStars = Math.floor(rating);
    const hasHalf = rating % 1 >= 0.5;
    return "★".repeat(fullStars) + (hasHalf ? "½" : "");
  }

  onMount(() => {
    // Load all data in parallel for faster startup
    Promise.all([
      loadTrackedShows(),
      loadTrackedMovies(),
      loadArchivedShows(),
      loadArchivedMovies(),
    ]).catch((error) => {
      console.error("Failed to load sidebar data:", error);
    });
  });

  function switchTab(tab: SidebarTab) {
    activeTab = tab;
    bulkMode = false;
    selectedItems = new Set();
  }

  async function handleRemoveShow(event: MouseEvent, showId: number) {
    event.stopPropagation();
    await removeShow(showId);
  }

  async function handleRemoveMovie(event: MouseEvent, movieId: number) {
    event.stopPropagation();
    await removeMovie(movieId);
  }

  async function handleUnscheduleMovie(event: MouseEvent, movieId: number) {
    event.stopPropagation();
    await unscheduleMovie(movieId);
  }

  function toggleItemSelection(itemId: number) {
    const newSelected = new Set(selectedItems);
    if (newSelected.has(itemId)) {
      newSelected.delete(itemId);
    } else {
      newSelected.add(itemId);
    }
    selectedItems = newSelected;
  }

  async function handleBulkRemove() {
    if (selectedItems.size === 0) return;

    const itemType = activeTab === "shows" ? "show" : "movie";
    const confirmed = await openConfirmDialog({
      title: "Remove Items",
      message: `Are you sure you want to remove ${selectedItems.size} ${itemType}${selectedItems.size > 1 ? 's' : ''}?`,
      type: "danger",
      confirmLabel: "Remove",
      cancelLabel: "Cancel",
    });

    if (confirmed) {
      for (const itemId of selectedItems) {
        if (activeTab === "shows") {
          await removeShow(itemId);
        } else if (activeTab === "movies") {
          await removeMovie(itemId);
        }
      }
      selectedItems = new Set();
      bulkMode = false;
    }
  }

  async function handleUnarchiveShow(showId: number) {
    await unarchiveShow(showId);
  }

  async function handleUnarchiveMovie(movieId: number) {
    await unarchiveMovie(movieId);
  }

  function handleAddClick() {
    if (activeTab === "shows") {
      openSearchModal();
    } else if (activeTab === "movies") {
      openMovieSearchModal();
    }
  }

  // Combined archive items with type indicator
  interface ArchiveItem {
    id: number;
    name: string;
    poster_url: string | null;
    type: "show" | "movie";
    color?: string | null;
  }

  function getArchiveItems(): ArchiveItem[] {
    const shows: ArchiveItem[] = getArchivedShows().map(s => ({
      id: s.id,
      name: s.name,
      poster_url: s.poster_url,
      type: "show" as const,
      color: s.color,
    }));
    const movies: ArchiveItem[] = getArchivedMovies().map(m => ({
      id: m.id,
      name: m.title,
      poster_url: m.poster_url,
      type: "movie" as const,
      color: m.color,
    }));
    return [...shows, ...movies].sort((a, b) => a.name.localeCompare(b.name));
  }
</script>

<aside class="bg-surface border-r border-border flex flex-col transition-all duration-200 {isSidebarCollapsed() ? 'w-16' : 'w-64'}">
  <!-- Logo -->
  <div class="p-4 border-b border-border">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2 text-xl font-semibold {isSidebarCollapsed() ? 'justify-center w-full' : ''}">
        <Tv class="w-6 h-6 text-accent flex-shrink-0" />
        {#if !isSidebarCollapsed()}
          <span>TVC</span>
        {/if}
      </div>
      {#if !isSidebarCollapsed()}
        <button
          type="button"
          onclick={toggleSidebar}
          class="p-1.5 rounded hover:bg-surface-hover transition-colors"
          aria-label="Collapse sidebar"
          title="Collapse sidebar"
        >
          <PanelLeftClose class="w-4 h-4 text-text-muted" />
        </button>
      {/if}
    </div>
    {#if isSidebarCollapsed()}
      <button
        type="button"
        onclick={toggleSidebar}
        class="w-full mt-2 p-1.5 rounded hover:bg-surface-hover transition-colors flex justify-center"
        aria-label="Expand sidebar"
        title="Expand sidebar"
      >
        <PanelLeft class="w-4 h-4 text-text-muted" />
      </button>
    {/if}
  </div>

  <!-- Tab Row -->
  <div class="flex {isSidebarCollapsed() ? 'flex-col' : ''} border-b border-border">
    <button
      type="button"
      onclick={() => switchTab("shows")}
      class="flex-1 flex items-center justify-center gap-1 px-1 py-2.5 text-xs font-medium transition-colors
        {activeTab === 'shows' ? 'text-accent border-b-2 border-accent bg-accent/5' : 'text-text-muted hover:text-text hover:bg-surface-hover'}
        {isSidebarCollapsed() ? 'border-b-0 border-l-2' : ''}"
      aria-label="TV Shows"
      aria-pressed={activeTab === 'shows'}
      title="TV Shows"
    >
      <Tv class="w-3.5 h-3.5 flex-shrink-0" />
      {#if !isSidebarCollapsed()}
        <span>Shows</span>
      {/if}
    </button>
    <button
      type="button"
      onclick={() => switchTab("movies")}
      class="flex-1 flex items-center justify-center gap-1 px-1 py-2.5 text-xs font-medium transition-colors
        {activeTab === 'movies' ? 'text-accent border-b-2 border-accent bg-accent/5' : 'text-text-muted hover:text-text hover:bg-surface-hover'}
        {isSidebarCollapsed() ? 'border-b-0 border-l-2' : ''}"
      aria-label="Movies"
      aria-pressed={activeTab === 'movies'}
      title="Movies"
    >
      <Film class="w-3.5 h-3.5 flex-shrink-0" />
      {#if !isSidebarCollapsed()}
        <span>Movies</span>
      {/if}
    </button>
    <button
      type="button"
      onclick={() => switchTab("rankings")}
      class="flex-1 flex items-center justify-center gap-1 px-1 py-2.5 text-xs font-medium transition-colors
        {activeTab === 'rankings' ? 'text-accent border-b-2 border-accent bg-accent/5' : 'text-text-muted hover:text-text hover:bg-surface-hover'}
        {isSidebarCollapsed() ? 'border-b-0 border-l-2' : ''}"
      aria-label="Rankings"
      aria-pressed={activeTab === 'rankings'}
      title="Rankings"
    >
      <Trophy class="w-3.5 h-3.5 flex-shrink-0" />
      {#if !isSidebarCollapsed()}
        <span>Rank</span>
      {/if}
    </button>
    <button
      type="button"
      onclick={() => switchTab("archive")}
      class="flex-1 flex items-center justify-center gap-1 px-1 py-2.5 text-xs font-medium transition-colors
        {activeTab === 'archive' ? 'text-accent border-b-2 border-accent bg-accent/5' : 'text-text-muted hover:text-text hover:bg-surface-hover'}
        {isSidebarCollapsed() ? 'border-b-0 border-l-2' : ''}"
      aria-label="Archive"
      aria-pressed={activeTab === 'archive'}
      title="Archive"
    >
      <Archive class="w-3.5 h-3.5 flex-shrink-0" />
      {#if !isSidebarCollapsed()}
        <span>Archive</span>
      {/if}
    </button>
  </div>

  <!-- Selection Controls Row (only for shows/movies tabs, hidden when collapsed) -->
  {#if (activeTab === "shows" || activeTab === "movies") && !isSidebarCollapsed()}
    <div class="flex items-center justify-end gap-2 px-3 py-2 border-b border-border bg-surface/50">
      {#if bulkMode}
        <button
          type="button"
          onclick={() => { bulkMode = false; selectedItems = new Set(); }}
          class="text-xs text-text-muted hover:text-text"
        >
          Cancel
        </button>
        {#if selectedItems.size > 0}
          <button
            type="button"
            onclick={handleBulkRemove}
            class="text-xs text-red-400 hover:text-red-300"
          >
            Remove ({selectedItems.size})
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
  {/if}

  <!-- Content Area (hidden when collapsed) -->
  <div class="flex-1 overflow-auto p-3 {isSidebarCollapsed() ? 'hidden' : ''}">
    <!-- TV Shows Tab -->
    {#if activeTab === "shows"}
      {#if isShowsLoading()}
        <div class="space-y-2 py-2">
          {#each Array(5) as _}
            <div class="flex items-center gap-3 px-3 py-2">
              {#if !theme.hidePosters}
                <SkeletonLoader width="2rem" height="3rem" />
              {/if}
              <SkeletonLoader width="60%" height="1rem" />
            </div>
          {/each}
        </div>
      {:else if getTrackedShows().length === 0}
        <EmptyState
          icon={Tv}
          title="No shows tracked"
          message="Start tracking your favorite TV shows to see them here."
          action={{ label: "Add Show", onclick: openSearchModal }}
        />
      {:else}
        <ul class="space-y-1">
          {#each getTrackedShows() as show}
            {@const isSelected = selectedItems.has(show.id)}
            <li>
              {#if bulkMode}
                <button
                  type="button"
                  onclick={() => toggleItemSelection(show.id)}
                  class="group w-full flex items-center gap-3 {isCompactList ? 'px-2 py-1' : 'px-3 py-2'} rounded-lg hover:bg-surface-hover transition-colors text-left"
                >
                  <span
                    class="w-5 h-5 rounded border flex-shrink-0 flex items-center justify-center
                      {isSelected ? 'bg-accent border-accent' : 'border-border'}"
                  >
                    {#if isSelected}
                      <Check class="w-3 h-3 text-white" />
                    {/if}
                  </span>
                  {#if !theme.hidePosters}
                    {#if show.poster_url}
                      <img src={show.poster_url} alt="" class="w-8 h-12 rounded object-cover" loading="lazy" decoding="async" />
                    {:else}
                      <div class="w-8 h-12 rounded bg-border flex items-center justify-center">
                        <Tv class="w-4 h-4 text-text-muted" />
                      </div>
                    {/if}
                  {/if}
                  <span class="flex-1 text-sm truncate">{show.name}</span>
                </button>
              {:else}
                <div class="group w-full flex items-center {isCompactList ? 'gap-2 px-2 py-1' : 'gap-3 px-3 py-2'} rounded-lg hover:bg-surface-hover transition-colors">
                  <button
                    type="button"
                    class="flex-1 flex items-center {isCompactList ? 'gap-2' : 'gap-3'} text-left"
                    onclick={() => openShowDetail(show.id)}
                  >
                    {#if !theme.hidePosters}
                      {#if show.poster_url}
                        <img src={show.poster_url} alt="" class="w-8 h-12 rounded object-cover" loading="lazy" decoding="async" />
                      {:else}
                        <div class="w-8 h-12 rounded bg-border flex items-center justify-center">
                          <Tv class="w-4 h-4 text-text-muted" />
                        </div>
                      {/if}
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
    {/if}

    <!-- Movies Tab -->
    {#if activeTab === "movies"}
      {#if isMoviesLoading()}
        <div class="space-y-2 py-2">
          {#each Array(5) as _}
            <div class="flex items-center gap-3 px-3 py-2">
              {#if !theme.hidePosters}
                <SkeletonLoader width="2rem" height="3rem" />
              {/if}
              <SkeletonLoader width="60%" height="1rem" />
            </div>
          {/each}
        </div>
      {:else if getTrackedMovies().length === 0}
        <EmptyState
          icon={Film}
          title="No movies tracked"
          message="Start tracking movies you want to watch."
          action={{ label: "Add Movie", onclick: openMovieSearchModal }}
        />
      {:else}
        <ul class="space-y-1">
          {#each getTrackedMovies() as movie}
            {@const isSelected = selectedItems.has(movie.id)}
            <li>
              {#if bulkMode}
                <button
                  type="button"
                  onclick={() => toggleItemSelection(movie.id)}
                  class="group w-full flex items-center gap-3 {isCompactList ? 'px-2 py-1' : 'px-3 py-2'} rounded-lg hover:bg-surface-hover transition-colors text-left"
                >
                  <span
                    class="w-5 h-5 rounded border flex-shrink-0 flex items-center justify-center
                      {isSelected ? 'bg-accent border-accent' : 'border-border'}"
                  >
                    {#if isSelected}
                      <Check class="w-3 h-3 text-white" />
                    {/if}
                  </span>
                  {#if !theme.hidePosters}
                    {#if movie.poster_url}
                      <img src={movie.poster_url} alt="" class="w-8 h-12 rounded object-cover" loading="lazy" decoding="async" />
                    {:else}
                      <div class="w-8 h-12 rounded bg-border flex items-center justify-center">
                        <Film class="w-4 h-4 text-text-muted" />
                      </div>
                    {/if}
                  {/if}
                  <span class="flex-1 text-sm truncate">{movie.title}</span>
                </button>
              {:else}
                <div class="group w-full flex items-center {isCompactList ? 'gap-2 px-2 py-1' : 'gap-3 px-3 py-2'} rounded-lg hover:bg-surface-hover transition-colors">
                  <button
                    type="button"
                    class="flex-1 flex items-center {isCompactList ? 'gap-2' : 'gap-3'} text-left"
                    onclick={() => openMovieDetail(movie.id)}
                  >
                    {#if !theme.hidePosters}
                      {#if movie.poster_url}
                        <img src={movie.poster_url} alt="" class="w-8 h-12 rounded object-cover" loading="lazy" decoding="async" />
                      {:else}
                        <div class="w-8 h-12 rounded bg-border flex items-center justify-center">
                          <Film class="w-4 h-4 text-text-muted" />
                        </div>
                      {/if}
                    {/if}
                    <div class="flex-1 flex items-center gap-2 min-w-0">
                      {#if movie.color}
                        <div
                          class="w-3 h-3 rounded-full flex-shrink-0"
                          style="background-color: {movie.color};"
                          title="Movie color"
                        ></div>
                      {/if}
                      <span class="text-sm truncate">{movie.title}</span>
                    </div>
                  </button>
                  <div class="flex items-center gap-1">
                    {#if movie.scheduled_date}
                      <button
                        type="button"
                        onclick={(e) => handleUnscheduleMovie(e, movie.id)}
                        class="opacity-0 group-hover:opacity-100 p-1.5 rounded hover:bg-orange-500/20 text-orange-400 transition-all"
                        aria-label="Remove from schedule"
                        title="Remove from schedule"
                      >
                        <CalendarX class="w-4 h-4" />
                      </button>
                    {/if}
                    <button
                      type="button"
                      onclick={(e) => { e.stopPropagation(); handleRemoveMovie(e, movie.id); }}
                      class="opacity-0 group-hover:opacity-100 p-1.5 rounded hover:bg-red-500/20 text-red-400 transition-all"
                      aria-label="Remove movie"
                    >
                      <Trash2 class="w-4 h-4" />
                    </button>
                  </div>
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    {/if}

    <!-- Archive Tab -->
    {#if activeTab === "archive"}
      {@const archiveItems = getArchiveItems()}
      {#if isArchivedShowsLoading() || isArchivedMoviesLoading()}
        <div class="space-y-2 py-2">
          {#each Array(5) as _}
            <div class="flex items-center gap-3 px-3 py-2">
              {#if !theme.hidePosters}
                <SkeletonLoader width="2rem" height="3rem" />
              {/if}
              <SkeletonLoader width="60%" height="1rem" />
            </div>
          {/each}
        </div>
      {:else if archiveItems.length === 0}
        <EmptyState
          icon={Archive}
          title="No archived items"
          message="Archive shows or movies you're done with to keep your list clean."
        />
      {:else}
        <ul class="space-y-1">
          {#each archiveItems as item}
            <li>
              <div class="group w-full flex items-center {isCompactList ? 'gap-2 px-2 py-1' : 'gap-3 px-3 py-2'} rounded-lg hover:bg-surface-hover transition-colors">
                <div class="flex-1 flex items-center {isCompactList ? 'gap-2' : 'gap-3'}">
                  {#if !theme.hidePosters}
                    {#if item.poster_url}
                      <img src={item.poster_url} alt="" class="w-8 h-12 rounded object-cover" loading="lazy" decoding="async" />
                    {:else}
                      <div class="w-8 h-12 rounded bg-border flex items-center justify-center">
                        {#if item.type === "show"}
                          <Tv class="w-4 h-4 text-text-muted" />
                        {:else}
                          <Film class="w-4 h-4 text-text-muted" />
                        {/if}
                      </div>
                    {/if}
                  {/if}
                  <div class="flex-1 flex flex-col min-w-0">
                    <div class="flex items-center gap-2">
                      {#if item.color}
                        <div
                          class="w-3 h-3 rounded-full flex-shrink-0"
                          style="background-color: {item.color};"
                        ></div>
                      {/if}
                      <span class="text-sm truncate">{item.name}</span>
                    </div>
                    {#if !isCompactList}
                      <span class="text-xs text-text-muted">
                        {item.type === "show" ? "TV Show" : "Movie"}
                      </span>
                    {/if}
                  </div>
                </div>
                <button
                  type="button"
                  onclick={() => item.type === "show" ? handleUnarchiveShow(item.id) : handleUnarchiveMovie(item.id)}
                  class="opacity-0 group-hover:opacity-100 p-1.5 rounded hover:bg-accent/20 text-accent transition-all"
                  aria-label="Restore from archive"
                  title="Restore from archive"
                >
                  <RotateCcw class="w-4 h-4" />
                </button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    {/if}

    <!-- Rankings Tab -->
    {#if activeTab === "rankings"}
      <!-- Sub-tabs for Shows/Movies -->
      <div class="flex gap-1 mb-3 p-1 bg-background rounded-lg">
        <button
          type="button"
          onclick={() => rankingsSubTab = "shows"}
          class="flex-1 flex items-center justify-center gap-1.5 px-2 py-1.5 text-xs font-medium rounded transition-colors
            {rankingsSubTab === 'shows' ? 'bg-surface text-accent' : 'text-text-muted hover:text-text'}"
        >
          <Tv class="w-3 h-3" />
          Shows
        </button>
        <button
          type="button"
          onclick={() => rankingsSubTab = "movies"}
          class="flex-1 flex items-center justify-center gap-1.5 px-2 py-1.5 text-xs font-medium rounded transition-colors
            {rankingsSubTab === 'movies' ? 'bg-surface text-accent' : 'text-text-muted hover:text-text'}"
        >
          <Film class="w-3 h-3" />
          Movies
        </button>
      </div>

      <!-- Statistics -->
      {@const stats = rankingsSubTab === "shows" ? showStats : movieStats}
      {#if stats.count > 0}
        <div class="mb-4 p-3 bg-background rounded-lg">
          <div class="flex items-center justify-between text-sm mb-2">
            <span class="text-text-muted">Average:</span>
            <span class="text-yellow-400 font-medium">{stats.avg.toFixed(1)} ★</span>
          </div>
          <div class="flex items-center justify-between text-sm mb-3">
            <span class="text-text-muted">Rated:</span>
            <span class="text-text">{stats.count} {rankingsSubTab === "shows" ? "shows" : "movies"}</span>
          </div>
          <!-- Distribution bar -->
          <div class="flex h-2 rounded-full overflow-hidden bg-surface">
            {#each Array.from(stats.distribution.entries()).sort((a, b) => b[0] - a[0]) as [rating, count]}
              <div
                class="h-full transition-all"
                style="width: {(count / stats.count) * 100}%; background-color: hsl({(rating / 5) * 60}, 80%, 50%);"
                title="{rating}★: {count}"
              ></div>
            {/each}
          </div>
          <div class="flex justify-between text-xs text-text-muted mt-1">
            <span>5★</span>
            <span>1★</span>
          </div>
        </div>
      {/if}

      <!-- Grouped Rankings List - Shows -->
      {#if rankingsSubTab === "shows"}
        {#if rankedShows.length === 0}
          <EmptyState
            icon={Trophy}
            title="No rated shows"
            message="Rate shows from their detail page to see them here."
          />
        {:else}
          {@const grouped = groupByRating(rankedShows)}
          <div class="space-y-4">
            {#each grouped as [rating, groupItems]}
              <div>
                <div class="flex items-center gap-2 mb-2 px-1">
                  <span class="text-yellow-400 text-sm font-medium">{renderStars(rating)}</span>
                  <span class="text-xs text-text-muted">({rating})</span>
                </div>
                <ul class="space-y-1">
                  {#each groupItems as show}
                    <li>
                      <button
                        type="button"
                        onclick={() => openShowDetail(show.id)}
                        class="w-full flex items-center {isCompactList ? 'gap-2 px-2 py-1' : 'gap-3 px-3 py-2'} rounded-lg hover:bg-surface-hover transition-colors text-left"
                      >
                        {#if !theme.hidePosters}
                          {#if show.poster_url}
                            <img src={show.poster_url} alt="" class="w-8 h-12 rounded object-cover" loading="lazy" decoding="async" />
                          {:else}
                            <div class="w-8 h-12 rounded bg-border flex items-center justify-center">
                              <Tv class="w-4 h-4 text-text-muted" />
                            </div>
                          {/if}
                        {/if}
                        <span class="flex-1 text-sm truncate">{show.name}</span>
                      </button>
                    </li>
                  {/each}
                </ul>
              </div>
            {/each}
          </div>
        {/if}
      {:else}
        <!-- Grouped Rankings List - Movies -->
        {#if rankedMovies.length === 0}
          <EmptyState
            icon={Trophy}
            title="No rated movies"
            message="Rate movies from their detail page to see them here."
          />
        {:else}
          {@const grouped = groupByRating(rankedMovies)}
          <div class="space-y-4">
            {#each grouped as [rating, groupItems]}
              <div>
                <div class="flex items-center gap-2 mb-2 px-1">
                  <span class="text-yellow-400 text-sm font-medium">{renderStars(rating)}</span>
                  <span class="text-xs text-text-muted">({rating})</span>
                </div>
                <ul class="space-y-1">
                  {#each groupItems as movie}
                    <li>
                      <button
                        type="button"
                        onclick={() => openMovieDetail(movie.id)}
                        class="w-full flex items-center {isCompactList ? 'gap-2 px-2 py-1' : 'gap-3 px-3 py-2'} rounded-lg hover:bg-surface-hover transition-colors text-left"
                      >
                        {#if !theme.hidePosters}
                          {#if movie.poster_url}
                            <img src={movie.poster_url} alt="" class="w-8 h-12 rounded object-cover" loading="lazy" decoding="async" />
                          {:else}
                            <div class="w-8 h-12 rounded bg-border flex items-center justify-center">
                              <Film class="w-4 h-4 text-text-muted" />
                            </div>
                          {/if}
                        {/if}
                        <span class="flex-1 text-sm truncate">{movie.title}</span>
                      </button>
                    </li>
                  {/each}
                </ul>
              </div>
            {/each}
          </div>
        {/if}
      {/if}
    {/if}
  </div>

  <!-- Footer -->
  <div class="p-3 border-t border-border {isSidebarCollapsed() ? 'px-2' : ''}">
    {#if activeTab === "shows" || activeTab === "movies"}
      <button
        type="button"
        onclick={handleAddClick}
        class="w-full flex items-center justify-center gap-2 {isSidebarCollapsed() ? 'p-2' : 'px-4 py-2.5'} bg-accent hover:bg-accent-hover text-white rounded-lg transition-colors font-medium"
        title={activeTab === "shows" ? "Add Show" : "Add Movie"}
      >
        <Plus class="w-4 h-4" />
        {#if !isSidebarCollapsed()}
          {activeTab === "shows" ? "Add Show" : "Add Movie"}
        {/if}
      </button>
    {/if}
    <div class="flex {isSidebarCollapsed() ? 'flex-col' : ''} gap-2 {(activeTab === 'shows' || activeTab === 'movies') ? 'mt-2' : ''}">
      <button
        type="button"
        onclick={openStatisticsModal}
        class="flex-1 flex items-center justify-center gap-1.5 px-2 py-1.5 text-xs text-text-muted hover:text-text hover:bg-surface-hover rounded transition-colors"
        aria-label="Statistics"
        title="Statistics"
      >
        <BarChart3 class="w-3 h-3" />
        {#if !isSidebarCollapsed()}
          Stats
        {/if}
      </button>
      <button
        type="button"
        onclick={openDataManagement}
        class="flex-1 flex items-center justify-center gap-1.5 px-2 py-1.5 text-xs text-text-muted hover:text-text hover:bg-surface-hover rounded transition-colors"
        aria-label="Data Management"
        title="Data Management"
      >
        <Database class="w-3 h-3" />
        {#if !isSidebarCollapsed()}
          Data
        {/if}
      </button>
    </div>
    {#if !isSidebarCollapsed()}
      <button
        type="button"
        onclick={triggerUpdateCheck}
        disabled={isCheckingForUpdates()}
        class="w-full flex items-center justify-center gap-2 mt-1 px-3 py-1.5 text-xs text-text-muted hover:text-text hover:bg-surface-hover rounded transition-colors disabled:opacity-50"
      >
        <RefreshCw class="w-3 h-3 {isCheckingForUpdates() ? 'animate-spin' : ''}" />
        {isCheckingForUpdates() ? "Checking..." : "Check for Updates"}
      </button>
      <p class="text-xs text-text-muted text-center mt-2">v0.7.1</p>
    {/if}
  </div>
</aside>
