<script lang="ts">
  import { X, Check, ChevronDown, ChevronRight, Plus } from "lucide-svelte";
  import {
    isEpisodePickerOpen,
    closeEpisodePicker,
    getEpisodePickerShow,
    getEpisodePickerEpisodes,
    getEpisodePickerDate,
    scheduleEpisode,
    scheduleMultipleEpisodes,
    type ShowEpisode,
  } from "../stores/shows.svelte";

  // Local UI state
  let searchQuery = $state("");
  let filterWatched = $state<"all" | "watched" | "unwatched">("all");
  let filterSeason = $state<number | null>(null);

  // Track expanded seasons
  let expandedSeasons = $state<Set<number>>(new Set());

  // Track selected episodes for multi-select
  let selectedEpisodes = $state<Set<number>>(new Set());

  // Episode preview tooltip state
  let previewEpisode = $state<ShowEpisode | null>(null);
  let previewPosition = $state<{ x: number; y: number } | null>(null);

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      closeEpisodePicker();
    }
  }

  function handleEpisodeClick(event: MouseEvent, episode: ShowEpisode) {
    const date = getEpisodePickerDate();
    if (!date) return;

    if (event.ctrlKey || event.metaKey) {
      // CTRL+click: toggle selection
      const newSelected = new Set(selectedEpisodes);
      if (newSelected.has(episode.id)) {
        newSelected.delete(episode.id);
      } else {
        newSelected.add(episode.id);
      }
      selectedEpisodes = newSelected;
    } else {
      // Regular click: schedule immediately (if no selection) or add to selection
      if (selectedEpisodes.size === 0) {
        scheduleEpisode(episode.id, date);
      } else {
        // If there's already a selection, toggle this one too
        const newSelected = new Set(selectedEpisodes);
        if (newSelected.has(episode.id)) {
          newSelected.delete(episode.id);
        } else {
          newSelected.add(episode.id);
        }
        selectedEpisodes = newSelected;
      }
    }
  }

  async function handleScheduleSelected() {
    const date = getEpisodePickerDate();
    if (date && selectedEpisodes.size > 0) {
      await scheduleMultipleEpisodes([...selectedEpisodes], date);
      selectedEpisodes = new Set();
    }
  }

  async function handleScheduleSeason(event: MouseEvent, seasonEpisodes: ShowEpisode[]) {
    event.stopPropagation();
    const date = getEpisodePickerDate();
    if (date) {
      const episodeIds = seasonEpisodes.map(ep => ep.id);
      await scheduleMultipleEpisodes(episodeIds, date);
      selectedEpisodes = new Set();
    }
  }

  function toggleSeason(season: number) {
    const newExpanded = new Set(expandedSeasons);
    if (newExpanded.has(season)) {
      newExpanded.delete(season);
    } else {
      newExpanded.add(season);
    }
    expandedSeasons = newExpanded;
  }

  function expandAll(grouped: Map<number, ShowEpisode[]>) {
    expandedSeasons = new Set(grouped.keys());
  }

  function collapseAll() {
    expandedSeasons = new Set();
  }

  function clearSelection() {
    selectedEpisodes = new Set();
  }

  // Group episodes by season
  function groupBySeason(episodes: ShowEpisode[]): Map<number, ShowEpisode[]> {
    const grouped = new Map<number, ShowEpisode[]>();
    for (const ep of episodes) {
      const season = ep.season_number ?? 0;
      if (!grouped.has(season)) {
        grouped.set(season, []);
      }
      grouped.get(season)!.push(ep);
    }
    return grouped;
  }

  // Format episode number safely
  function formatEpisodeNumber(num: number | null): string {
    if (num === null || num === undefined) return "??";
    return String(num).padStart(2, "0");
  }

  // Get available seasons for filter
  function getAvailableSeasons(): number[] {
    const seasons = new Set<number>();
    for (const ep of getEpisodePickerEpisodes()) {
      seasons.add(ep.season_number ?? 0);
    }
    return Array.from(seasons).sort((a, b) => a - b);
  }

  // Filter episodes based on current filters (memoized)
  let filteredEpisodes = $derived.by(() => {
    let episodes = getEpisodePickerEpisodes();

    // Filter by watched status
    if (filterWatched === "watched") {
      episodes = episodes.filter((ep) => ep.watched);
    } else if (filterWatched === "unwatched") {
      episodes = episodes.filter((ep) => !ep.watched);
    }

    // Filter by season
    if (filterSeason !== null) {
      episodes = episodes.filter((ep) => (ep.season_number ?? 0) === filterSeason);
    }

    // Filter by search query
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      episodes = episodes.filter(
        (ep) =>
          ep.name?.toLowerCase().includes(query) ||
          `s${String(ep.season_number ?? 0).padStart(2, "0")}e${String(ep.episode_number ?? 0).padStart(2, "0")}`.includes(query)
      );
    }

    return episodes;
  });

  function handleSelectAll() {
    const newSelected = new Set(selectedEpisodes);
    for (const ep of filteredEpisodes) {
      newSelected.add(ep.id);
    }
    selectedEpisodes = newSelected;
  }

  function handleSelectNone() {
    const newSelected = new Set(selectedEpisodes);
    for (const ep of filteredEpisodes) {
      newSelected.delete(ep.id);
    }
    selectedEpisodes = newSelected;
  }

  async function handleScheduleNextN(n: number) {
    const date = getEpisodePickerDate();
    if (!date) return;

    const unwatched = getEpisodePickerEpisodes()
      .filter((ep) => !ep.watched && !ep.scheduled_date)
      .slice(0, n);

    if (unwatched.length > 0) {
      const episodeIds = unwatched.map((ep) => ep.id);
      await scheduleMultipleEpisodes(episodeIds, date);
      selectedEpisodes = new Set();
    }
  }
