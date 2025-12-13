<script lang="ts">
  import {
    startOfMonth,
    endOfMonth,
    startOfWeek,
    endOfWeek,
    eachDayOfInterval,
    format,
    isToday,
    isSameMonth,
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

  // Load episodes when month changes
  $effect(() => {
    const monthStart = startOfMonth(getCurrentDate());
    const monthEnd = endOfMonth(getCurrentDate());
    const calendarStart = startOfWeek(monthStart, { weekStartsOn: 1 });
    const calendarEnd = endOfWeek(monthEnd, { weekStartsOn: 1 });

    loadEpisodesForRange(format(calendarStart, "yyyy-MM-dd"), format(calendarEnd, "yyyy-MM-dd"));
  });

  let calendarDays = $derived(() => {
    const monthStart = startOfMonth(getCurrentDate());
    const monthEnd = endOfMonth(getCurrentDate());
    const calendarStart = startOfWeek(monthStart, { weekStartsOn: 1 });
    const calendarEnd = endOfWeek(monthEnd, { weekStartsOn: 1 });

    return eachDayOfInterval({ start: calendarStart, end: calendarEnd });
  });

  function getEpisodesForDay(day: Date): Episode[] {
    const dayStr = format(day, "yyyy-MM-dd");
    return getCalendarEpisodes().filter((ep) => {
      // Check scheduled_date first, then aired
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

  const weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
</script>

<div class="h-full flex flex-col">
  <!-- Weekday headers -->
  <div class="grid grid-cols-7 border-b border-border">
    {#each weekdays as day}
      <div class="py-2 text-center text-sm font-medium text-text-muted">
        {day}
      </div>
    {/each}
  </div>

  <!-- Calendar grid -->
  <div class="flex-1 grid grid-cols-7 auto-rows-fr">
    {#each calendarDays() as day}
      {@const isCurrentMonth = isSameMonth(day, getCurrentDate())}
      {@const today = isToday(day)}
      {@const dayEpisodes = getEpisodesForDay(day)}
      <div
        class="border-b border-r border-border p-2 min-h-[100px] overflow-hidden group {isCurrentMonth
          ? ''
          : 'bg-background/50'}"
      >
        <div class="flex items-center justify-between mb-1">
          <button
            onclick={() => handleDayClick(day)}
            class="text-sm hover:bg-surface-hover rounded-full transition-colors {today
              ? 'bg-accent text-white w-6 h-6 flex items-center justify-center'
              : isCurrentMonth
                ? 'text-text w-6 h-6 flex items-center justify-center'
                : 'text-text-muted w-6 h-6 flex items-center justify-center'}"
          >
            {format(day, "d")}
          </button>
          <button
            onclick={() => handleAddClick(day)}
            class="p-1 rounded hover:bg-surface-hover opacity-0 group-hover:opacity-100 transition-opacity"
            aria-label="Schedule episode"
          >
            <Plus class="w-4 h-4 text-text-muted" />
          </button>
        </div>

        <!-- Episode cards -->
        <div class="space-y-1">
          {#each dayEpisodes.slice(0, 3) as episode}
            {@const isScheduled = !!episode.scheduled_date}
            <button
              onclick={(e) => handleToggleWatched(e, episode)}
              oncontextmenu={(e) => { e.preventDefault(); handleUnschedule(e, episode); }}
              class="w-full text-left p-1.5 rounded text-xs transition-colors relative group/ep {episode.watched
                ? 'bg-watched/20 text-watched line-through'
                : isScheduled
                  ? 'bg-premiere/20 text-premiere hover:bg-premiere/30'
                  : 'bg-upcoming/20 text-upcoming hover:bg-upcoming/30'}"
              title={isScheduled ? "Right-click to unschedule" : "Click to toggle watched"}
            >
              <div class="flex items-center gap-1">
                {#if episode.watched}
                  <Check class="w-3 h-3 flex-shrink-0" />
                {:else if isScheduled}
                  <Calendar class="w-3 h-3 flex-shrink-0" />
                {/if}
                <span class="truncate font-medium">{episode.show_name}</span>
              </div>
              <div class="truncate opacity-75">
                S{String(episode.season_number).padStart(2, "0")}E{String(episode.episode_number).padStart(2, "0")}
                {#if episode.name}
                  - {episode.name}
                {/if}
              </div>
            </button>
          {/each}
          {#if dayEpisodes.length > 3}
            <button
              onclick={() => handleDayClick(day)}
              class="w-full text-xs text-accent hover:text-accent/80 text-center py-1 hover:bg-surface-hover rounded transition-colors"
            >
              +{dayEpisodes.length - 3} more
            </button>
          {/if}
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
