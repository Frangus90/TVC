<script lang="ts">
  import { Plus, Tv, Trash2, RefreshCw, Check, Film, Archive, RotateCcw, CalendarX, BarChart3, Database, PanelLeftClose, PanelLeft } from "lucide-svelte";
  import { onMount } from "svelte";
  import {
    getTrackedShows,
    loadTrackedShows,
    openSearchModal,
    removeShow,
    getArchivedShows,
    loadArchivedShows,
    unarchiveShow,
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
  } from "../../stores/movies.svelte";
  import { openShowDetail } from "../../stores/showDetail.svelte";
  import {
    triggerUpdateCheck,
    isCheckingForUpdates,
  } from "../../stores/updates.svelte";
  import { openStatisticsModal } from "../../stores/statistics.svelte";
  import { openDataManagement } from "../../stores/dataManagement.svelte";
  import { isSidebarCollapsed, toggleSidebar } from "../../stores/sidebar.svelte";

  type SidebarTab = "shows" | "movies" | "archive";
  let activeTab = $state<SidebarTab>("shows");
  let selectedItems = $state<Set<number>>(new Set());
  let bulkMode = $state(false);

  onMount(() => {
    loadTrackedShows();
    loadTrackedMovies();
    loadArchivedShows();
    loadArchivedMovies();
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
    if (confirm(`Are you sure you want to remove ${selectedItems.size} ${itemType}${selectedItems.size > 1 ? 's' : ''}?`)) {
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
      class="flex-1 flex items-center justify-center gap-1.5 px-2 py-2.5 text-xs font-medium transition-colors
        {activeTab === 'shows' ? 'text-accent border-b-2 border-accent bg-accent/5' : 'text-text-muted hover:text-text hover:bg-surface-hover'}
        {isSidebarCollapsed() ? 'border-b-0 border-l-2' : ''}"
      title="TV Shows"
    >
      <Tv class="w-3.5 h-3.5" />
      {#if !isSidebarCollapsed()}
        <span>TV Shows</span>
      {/if}
    </button>
    <button
      type="button"
      onclick={() => switchTab("movies")}
      class="flex-1 flex items-center justify-center gap-1.5 px-2 py-2.5 text-xs font-medium transition-colors
        {activeTab === 'movies' ? 'text-accent border-b-2 border-accent bg-accent/5' : 'text-text-muted hover:text-text hover:bg-surface-hover'}
        {isSidebarCollapsed() ? 'border-b-0 border-l-2' : ''}"
      title="Movies"
    >
      <Film class="w-3.5 h-3.5" />
      {#if !isSidebarCollapsed()}
        <span>Movies</span>
      {/if}
    </button>
    <button
      type="button"
      onclick={() => switchTab("archive")}
      class="flex-1 flex items-center justify-center gap-1.5 px-2 py-2.5 text-xs font-medium transition-colors
        {activeTab === 'archive' ? 'text-accent border-b-2 border-accent bg-accent/5' : 'text-text-muted hover:text-text hover:bg-surface-hover'}
        {isSidebarCollapsed() ? 'border-b-0 border-l-2' : ''}"
      title="Archive"
    >
      <Archive class="w-3.5 h-3.5" />
      {#if !isSidebarCollapsed()}
        <span>Archive</span>
      {/if}
    </button>
  </div>

  <!-- Selection Controls Row (only for shows/movies tabs, hidden when collapsed) -->
  {#if activeTab !== "archive" && !isSidebarCollapsed()}
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
      {#if getTrackedShows().length === 0}
        <p class="text-sm text-text-muted py-4 text-center">
          No shows tracked yet.
          <br />
          Click "Add Show" to get started.
        </p>
      {:else}
        <ul class="space-y-1">
          {#each getTrackedShows() as show}
            {@const isSelected = selectedItems.has(show.id)}
            <li>
              {#if bulkMode}
                <button
                  type="button"
                  onclick={() => toggleItemSelection(show.id)}
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
    {/if}

    <!-- Movies Tab -->
    {#if activeTab === "movies"}
      {#if getTrackedMovies().length === 0}
        <p class="text-sm text-text-muted py-4 text-center">
          No movies tracked yet.
          <br />
          Click "Add Movie" to get started.
        </p>
      {:else}
        <ul class="space-y-1">
          {#each getTrackedMovies() as movie}
            {@const isSelected = selectedItems.has(movie.id)}
            <li>
              {#if bulkMode}
                <button
                  type="button"
                  onclick={() => toggleItemSelection(movie.id)}
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
                  {#if movie.poster_url}
                    <img src={movie.poster_url} alt="" class="w-8 h-12 rounded object-cover" loading="lazy" decoding="async" />
                  {:else}
                    <div class="w-8 h-12 rounded bg-border flex items-center justify-center">
                      <Film class="w-4 h-4 text-text-muted" />
                    </div>
                  {/if}
                  <span class="flex-1 text-sm truncate">{movie.title}</span>
                </button>
              {:else}
                <div class="group w-full flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-surface-hover transition-colors">
                  <button
                    type="button"
                    class="flex-1 flex items-center gap-3 text-left"
                    onclick={() => openMovieDetail(movie.id)}
                  >
                    {#if movie.poster_url}
                      <img src={movie.poster_url} alt="" class="w-8 h-12 rounded object-cover" loading="lazy" decoding="async" />
                    {:else}
                      <div class="w-8 h-12 rounded bg-border flex items-center justify-center">
                        <Film class="w-4 h-4 text-text-muted" />
                      </div>
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
      {#if archiveItems.length === 0}
        <p class="text-sm text-text-muted py-4 text-center">
          No archived items.
          <br />
          Archive shows or movies to see them here.
        </p>
      {:else}
        <ul class="space-y-1">
          {#each archiveItems as item}
            <li>
              <div class="group w-full flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-surface-hover transition-colors">
                <div class="flex-1 flex items-center gap-3">
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
                    <span class="text-xs text-text-muted">
                      {item.type === "show" ? "TV Show" : "Movie"}
                    </span>
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
  </div>

  <!-- Footer -->
  <div class="p-3 border-t border-border {isSidebarCollapsed() ? 'px-2' : ''}">
    {#if activeTab !== "archive"}
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
    <div class="flex {isSidebarCollapsed() ? 'flex-col' : ''} gap-2 {activeTab !== 'archive' ? 'mt-2' : ''}">
      <button
        type="button"
        onclick={openStatisticsModal}
        class="flex-1 flex items-center justify-center gap-1.5 px-2 py-1.5 text-xs text-text-muted hover:text-text hover:bg-surface-hover rounded transition-colors"
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
      <p class="text-xs text-text-muted text-center mt-2">v0.6.5</p>
    {/if}
  </div>
</aside>