</script>

<svelte:window on:keydown={handleKeyDown} />

{#if isEpisodePickerOpen()}
  {@const show = getEpisodePickerShow()}
  {@const date = getEpisodePickerDate()}
  {@const grouped = groupBySeason(filteredEpisodes)}
  {@const sortedSeasons = [...grouped.entries()].sort((a, b) => a[0] - b[0])}
  {@const availableSeasons = getAvailableSeasons()}

  <!-- Backdrop -->
  <button
    class="fixed inset-0 bg-black/60 z-50"
    onclick={closeEpisodePicker}
    aria-label="Close modal"
  ></button>

  <!-- Modal -->
  <div
    class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-full max-w-4xl z-50 bg-surface rounded-xl border border-border shadow-2xl max-h-[80vh] flex flex-col"
  >
    <!-- Header -->
    <div class="flex items-center justify-between p-4 border-b border-border">
      <div>
        <h2 class="font-semibold text-text">Schedule Episodes</h2>
        <p class="text-sm text-text-muted">
          {show?.name} - {date}
        </p>
      </div>
      <button
        onclick={closeEpisodePicker}
        class="p-1.5 rounded-lg hover:bg-surface-hover transition-colors"
        aria-label="Close"
      >
        <X class="w-5 h-5 text-text-muted" />
      </button>
    </div>

    <!-- Filters and controls -->
    <div class="px-4 py-3 border-b border-border space-y-3">
      <!-- Search and filters row -->
      <div class="flex items-center gap-2">
        <input
          type="text"
          placeholder="Search episodes..."
          bind:value={searchQuery}
          class="flex-1 px-3 py-1.5 text-sm rounded border border-border bg-surface text-text placeholder:text-text-muted outline-none focus:ring-2 focus:ring-accent"
        />
        <select
          bind:value={filterWatched}
          class="px-2 py-1.5 text-sm rounded border border-border bg-surface text-text outline-none focus:ring-2 focus:ring-accent"
        >
          <option value="all">All</option>
          <option value="watched">Watched</option>
          <option value="unwatched">Unwatched</option>
        </select>
        <select
          bind:value={filterSeason}
          class="px-2 py-1.5 text-sm rounded border border-border bg-surface text-text outline-none focus:ring-2 focus:ring-accent"
        >
          <option value={null}>All Seasons</option>
          {#each availableSeasons as season}
            <option value={season}>{season === 0 ? "Specials" : `Season ${season}`}</option>
          {/each}
        </select>
      </div>

      <!-- Action buttons row -->
      <div class="flex items-center justify-between text-xs">
        <div class="flex gap-2">
          {#if sortedSeasons.length > 1}
            <button
              onclick={() => expandAll(grouped)}
              class="text-accent hover:underline"
            >
              Expand All
            </button>
            <span class="text-text-muted">|</span>
            <button
              onclick={collapseAll}
              class="text-accent hover:underline"
            >
              Collapse All
            </button>
          {/if}
        </div>
        <div class="flex items-center gap-2">
          <button
            onclick={handleSelectAll}
            class="text-accent hover:underline"
          >
            Select All
          </button>
          <span class="text-text-muted">|</span>
          <button
            onclick={handleSelectNone}
            class="text-accent hover:underline"
          >
            Select None
          </button>
          <span class="text-text-muted">|</span>
          <button
            onclick={() => handleScheduleNextN(5)}
            class="text-accent hover:underline"
            title="Schedule next 5 unwatched episodes"
          >
            Next 5
          </button>
        </div>
      </div>
    </div>

    <!-- Episodes list -->
    <div class="flex-1 overflow-auto">
      {#if filteredEpisodes.length === 0}
        <p class="text-text-muted text-center py-8">
          {#if searchQuery.trim() || filterWatched !== "all" || filterSeason !== null}
            No episodes match the current filters
          {:else}
            No episodes found
          {/if}
        </p>
      {:else}
        {#each sortedSeasons as [season, seasonEpisodes]}
          {@const isExpanded = expandedSeasons.has(season)}
          {@const watchedCount = seasonEpisodes.filter(e => e.watched).length}
          {@const selectedInSeason = seasonEpisodes.filter(e => selectedEpisodes.has(e.id)).length}
          <div class="border-b border-border last:border-b-0">
            <!-- Season header (clickable) -->
            <div class="flex items-center hover:bg-surface-hover transition-colors">
              <button
                onclick={() => toggleSeason(season)}
                class="flex-1 flex items-center gap-2 p-3 text-left"
              >
                {#if isExpanded}
                  <ChevronDown class="w-4 h-4 text-text-muted" />
                {:else}
                  <ChevronRight class="w-4 h-4 text-text-muted" />
                {/if}
                <span class="font-medium text-text">
                  {season === 0 ? "Specials" : `Season ${season}`}
                </span>
                <span class="text-xs text-text-muted">
                  {watchedCount}/{seasonEpisodes.length} watched
                  {#if selectedInSeason > 0}
                    <span class="text-accent ml-1">({selectedInSeason} selected)</span>
                  {/if}
                </span>
              </button>
              <!-- Add season button -->
              <button
                onclick={(e) => handleScheduleSeason(e, seasonEpisodes)}
                class="p-2 mr-2 rounded-lg hover:bg-accent/20 text-accent transition-colors"
                title="Schedule entire {season === 0 ? 'Specials' : `Season ${season}`}"
              >
                <Plus class="w-4 h-4" />
              </button>
            </div>

            <!-- Episodes (collapsible) -->
            {#if isExpanded}
              <ul class="pb-2">
                {#each seasonEpisodes as episode}
                  {@const isSelected = selectedEpisodes.has(episode.id)}
                  {@const showColor = show?.color || null}
                  <li>
                    <button
                      onclick={(e) => handleEpisodeClick(e, episode)}
                      onmouseenter={(e) => {
                        previewEpisode = episode;
                        const rect = e.currentTarget.getBoundingClientRect();
                        const previewWidth = 320; // max-w-xs = 20rem = 320px
                        const previewHeight = 300; // Approximate height
                        const padding = 16;
                        
                        let x = rect.left + rect.width + padding; // Position to the right of the episode item
                        let y = rect.top; // Align with the top of the episode item
                        
                        // Check if preview would go off the right edge
                        if (x + previewWidth > window.innerWidth) {
                          // Position to the left of the episode item instead
                          x = rect.left - previewWidth - padding;
                          // If still off screen, position at the right edge
                          if (x < 0) {
                            x = window.innerWidth - previewWidth - padding;
                          }
                        }
                        
                        // Check if preview would go off the bottom edge
                        if (y + previewHeight > window.innerHeight) {
                          y = window.innerHeight - previewHeight - padding;
                        }
                        
                        // Check if preview would go off the top edge
                        if (y < padding) {
                          y = padding;
                        }
                        
                        previewPosition = { x, y };
                      }}
                      onmouseleave={() => {
                        previewEpisode = null;
                        previewPosition = null;
                      }}
                      class="w-full flex items-center gap-3 px-4 py-2 transition-colors text-left group
                        {isSelected ? 'bg-accent/20' : 'hover:bg-surface-hover'}"
                      style={showColor ? `border-left: 3px solid ${showColor};` : ''}
                    >
                      <!-- Selection indicator -->
                      <div class="w-5 h-5 rounded border flex-shrink-0 flex items-center justify-center
                        {isSelected ? 'bg-accent border-accent' : 'border-border'}">
                        {#if isSelected}
                          <Check class="w-3 h-3 text-white" />
                        {/if}
                      </div>
                      <!-- Episode thumbnail -->
                      {#if episode.image_url}
                        <img
                          src={episode.image_url}
                          alt=""
                          class="w-12 h-[72px] rounded object-cover flex-shrink-0"
                          loading="lazy"
                        />
                      {:else}
                        <div class="w-12 h-[72px] rounded bg-border flex-shrink-0"></div>
                      {/if}
                      <span class="text-sm text-text-muted w-10 flex-shrink-0 font-mono">
                        E{formatEpisodeNumber(episode.episode_number)}
                      </span>
                      <div class="flex-1 min-w-0">
                        <span class="text-sm truncate block {episode.watched ? 'text-text-muted' : 'text-text'}">
                          {episode.name || "TBA"}
                        </span>
                        {#if episode.overview}
                          <p class="text-xs text-text-muted line-clamp-1 mt-0.5">
                            {episode.overview}
                          </p>
                        {/if}
                      </div>
                      <div class="flex items-center gap-2 flex-shrink-0">
                        {#if episode.watched}
                          <Check class="w-4 h-4 text-watched" />
                        {/if}
                        {#if episode.scheduled_date}
                          <span class="text-xs text-premiere bg-premiere/10 px-2 py-0.5 rounded">
                            {episode.scheduled_date}
                          </span>
                        {:else if episode.aired}
                          <span class="text-xs text-text-muted">
                            {episode.aired}
                          </span>
                        {:else}
                          <span class="text-xs text-text-muted italic">TBA</span>
                        {/if}
                      </div>
                    </button>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        {/each}
      {/if}
    </div>

    <!-- Footer with schedule button (shown when episodes are selected) -->
    {#if selectedEpisodes.size > 0}
      <div class="p-4 border-t border-border flex items-center justify-between gap-3">
        <button
          onclick={clearSelection}
          class="px-3 py-2 text-sm text-text-muted hover:bg-surface-hover rounded-lg transition-colors"
        >
          Clear selection
        </button>
        <button
          onclick={handleScheduleSelected}
          class="px-4 py-2 bg-accent hover:bg-accent-hover text-white rounded-lg transition-colors font-medium"
        >
          Schedule {selectedEpisodes.size} episode{selectedEpisodes.size > 1 ? 's' : ''}
        </button>
      </div>
    {/if}
  </div>

  <!-- Episode preview tooltip -->
  {#if previewEpisode && previewPosition}
    <div
      class="fixed z-[60] bg-surface border border-border rounded-lg shadow-xl p-4 max-w-xs pointer-events-none"
      style="left: {previewPosition.x}px; top: {previewPosition.y}px; transform: translateY(0);"
    >
      {#if previewEpisode.image_url}
        <img
          src={previewEpisode.image_url}
          alt=""
          class="w-full h-48 rounded object-cover mb-2"
        />
      {/if}
      <h4 class="font-semibold text-sm mb-1">
        S{String(previewEpisode.season_number ?? 0).padStart(2, "0")}E{formatEpisodeNumber(previewEpisode.episode_number)}
        {#if previewEpisode.name}
          - {previewEpisode.name}
        {/if}
      </h4>
      {#if previewEpisode.overview}
        <p class="text-xs text-text-muted line-clamp-3">{previewEpisode.overview}</p>
      {/if}
      {#if previewEpisode.aired}
        <p class="text-xs text-text-muted mt-2">Aired: {previewEpisode.aired}</p>
      {/if}
    </div>
  {/if}
{/if}

<style>
  .line-clamp-1 {
    display: -webkit-box;
    -webkit-line-clamp: 1;
    -webkit-box-orient: vertical;
    overflow: hidden;
    line-clamp: 1;
  }
  .line-clamp-3 {
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
    line-clamp: 3;
  }
</style>
