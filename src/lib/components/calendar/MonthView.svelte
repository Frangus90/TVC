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
  import { Check, Plus, Film } from "lucide-svelte";
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
  import {
    getCalendarMovies,
    loadMoviesForRange,
    markMovieWatched,
    openMovieDetail,
    getTrackedMovies,
    scheduleMovie,
    type CalendarMovie,
  } from "../../stores/movies.svelte";
  import { Tv } from "lucide-svelte";

  let showPickerOpen = $state(false);
  let showPickerDate = $state<string | null>(null);
  type PickerTab = "shows" | "movies";
  let pickerTab = $state<PickerTab>("shows");

  // Load episodes and movies when month changes
  $effect(() => {
    const monthStart = startOfMonth(getCurrentDate());
    const monthEnd = endOfMonth(getCurrentDate());
    const calendarStart = startOfWeek(monthStart, { weekStartsOn: 1 });
    const calendarEnd = endOfWeek(monthEnd, { weekStartsOn: 1 });

    const startStr = format(calendarStart, "yyyy-MM-dd");
    const endStr = format(calendarEnd, "yyyy-MM-dd");
    loadEpisodesForRange(startStr, endStr);
    loadMoviesForRange(startStr, endStr);
  });

  let calendarDays = $derived.by(() => {
    const monthStart = startOfMonth(getCurrentDate());
    const monthEnd = endOfMonth(getCurrentDate());
    const calendarStart = startOfWeek(monthStart, { weekStartsOn: 1 });
    const calendarEnd = endOfWeek(monthEnd, { weekStartsOn: 1 });

    return eachDayOfInterval({ start: calendarStart, end: calendarEnd });
  });

  function getEpisodesForDay(day: Date): Episode[] {
    return getCalendarEpisodes().filter((ep) => {
      // Check scheduled_date first, then aired
      const displayDate = ep.scheduled_date || ep.aired;
      if (!displayDate) return false;
      return isSameDay(parseISO(displayDate), day);
    });
  }

  function getMoviesForDay(day: Date): CalendarMovie[] {
    return getCalendarMovies().filter((movie) => {
      // Only show movies with a scheduled_date (not digital_release_date)
      if (!movie.scheduled_date) return false;
      return isSameDay(parseISO(movie.scheduled_date), day);
    });
  }

  // Unified calendar item type
  interface CalendarItem {
    type: "episode" | "movie";
    id: number;
    title: string;
    subtitle: string;
    watched: boolean;
    hasAired: boolean;
    color: string | null;
    data: Episode | CalendarMovie;
  }

  function getItemsForDay(day: Date): CalendarItem[] {
    const episodes = getEpisodesForDay(day).map((ep): CalendarItem => {
      const hasAired = ep.aired ? new Date(ep.aired) <= new Date() : false;
      const title = ep.network ? `${ep.show_name} | ${ep.network}` : ep.show_name;
      return {
        type: "episode",
        id: ep.id,
        title,
        subtitle: `S${String(ep.season_number).padStart(2, "0")}E${String(ep.episode_number).padStart(2, "0")}${ep.name ? ` - ${ep.name}` : ""}`,
        watched: ep.watched,
        hasAired,
        color: getShowColor(ep.show_id),
        data: ep,
      };
    });

    const movies = getMoviesForDay(day).map((movie): CalendarItem => {
      // Only use scheduled_date (not digital_release_date)
      const displayDate = movie.scheduled_date;
      const hasReleased = displayDate ? new Date(displayDate) <= new Date() : false;
      return {
        type: "movie",
        id: movie.id,
        title: movie.title,
        subtitle: "",
        watched: movie.watched,
        hasAired: hasReleased,
        color: movie.color,
        data: movie,
      };
    });

    return [...episodes, ...movies];
  }

  function getShowColor(showId: number): string | null {
    const show = getTrackedShows().find((s) => s.id === showId);
    return show?.color || null;
  }



  async function handleUnschedule(event: MouseEvent, episode: Episode) {
    event.stopPropagation();
    if (episode.scheduled_date) {
      await unscheduleEpisode(episode.id);
    }
  }

  async function handleItemClick(item: CalendarItem) {
    if (item.type === "episode") {
      await toggleEpisodeWatched(item.id, !item.watched);
    } else {
      await markMovieWatched(item.id, !item.watched);
    }
  }

  function handleMovieClick(movie: CalendarMovie) {
    openMovieDetail(movie.id);
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
    pickerTab = "shows";
  }

  async function handleScheduleMovie(movieId: number) {
    if (!showPickerDate) return;
    await scheduleMovie(movieId, showPickerDate);
    closeShowPicker();
  }

  const weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
  const weekdaysShort = ["M", "T", "W", "T", "F", "S", "S"];
