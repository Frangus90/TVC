<script lang="ts">
  import {
    startOfWeek,
    endOfWeek,
    eachDayOfInterval,
    format,
    isToday,
    parseISO,
    isSameDay,
  } from "date-fns";
  import { Check, Plus, Calendar } from "lucide-svelte";
  import { getCurrentDate } from "../../stores/calendar.svelte";
  import {
    getCalendarEpisodes,
    loadEpisodesForRange,
    toggleEpisodeWatched,
    getTrackedShows,
    openEpisodePicker,
    unscheduleEpisode,
    openDayDetail,
    type Episode,
  } from "../../stores/shows.svelte";

  let showPickerOpen = $state(false);
  let showPickerDate = $state<string | null>(null);

  // Load episodes when week changes
  $effect(() => {
    const weekStart = startOfWeek(getCurrentDate(), { weekStartsOn: 1 });
    const weekEnd = endOfWeek(getCurrentDate(), { weekStartsOn: 1 });
    loadEpisodesForRange(format(weekStart, "yyyy-MM-dd"), format(weekEnd, "yyyy-MM-dd"));
  });

  let weekDays = $derived(() => {
    const weekStart = startOfWeek(getCurrentDate(), { weekStartsOn: 1 });
    const weekEnd = endOfWeek(getCurrentDate(), { weekStartsOn: 1 });
    return eachDayOfInterval({ start: weekStart, end: weekEnd });
  });

  function getEpisodesForDay(day: Date): Episode[] {
    const dayStr = format(day, "yyyy-MM-dd");
    return getCalendarEpisodes().filter((ep) => {
      const displayDate = ep.scheduled_date || ep.aired;
      if (!displayDate) return false;
      return isSameDay(parseISO(displayDate), day);
    });
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

  function handleAddClick(day: Date) {
    showPickerDate = format(day, "yyyy-MM-dd");
    showPickerOpen = true;
  }

  function handleDayClick(day: Date) {
    openDayDetail(format(day, "yyyy-MM-dd"));
  }

  function closeShowPicker() {
    showPickerOpen = false;
    showPickerDate = null;
  }
</script>

<div class="h-full flex flex-col">
  <!-- Week grid - 7 columns -->
  <div class="flex-1 grid grid-cols-7 gap-2">
    {#each weekDays() as day}
      {@const today = isToday(day)}
      {@const dayEpisodes = getEpisodesForDay(day)}
      <div class="flex flex-col border border-border rounded-lg overflow-hidden bg-surface">
        <!-- Day header -->
        <div class="p-3 border-b border-border flex items-center justify-between {today ? 'bg-accent/10' : ''}">
          <button
            onclick={() => handleDayClick(day)}
            class="text-center hover:bg-surface-hover rounded-lg px-2 py-1 transition-colors"
          >
            <div class="text-xs text-text-muted uppercase">{format(day, "EEE")}</div>
            <div class="text-xl font-semibold {today ? 'text-accent' : 'text-text'}">
              {format(day, "d")}
            </div>
          </button>
          <button
            onclick={() => handleAddClick(day)}
            class="p-1.5 rounded hover:bg-surface-hover transition-colors"
            aria-label="Schedule episode"
          >
            <Plus class="w-4 h-4 text-text-muted" />
          </button>
        </div>

        <!-- Episodes for this day -->
        <div class="flex-1 p-2 space-y-2 overflow-auto">
          {#each dayEpisodes as episode}
            {@const isScheduled = !!episode.scheduled_date}
            <button
              onclick={(e) => handleToggleWatched(e, episode)}
              oncontextmenu={(e) => { e.preventDefault(); handleUnschedule(e, episode); }}
              class="w-full text-left p-2 rounded-lg text-sm transition-colors {episode.watched
                ? 'bg-watched/20 text-watched'
                : isScheduled
                  ? 'bg-premiere/20 text-premiere hover:bg-premiere/30'
                  : 'bg-upcoming/20 text-upcoming hover:bg-upcoming/30'}"
              title={isScheduled ? "Right-click to unschedule" : "Click to toggle watched"}
            >
              <div class="flex items-center gap-1.5 mb-1">
                {#if episode.watched}
                  <Check class="w-3.5 h-3.5 flex-shrink-0" />
                {:else if isScheduled}
                  <Calendar class="w-3.5 h-3.5 flex-shrink-0" />
                {/if}
                <span class="font-medium truncate">{episode.show_name}</span>
              </div>
              <div class="text-xs opacity-75 {episode.watched ? 'line-through' : ''}">
                S{String(episode.season_number).padStart(2, "0")}E{String(episode.episode_number).padStart(2, "0")}
                {#if episode.name}
                  - {episode.name}
                {/if}
              </div>
            </button>
          {:else}
            <div class="text-xs text-text-muted text-center py-4">No episodes</div>
          {/each}
        </div>
      </div>
    {/each}
  </div>
</div>

<!-- Show picker for scheduling -->
{#if showPickerOpen && showPickerDate}
  <button
    class="fixed inset-0 bg-black/40 z-40"
    onclick={closeShowPicker}
    aria-label="Close"
  ></button>
  <div class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 bg-surface rounded-xl border border-border shadow-2xl p-5 w-[500px] max-w-[90vw]">
    <h3 class="font-semibold text-lg mb-4">Schedule episode for {showPickerDate}</h3>
    {#if getTrackedShows().length === 0}
      <p class="text-text-muted">No shows tracked yet</p>
    {:else}
      <p class="text-text-muted mb-3">Select a show:</p>
      <ul class="space-y-2 max-h-[500px] overflow-auto">
        {#each getTrackedShows() as show}
          <li>
            <button
              onclick={() => { const date = showPickerDate!; closeShowPicker(); openEpisodePicker(show, date); }}
              class="w-full flex items-center gap-4 p-3 rounded-lg hover:bg-surface-hover transition-colors text-left"
            >
              {#if show.poster_url}
                <img src={show.poster_url} alt="" class="w-12 h-[72px] rounded object-cover flex-shrink-0" />
              {:else}
                <div class="w-12 h-[72px] rounded bg-border flex-shrink-0"></div>
              {/if}
              <span class="font-medium">{show.name}</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
    <button
      onclick={closeShowPicker}
      class="mt-4 w-full py-2.5 text-text-muted hover:bg-surface-hover rounded-lg transition-colors"
    >
      Cancel
    </button>
  </div>
{/if}
