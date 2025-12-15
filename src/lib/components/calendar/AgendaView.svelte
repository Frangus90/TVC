<script lang="ts">
  import { format, parseISO, addDays, isToday, isTomorrow, isPast } from "date-fns";
  import { Check, Tv } from "lucide-svelte";
  import {
    getCalendarEpisodes,
    loadEpisodesForRange,
    toggleEpisodeWatched,
    unscheduleEpisode,
    type Episode,
  } from "../../stores/shows.svelte";

  // Load episodes for the next 60 days
  $effect(() => {
    const today = new Date();
    const futureDate = addDays(today, 60);
    loadEpisodesForRange(format(today, "yyyy-MM-dd"), format(futureDate, "yyyy-MM-dd"));
  });

  // Group episodes by date
  function groupByDate(episodes: Episode[]): Map<string, Episode[]> {
    const grouped = new Map<string, Episode[]>();

    // Sort episodes by display date
    const sorted = [...episodes].sort((a, b) => {
      const dateA = a.scheduled_date || a.aired || "";
      const dateB = b.scheduled_date || b.aired || "";
      return dateA.localeCompare(dateB);
    });

    for (const ep of sorted) {
      const displayDate = ep.scheduled_date || ep.aired;
      if (!displayDate) continue;

      if (!grouped.has(displayDate)) {
        grouped.set(displayDate, []);
      }
      grouped.get(displayDate)!.push(ep);
    }

    return grouped;
  }

  function formatDateHeader(dateStr: string): string {
    const date = parseISO(dateStr);
    if (isToday(date)) return "Today";
    if (isTomorrow(date)) return "Tomorrow";
    return format(date, "EEEE, MMMM d");
  }

  function isDatePast(dateStr: string): boolean {
    return isPast(parseISO(dateStr)) && !isToday(parseISO(dateStr));
  }

  async function handleToggleWatched(event: MouseEvent, episode: Episode) {
    event.stopPropagation();
    await toggleEpisodeWatched(episode.id, !episode.watched);
  }

  async function handleUnschedule(event: MouseEvent, episode: Episode) {
    event.stopPropagation();
    if (episode.scheduled_date) {
      await unscheduleEpisode(episode.id);
    }
  }
</script>

<div class="h-full overflow-auto">
  <div class="max-w-3xl mx-auto py-4 space-y-6">
    {#if getCalendarEpisodes().length === 0}
      <div class="text-center py-12">
        <Tv class="w-12 h-12 text-text-muted mx-auto mb-4" />
        <p class="text-text-muted">No upcoming episodes</p>
        <p class="text-text-muted text-sm mt-1">Add some shows to start tracking!</p>
      </div>
    {:else}
      {#each [...groupByDate(getCalendarEpisodes())] as [dateStr, episodes]}
        {@const isPastDate = isDatePast(dateStr)}
        <div class="space-y-2">
          <!-- Date header -->
          <h2 class="text-sm font-semibold text-text-muted uppercase tracking-wide px-2 {isPastDate ? 'opacity-50' : ''}">
            {formatDateHeader(dateStr)}
          </h2>

          <!-- Episodes for this date -->
          <div class="space-y-2">
            {#each episodes as episode}
              {@const hasAired = episode.aired && new Date(episode.aired) <= new Date()}
              <button
                onclick={(e) => handleToggleWatched(e, episode)}
                oncontextmenu={(e) => { e.preventDefault(); handleUnschedule(e, episode); }}
                class="w-full flex items-center gap-4 p-4 rounded-xl border transition-colors text-left
                  {episode.watched
                    ? 'bg-watched/10 border-watched/30 text-watched'
                    : hasAired
                      ? 'bg-premiere/10 border-premiere/30 hover:bg-premiere/20'
                      : 'bg-surface border-border hover:bg-surface-hover'}"
                title={episode.watched ? "Watched" : hasAired ? "Click to mark watched" : "Upcoming"}
              >
                <!-- Poster -->
                {#if episode.poster_url}
                  <img
                    src={episode.poster_url}
                    alt=""
                    class="w-12 h-18 rounded object-cover flex-shrink-0 {episode.watched ? 'opacity-50' : ''}"
                  />
                {:else}
                  <div class="w-12 h-18 rounded bg-border flex-shrink-0"></div>
                {/if}

                <!-- Episode info -->
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2">
                    <span class="font-semibold truncate {episode.watched ? 'line-through opacity-75' : 'text-text'}">
                      {episode.show_name}
                    </span>
                  </div>
                  <div class="text-sm {episode.watched ? 'opacity-75' : 'text-text-muted'}">
                    <span class="font-mono">
                      S{String(episode.season_number).padStart(2, "0")}E{String(episode.episode_number).padStart(2, "0")}
                    </span>
                    {#if episode.name}
                      <span class="mx-1">-</span>
                      <span class="{episode.watched ? 'line-through' : ''}">{episode.name}</span>
                    {/if}
                  </div>
                </div>

                <!-- Status indicator -->
                <div class="flex-shrink-0">
                  {#if episode.watched}
                    <div class="w-8 h-8 rounded-full bg-watched/20 flex items-center justify-center">
                      <Check class="w-5 h-5 text-watched" />
                    </div>
                  {:else}
                    <div class="w-8 h-8 rounded-full bg-surface-hover flex items-center justify-center opacity-0 group-hover:opacity-100">
                      <Check class="w-5 h-5 text-text-muted" />
                    </div>
                  {/if}
                </div>
              </button>
            {/each}
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>