</script>

<div class="h-full flex flex-col">
  <!-- Weekday headers -->
  <div class="grid grid-cols-7 border-b border-border">
    {#each weekdays as day, i}
      <div class="py-2 text-center text-sm font-medium text-text-muted">
        <span class="hidden sm:inline">{day}</span>
        <span class="sm:hidden">{weekdaysShort[i]}</span>
      </div>
    {/each}
  </div>

  <!-- Calendar grid -->
  <div class="flex-1 grid grid-cols-7 auto-rows-fr">
    {#each calendarDays as day}
      {@const isCurrentMonth = isSameMonth(day, getCurrentDate())}
      {@const today = isToday(day)}
      {@const dayItems = getItemsForDay(day)}
      <div
        role="region"
        aria-label="Calendar day {format(day, 'MMMM d, yyyy')}"
        data-date={format(day, "yyyy-MM-dd")}
        class="border-b border-r border-border p-2 min-h-[100px] overflow-hidden group {isCurrentMonth
          ? ''
          : 'bg-background/50'}"
      >
        <div class="flex items-center justify-between mb-1">
          <div class="flex items-center gap-1.5">
            <button
              onclick={() => handleDayClick(day)}
              class="text-sm hover:bg-surface-hover rounded-full transition-colors {today
                ? 'bg-accent text-white w-7 h-7 flex items-center justify-center font-semibold ring-2 ring-accent/50'
                : isCurrentMonth
                  ? 'text-text w-6 h-6 flex items-center justify-center'
                  : 'text-text-muted w-6 h-6 flex items-center justify-center'}"
            >
              {format(day, "d")}
            </button>
          </div>
          <button
            onclick={() => handleAddClick(day)}
            class="p-1 rounded hover:bg-surface-hover opacity-0 group-hover:opacity-100 transition-opacity"
            aria-label="Schedule episode"
          >
            <Plus class="w-4 h-4 text-text-muted" />
          </button>
        </div>

        <!-- Calendar items (episodes + movies) -->
        <div class="space-y-1">
          {#each dayItems.slice(0, 3) as item}
            <div
              role="button"
              tabindex="0"
              onclick={(e) => { e.stopPropagation(); handleItemClick(item); }}
              onkeydown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  handleItemClick(item);
                }
              }}
              ondblclick={() => item.type === "movie" && handleMovieClick(item.data as CalendarMovie)}
              oncontextmenu={(e) => {
                e.preventDefault();
                if (item.type === "episode") {
                  const ep = item.data as Episode;
                  if (ep.scheduled_date) handleUnschedule(e, ep);
                }
              }}
              class="w-full text-left p-1.5 rounded text-xs transition-colors relative group/item {item.watched
                ? 'bg-watched/20 text-watched line-through cursor-default'
                : item.hasAired
                  ? 'bg-premiere/20 text-premiere hover:bg-premiere/30 cursor-pointer'
                  : 'bg-upcoming/20 text-upcoming hover:bg-upcoming/30 cursor-pointer'}"
              style={item.color ? `border-left: 2px solid ${item.color}` : ''}
              title={item.watched ? "Watched" : item.hasAired ? "Click to mark watched" : "Upcoming"}
            >
              <div class="flex items-center gap-1">
                {#if item.watched}
                  <Check class="w-3 h-3 flex-shrink-0" />
                {:else if item.type === "movie"}
                  <Film class="w-3 h-3 flex-shrink-0" />
                {:else}
                  <Tv class="w-3 h-3 flex-shrink-0" />
                {/if}
                <span class="truncate font-medium">{item.title}</span>
              </div>
              {#if item.subtitle}
                <div class="truncate opacity-75">
                  {item.subtitle}
                </div>
              {/if}
            </div>
          {/each}
          {#if dayItems.length > 3}
            <button
              onclick={() => handleDayClick(day)}
              class="w-full text-xs text-accent hover:text-accent/80 text-center py-1 hover:bg-surface-hover rounded transition-colors"
            >
              +{dayItems.length - 3} more
            </button>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>

<!-- Picker for scheduling episodes or movies -->
{#if showPickerOpen && showPickerDate}
  <button
    class="fixed inset-0 bg-black/40 z-40"
    onclick={closeShowPicker}
    aria-label="Close"
  ></button>
  <div class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 bg-surface rounded-xl border border-border shadow-2xl w-[500px] max-w-[90vw]">
    <div class="p-5 pb-0">
      <h3 class="font-semibold text-lg mb-4">Schedule for {showPickerDate}</h3>

      <!-- Tabs -->
      <div class="flex border-b border-border mb-4">
        <button
          type="button"
          onclick={() => pickerTab = "shows"}
          class="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 text-sm font-medium transition-colors
            {pickerTab === 'shows' ? 'text-accent border-b-2 border-accent' : 'text-text-muted hover:text-text'}"
        >
          <Tv class="w-4 h-4" />
          TV Shows
        </button>
        <button
          type="button"
          onclick={() => pickerTab = "movies"}
          class="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 text-sm font-medium transition-colors
            {pickerTab === 'movies' ? 'text-accent border-b-2 border-accent' : 'text-text-muted hover:text-text'}"
        >
          <Film class="w-4 h-4" />
          Movies
        </button>
      </div>
    </div>

    <div class="px-5 max-h-[400px] overflow-auto">
      <!-- TV Shows Tab -->
      {#if pickerTab === "shows"}
        {#if getTrackedShows().length === 0}
          <p class="text-text-muted text-center py-8">No shows tracked yet</p>
        {:else}
          <ul class="space-y-2">
            {#each getTrackedShows() as show}
              <li>
                <button
                  onclick={() => { const date = showPickerDate!; closeShowPicker(); openEpisodePicker(show, date); }}
                  class="w-full flex items-center gap-4 p-3 rounded-lg hover:bg-surface-hover transition-colors text-left"
                >
                  {#if show.poster_url}
                    <img src={show.poster_url} alt="" class="w-12 h-[72px] rounded object-cover flex-shrink-0" />
                  {:else}
                    <div class="w-12 h-[72px] rounded bg-border flex items-center justify-center flex-shrink-0">
                      <Tv class="w-5 h-5 text-text-muted" />
                    </div>
                  {/if}
                  <span class="font-medium">{show.name}</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      {/if}

      <!-- Movies Tab -->
      {#if pickerTab === "movies"}
        {#if getTrackedMovies().length === 0}
          <p class="text-text-muted text-center py-8">No movies tracked yet</p>
        {:else}
          <ul class="space-y-2">
            {#each getTrackedMovies() as movie}
              <li>
                <button
                  onclick={() => handleScheduleMovie(movie.id)}
                  class="w-full flex items-center gap-4 p-3 rounded-lg hover:bg-surface-hover transition-colors text-left"
                >
                  {#if movie.poster_url}
                    <img src={movie.poster_url} alt="" class="w-12 h-[72px] rounded object-cover flex-shrink-0" />
                  {:else}
                    <div class="w-12 h-[72px] rounded bg-border flex items-center justify-center flex-shrink-0">
                      <Film class="w-5 h-5 text-text-muted" />
                    </div>
                  {/if}
                  <div class="flex-1">
                    <span class="font-medium">{movie.title}</span>
                    {#if movie.scheduled_date}
                      <p class="text-xs text-text-muted">Currently scheduled: {movie.scheduled_date}</p>
                    {/if}
                  </div>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      {/if}
    </div>

    <div class="p-5 pt-4">
      <button
        onclick={closeShowPicker}
        class="w-full py-2.5 text-text-muted hover:bg-surface-hover rounded-lg transition-colors"
      >
        Cancel
      </button>
    </div>
  </div>
{/if}
