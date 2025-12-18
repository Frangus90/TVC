<script lang="ts">
  import { format, parseISO } from "date-fns";
  import { X, Check, Calendar, CalendarX, Eye, EyeOff, Trash2 } from "lucide-svelte";
  import {
    isDayDetailOpen,
    getDayDetailDate,
    closeDayDetail,
    getEpisodesForDate,
    toggleEpisodeWatched,
    unscheduleEpisode,
    getTrackedShows,
    type Episode,
  } from "../stores/shows.svelte";

  interface GroupedEpisodes {
    showName: string;
    posterUrl: string | null;
    showId: number;
    showColor: string | null;
    episodes: Episode[];
  }

  let groupedEpisodes = $derived(() => {
    const date = getDayDetailDate();
    if (!date) return [];

    const episodes = getEpisodesForDate(date);

    // Group by show
    const shows = getTrackedShows();
    const groups = new Map<number, GroupedEpisodes>();
    for (const ep of episodes) {
      if (!groups.has(ep.show_id)) {
        const show = shows.find(s => s.id === ep.show_id);
        groups.set(ep.show_id, {
          showName: ep.show_name,
          posterUrl: ep.poster_url,
          showId: ep.show_id,
          showColor: show?.color || null,
          episodes: [],
        });
      }
      groups.get(ep.show_id)!.episodes.push(ep);
    }

    // Sort episodes within each group
    for (const group of groups.values()) {
      group.episodes.sort((a, b) => {
        if (a.season_number !== b.season_number) return a.season_number - b.season_number;
        return a.episode_number - b.episode_number;
      });
    }

    return Array.from(groups.values()).sort((a, b) => a.showName.localeCompare(b.showName));
  });

  function formatDate(dateStr: string): string {
    const date = parseISO(dateStr);
    return format(date, "EEEE, MMMM d, yyyy");
  }

  async function handleToggleWatched(episode: Episode) {
    await toggleEpisodeWatched(episode.id, !episode.watched);
  }

  async function handleUnschedule(episode: Episode) {
    if (episode.scheduled_date) {
      await unscheduleEpisode(episode.id);
    }
  }

  function getTotalEpisodes(): number {
    return groupedEpisodes().reduce((sum, g) => sum + g.episodes.length, 0);
  }

  function getWatchedCount(): number {
    return groupedEpisodes().reduce(
      (sum, g) => sum + g.episodes.filter((e) => e.watched).length,
      0
    );
  }

  function getScheduledCount(): number {
    return groupedEpisodes().reduce(
      (sum, g) => sum + g.episodes.filter((e) => !!e.scheduled_date).length,
      0
    );
  }

  async function handleClearAll() {
    const date = getDayDetailDate();
    if (!date) return;

    const scheduledCount = getScheduledCount();
    if (scheduledCount === 0) return;

    if (confirm(`Are you sure you want to unschedule all ${scheduledCount} episode${scheduledCount !== 1 ? 's' : ''} for this day?`)) {
      const episodes = getEpisodesForDate(date);
      const scheduledEpisodes = episodes.filter(ep => ep.scheduled_date);
      
      // Unschedule all scheduled episodes
      for (const episode of scheduledEpisodes) {
        await unscheduleEpisode(episode.id);
      }
    }
  }
</script>

