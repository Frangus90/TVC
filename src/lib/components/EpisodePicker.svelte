<script lang="ts">
  import { X, Calendar, Check, ChevronDown, ChevronRight, Plus } from "lucide-svelte";
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

  // Track expanded seasons
  let expandedSeasons = $state<Set<number>>(new Set());

  // Track selected episodes for multi-select
  let selectedEpisodes = $state<Set<number>>(new Set());

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
</script>

<svelte:window on:keydown={handleKeyDown} />

{#if isEpisodePickerOpen()}
  {@const show = getEpisodePickerShow()}
  {@const episodes = getEpisodePickerEpisodes()}
  {@const date = getEpisodePickerDate()}
  {@const grouped = groupBySeason(episodes)}
  {@const sortedSeasons = [...grouped.entries()].sort((a, b) => a[0] - b[0])}

  <!-- Backdrop -->
  <button
    class="fixed inset-0 bg-black/60 z-50"
    onclick={closeEpisodePicker}
    aria-label="Close modal"
  ></button>

  <!-- Modal -->
  <div
    class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-full max-w-lg z-50 bg-surface rounded-xl border border-border shadow-2xl max-h-[80vh] flex flex-col"
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

    <!-- Expand/Collapse controls + hint -->
    <div class="flex items-center justify-between px-4 py-2 border-b border-border text-xs">
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
      <span class="text-text-muted">Ctrl+click to multi-select</span>
    </div>

    <!-- Episodes list -->
    <div class="flex-1 overflow-auto">
      {#if episodes.length === 0}
        <p class="text-text-muted text-center py-8">No episodes found</p>
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
                  <li>
                    <button
                      onclick={(e) => handleEpisodeClick(e, episode)}
                      class="w-full flex items-center gap-3 px-4 py-2 transition-colors text-left group
                        {isSelected ? 'bg-accent/20' : 'hover:bg-surface-hover'}"
                    >
                      <!-- Selection indicator -->
                      <div class="w-5 h-5 rounded border flex-shrink-0 flex items-center justify-center
                        {isSelected ? 'bg-accent border-accent' : 'border-border'}">
                        {#if isSelected}
                          <Check class="w-3 h-3 text-white" />
                        {/if}
                      </div>
                      <span class="text-sm text-text-muted w-10 flex-shrink-0 font-mono">
                        E{formatEpisodeNumber(episode.episode_number)}
                      </span>
                      <span class="flex-1 text-sm truncate {episode.watched ? 'text-text-muted' : 'text-text'}">
                        {episode.name || "TBA"}
                      </span>
                      <div class="flex items-center gap-2">
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
{/if}
