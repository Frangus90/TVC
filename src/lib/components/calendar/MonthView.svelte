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
  import { Check, Plus, Film, Clock, PlayCircle } from "lucide-svelte";
  import { getCurrentDate, getCalendarFilter } from "../../stores/calendar.svelte";
  import { getNow } from "../../stores/now.svelte";
  import { logger } from "../../utils/logger";
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
  import { getThemeSettings } from "../../stores/theme.svelte";
  import { formatDate } from "../../utils/dateFormat";
  import { openShowDetail } from "../../stores/showDetail.svelte";
  import WatchedBadge from "../common/WatchedBadge.svelte";

  let showPickerOpen = $state(false);
  let showPickerDate = $state<string | null>(null);
  type PickerTab = "shows" | "movies";
  let pickerTab = $state<PickerTab>("shows");

  // Load episodes and movies when month changes (in parallel for better performance)
  $effect(() => {
    const monthStart = startOfMonth(getCurrentDate());
    const monthEnd = endOfMonth(getCurrentDate());
    const calendarStart = startOfWeek(monthStart, { weekStartsOn: 1 });
    const calendarEnd = endOfWeek(monthEnd, { weekStartsOn: 1 });

    const startStr = format(calendarStart, "yyyy-MM-dd");
    const endStr = format(calendarEnd, "yyyy-MM-dd");
    // Load episodes and movies in parallel
    Promise.all([
      loadEpisodesForRange(startStr, endStr),
      loadMoviesForRange(startStr, endStr),
    ]).catch((error) => {
      logger.error("Failed to load calendar data", error);
    });
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
    const filter = getCalendarFilter();

    const episodes = filter === "movies" ? [] : getEpisodesForDay(day).map((ep): CalendarItem => {
      const hasAired = ep.aired ? new Date(ep.aired) <= getNow() : false;
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

    const movies = filter === "shows" ? [] : getMoviesForDay(day).map((movie): CalendarItem => {
      // Only use scheduled_date (not digital_release_date)
      const displayDate = movie.scheduled_date;
      const hasReleased = displayDate ? new Date(displayDate) <= getNow() : false;
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

<div class="focus-month flex flex-col">
  <!-- Weekday headers -->
  <div class="focus-weekdays grid grid-cols-7 border-b border-border">
    {#each weekdays as day, i}
      <div class="px-3 py-3 text-left text-xs font-semibold text-text-muted">
        <span class="hidden sm:inline">{day}</span>
        <span class="sm:hidden">{weekdaysShort[i]}</span>
      </div>
    {/each}
  </div>

  <!-- Calendar grid -->
  <div class="focus-month-grid grid grid-cols-7">
    {#each calendarDays as day}
      {@const isCurrentMonth = isSameMonth(day, getCurrentDate())}
      {@const today = isToday(day)}
      {@const dayItems = getItemsForDay(day)}
      <div
        role="region"
        aria-label="Calendar day {formatDate(day)}"
        data-date={format(day, "yyyy-MM-dd")}
        class="focus-month-day border-b border-r border-border p-2 min-h-[100px] group hover:bg-surface-hover/30 transition-colors {isCurrentMonth
          ? ''
          : 'bg-background/50'}"
        class:is-today={today}
        class:is-weekend={day.getDay() === 0 || day.getDay() === 6}
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
            aria-label="Schedule for {formatDate(day)}"
          >
            <Plus class="w-4 h-4 text-text-muted" />
          </button>
        </div>

        <!-- Calendar items (episodes + movies) -->
        <div class="space-y-1">
          {#each dayItems.slice(0, 3) as item}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              oncontextmenu={(e) => {
                e.preventDefault();
                if (item.type === "episode") {
                  const ep = item.data as Episode;
                  if (ep.scheduled_date) handleUnschedule(e, ep);
                }
              }}
              class="focus-event p-2 {item.watched ? 'is-watched' : ''}"
              style={item.color ? `border-left: 2px solid ${item.color}` : ''}
            >
              <button
                type="button"
                class="focus-event-open"
                onclick={() => item.type === "movie" ? handleMovieClick(item.data as CalendarMovie) : openShowDetail((item.data as Episode).show_id)}
                title="Open {item.title}{item.subtitle ? ` · ${item.subtitle}` : ''}"
              >
                <span class="focus-event-type">
                  {#if item.type === "movie"}<Film class="w-3 h-3" />{:else}<Tv class="w-3 h-3" />{/if}
                  {item.type === "movie" ? 'Movie' : 'TV episode'}
                  {#if item.watched}<WatchedBadge />{/if}
                  {#if !item.watched}
                    <span class="inline-flex items-center gap-1 {item.hasAired ? 'text-available' : 'text-upcoming'}">
                      {#if item.hasAired}<PlayCircle class="w-3 h-3" aria-hidden="true" />Available{:else}<Clock class="w-3 h-3" aria-hidden="true" />Upcoming{/if}
                    </span>
                  {/if}
                </span>
                <span class="focus-event-title">{item.title}</span>
              </button>
              <div class="focus-event-bottom">
                <span class="truncate">{item.subtitle || (item.watched ? 'Completed watching' : 'Planned watching')}</span>
                <button type="button" class="focus-watch-button" aria-pressed={item.watched} aria-label="{item.watched ? 'Mark unwatched' : 'Mark watched'}: {item.title}" title={item.watched ? 'Mark unwatched' : 'Mark watched'} onclick={() => handleItemClick(item)}>
                  {#if item.watched}<Check class="w-3.5 h-3.5" />{:else}<span class="focus-watch-circle"></span>{/if}
                </button>
              </div>
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
  {@const theme = getThemeSettings()}
  
  <div class="focus-panel focus-dialog fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 bg-surface rounded-xl border border-border shadow-2xl w-full max-w-4xl max-h-[85vh] flex flex-col">
    <div class="p-5 pb-0">
      <h3 class="font-semibold text-lg mb-4">Schedule for {formatDate(showPickerDate)}</h3>

      <!-- Tabs -->
      <div class="focus-tabs flex border-b border-border mb-4">
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

    <div class="flex-1 overflow-auto px-5 pb-5">
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
                  {#if !theme.hidePosters}
                    {#if show.poster_url}
                      <img src={show.poster_url} alt="" class="w-12 h-[72px] rounded object-cover flex-shrink-0" />
                    {:else}
                      <div class="w-12 h-[72px] rounded bg-border flex items-center justify-center flex-shrink-0">
                        <Tv class="w-5 h-5 text-text-muted" />
                      </div>
                    {/if}
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
                  {#if !theme.hidePosters}
                    {#if movie.poster_url}
                      <img src={movie.poster_url} alt="" class="w-12 h-[72px] rounded object-cover flex-shrink-0" />
                    {:else}
                      <div class="w-12 h-[72px] rounded bg-border flex items-center justify-center flex-shrink-0">
                        <Film class="w-5 h-5 text-text-muted" />
                      </div>
                    {/if}
                  {/if}
                  <div class="flex-1">
                    <span class="font-medium">{movie.title}</span>
                    {#if movie.scheduled_date}
                      <p class="text-xs text-text-muted">Currently scheduled: {formatDate(movie.scheduled_date)}</p>
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

<style>
  .focus-month { border: 1px solid var(--color-border); border-radius: 0.875rem; background: var(--color-surface); overflow-x: auto; }
  .focus-weekdays, .focus-month-grid { min-width: 49rem; }
  .focus-month-grid { grid-auto-rows: minmax(6.25rem, auto); }
  .focus-month-day:nth-child(7n) { border-right: 0; }
  .focus-month-day:nth-last-child(-n+7) { border-bottom: 0; }
  .focus-month-day.is-weekend { background: color-mix(in srgb, var(--color-background) 50%, var(--color-surface)); }
  .focus-month-day.is-today { background: color-mix(in srgb, var(--color-accent) 4%, var(--color-surface)); }
  .focus-event { border: 1px solid var(--color-border); border-radius: 0.5rem; background: var(--color-surface-hover); }
  .focus-event.is-watched { background: var(--focus-watched-background); border-color: var(--focus-watched-border); }
  .focus-event-open { display: block; width: 100%; min-width: 0; text-align: left; }
  .focus-event-type { display: flex; align-items: center; flex-wrap: wrap; gap: 0.3rem; color: var(--color-text-muted); font-size: 0.625rem; }
  .focus-event-title { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--color-text); font-size: 0.75rem; font-weight: 600; margin-top: 0.2rem; }
  .focus-event-bottom { display: flex; align-items: center; justify-content: space-between; gap: 0.3rem; color: var(--color-text-muted); font-size: 0.625rem; }
  .focus-watch-button { display: flex; align-items: center; justify-content: center; width: 1.5rem; height: 1.5rem; border: 1px solid var(--color-border); border-radius: 0.3rem; flex: none; color: var(--color-text-muted); }
  .focus-watch-button:hover { background: var(--color-surface); color: var(--color-text); }
  .focus-watch-button[aria-pressed="true"] { background: var(--focus-watched); border-color: var(--focus-watched); color: var(--focus-watched-ink); }
  .focus-watch-button[aria-pressed="true"]:hover { background: color-mix(in srgb, var(--focus-watched) 85%, white); }
  .focus-watch-circle { width: 0.7rem; height: 0.7rem; border: 1px solid var(--color-text-muted); border-radius: 50%; }
  :global(.compact-spacing) .focus-month-grid { grid-auto-rows: minmax(5.75rem, auto); }
  :global(.compact-spacing) .focus-month-day { min-height: 5.75rem; }
</style>