{#if isDayDetailOpen() && getDayDetailDate()}
  <!-- Backdrop -->
  <button
    class="fixed inset-0 bg-black/50 z-40"
    onclick={closeDayDetail}
    aria-label="Close"
  ></button>

  <!-- Modal -->
  <div class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 bg-surface rounded-xl border border-border shadow-2xl w-[600px] max-w-[95vw] max-h-[85vh] flex flex-col">
    <!-- Header -->
    <div class="flex items-center justify-between p-5 border-b border-border">
      <div>
        <h2 class="text-xl font-semibold">{formatDate(getDayDetailDate()!)}</h2>
        <p class="text-sm text-text-muted mt-1">
          {getTotalEpisodes()} episode{getTotalEpisodes() !== 1 ? 's' : ''}
          {#if getWatchedCount() > 0}
            <span class="text-watched">({getWatchedCount()} watched)</span>
          {/if}
        </p>
      </div>
      <div class="flex items-center gap-2">
        {#if getScheduledCount() > 0}
          <button
            onclick={handleClearAll}
            class="px-3 py-1.5 text-sm bg-red-500/20 hover:bg-red-500/30 text-red-400 rounded-lg transition-colors flex items-center gap-2"
            title="Unschedule all episodes for this day"
          >
            <Trash2 class="w-4 h-4" />
            Clear All
          </button>
        {/if}
        <button
          onclick={closeDayDetail}
          class="p-2 rounded-lg hover:bg-surface-hover transition-colors"
          aria-label="Close"
        >
          <X class="w-5 h-5" />
        </button>
      </div>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-5">
      {#if groupedEpisodes().length === 0}
        <div class="text-center py-10 text-text-muted">
          <Calendar class="w-12 h-12 mx-auto mb-3 opacity-50" />
          <p>No episodes scheduled for this day</p>
        </div>
      {:else}
        <div class="space-y-6">
          {#each groupedEpisodes() as group}
            <div class="bg-background rounded-lg p-4">
              <!-- Show header -->
              <div class="flex items-center gap-4 mb-4">
                {#if group.posterUrl}
                  <img
                    src={group.posterUrl}
                    alt=""
                    class="w-14 h-[84px] rounded object-cover flex-shrink-0"
                  />
                {:else}
                  <div class="w-14 h-[84px] rounded bg-border flex-shrink-0"></div>
                {/if}
                <div>
                  <h3 class="font-semibold text-lg">{group.showName}</h3>
                  <p class="text-sm text-text-muted">
                    {group.episodes.length} episode{group.episodes.length !== 1 ? 's' : ''}
                  </p>
                </div>
              </div>

              <!-- Episodes -->
              <div class="space-y-2">
                {#each group.episodes as episode}
                  {@const isScheduled = !!episode.scheduled_date}
                  <div
                    class="flex items-center gap-3 p-3 rounded-lg transition-colors {episode.watched
                      ? 'bg-watched/10'
                      : isScheduled
                        ? 'bg-premiere/10'
                        : 'bg-upcoming/10'}"
                    style={group.showColor ? `border-left: 3px solid ${group.showColor};` : ''}
                  >
                    <!-- Episode info -->
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center gap-2">
                        {#if episode.watched}
                          <Check class="w-4 h-4 text-watched flex-shrink-0" />
                        {:else if isScheduled}
                          <Calendar class="w-4 h-4 text-premiere flex-shrink-0" />
                        {/if}
                        <span class="font-medium {episode.watched ? 'text-watched line-through' : ''}">
                          S{String(episode.season_number).padStart(2, "0")}E{String(episode.episode_number).padStart(2, "0")}
                        </span>
                      </div>
                      {#if episode.name}
                        <p class="text-sm text-text-muted truncate mt-0.5">{episode.name}</p>
                      {/if}
                    </div>

                    <!-- Actions -->
                    <div class="flex items-center gap-1">
                      <button
                        onclick={() => handleToggleWatched(episode)}
                        class="p-2 rounded-lg hover:bg-surface-hover transition-colors"
                        title={episode.watched ? "Mark as unwatched" : "Mark as watched"}
                      >
                        {#if episode.watched}
                          <EyeOff class="w-4 h-4 text-text-muted" />
                        {:else}
                          <Eye class="w-4 h-4 text-text-muted" />
                        {/if}
                      </button>
                      {#if isScheduled}
                        <button
                          onclick={() => handleUnschedule(episode)}
                          class="p-2 rounded-lg hover:bg-surface-hover transition-colors"
                          title="Remove from schedule"
                        >
                          <CalendarX class="w-4 h-4 text-text-muted" />
                        </button>
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Footer -->
    <div class="p-4 border-t border-border">
      <button
        onclick={closeDayDetail}
        class="w-full py-2.5 text-text-muted hover:bg-surface-hover rounded-lg transition-colors"
      >
        Close
      </button>
    </div>
  </div>
{/if}
