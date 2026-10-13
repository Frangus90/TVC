<script lang="ts">
  import WatchedBadge from "../common/WatchedBadge.svelte";
  import {
    startOfWeek,
    endOfWeek,
    eachDayOfInterval,
    format,
    isToday,
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

  let showPickerOpen = $state(false);
  let showPickerDate = $state<string | null>(null);
  type PickerTab = "shows" | "movies";
  let pickerTab = $state<PickerTab>("shows");

  // Load episodes and movies when week changes (in parallel for better performance)
  $effect(() => {
    const weekStart = startOfWeek(getCurrentDate(), { weekStartsOn: 1 });
    const weekEnd = endOfWeek(getCurrentDate(), { weekStartsOn: 1 });
    const startStr = format(weekStart, "yyyy-MM-dd");
    const endStr = format(weekEnd, "yyyy-MM-dd");
    // Load episodes and movies in parallel
    Promise.all([
      loadEpisodesForRange(startStr, endStr),
      loadMoviesForRange(startStr, endStr),
    ]).catch((error) => {
      logger.error("Failed to load calendar data", error);
    });
  });

  let weekDays = $derived.by(() => {
    const weekStart = startOfWeek(getCurrentDate(), { weekStartsOn: 1 });
    const weekEnd = endOfWeek(getCurrentDate(), { weekStartsOn: 1 });
    return eachDayOfInterval({ start: weekStart, end: weekEnd });
  });

  function getEpisodesForDay(day: Date): Episode[] {
    return getCalendarEpisodes().filter((ep) => {
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
</script>

<div class="focus-week h-full flex flex-col">
  <!-- Week grid - 7 columns -->
  <div class="focus-week-grid flex-1 grid grid-cols-7 gap-3">
    {#each weekDays as day}
      {@const today = isToday(day)}
      {@const dayItems = getItemsForDay(day)}
      <div class="focus-week-column flex flex-col border border-border rounded-xl overflow-hidden bg-surface">
        <!-- Day header -->
        <div class="p-3 border-b border-border flex items-center justify-between {today ? 'bg-accent/10' : ''}">
          <div class="flex items-center gap-2">
            <button
              onclick={() => handleDayClick(day)}
              class="text-center hover:bg-surface-hover rounded-lg px-2 py-1 transition-colors"
            >
              <div class="text-xs text-text-muted uppercase">{format(day, "EEE")}</div>
              <div class="text-xl font-semibold {today ? 'text-accent' : 'text-text'}">
                {format(day, "d")}
              </div>
            </button>
          </div>
          <button
            onclick={() => handleAddClick(day)}
            class="p-1.5 rounded hover:bg-surface-hover transition-colors"
            aria-label="Schedule for {formatDate(day)}"
          >
            <Plus class="w-4 h-4 text-text-muted" />
          </button>
        </div>

        <!-- Items for this day -->
        <div class="flex-1 p-2 space-y-2 overflow-auto">
          {#each dayItems as item}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              oncontextmenu={(e) => {
                e.preventDefault();
                if (item.type === "episode") {
                  const ep = item.data as Episode;
                  if (ep.scheduled_date) handleUnschedule(e, ep);
                }
              }}
              class="focus-event p-3 {item.watched ? 'is-watched' : ''}"
              style={item.color ? `border-left: 3px solid ${item.color}` : ''}
            >
              <button type="button" class="focus-event-open" onclick={() => item.type === "movie" ? handleMovieClick(item.data as CalendarMovie) : openShowDetail((item.data as Episode).show_id)} title="Open {item.title}">
                <span class="focus-event-type">
                  {#if item.type === "movie"}<Film class="w-3.5 h-3.5" />{:else}<Tv class="w-3.5 h-3.5" />{/if}
                  {item.type === "movie" ? 'Movie' : 'TV episode'}
                </span>
                <span class="focus-event-title">{item.title}</span>
                {#if item.subtitle}<span class="focus-event-subtitle">{item.subtitle}</span>{/if}
              </button>
              <div class="focus-event-bottom">
                {#if item.watched}<WatchedBadge />{:else}
                  <span class="inline-flex items-center gap-1 {item.hasAired ? 'text-available' : 'text-upcoming'}">
                    {#if item.hasAired}<PlayCircle class="w-3 h-3" aria-hidden="true" />Available{:else}<Clock class="w-3 h-3" aria-hidden="true" />Upcoming{/if}
                  </span>
                {/if}
                <button type="button" class="focus-watch-button" aria-pressed={item.watched} aria-label="{item.watched ? 'Mark unwatched' : 'Mark watched'}: {item.title}" onclick={() => handleItemClick(item)}>
                  {#if item.watched}<Check class="w-4 h-4" />{:else}<span class="focus-watch-circle"></span>{/if}
                </button>
              </div>
            </div>
          {:else}
            <div class="text-xs text-text-muted text-center py-4">No items</div>
          {/each}
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
  .focus-week-grid { min-height: 0; grid-template-columns: repeat(7, minmax(8rem, 1fr)); overflow-x: auto; }
  .focus-week-column { min-width: 0; }
  .focus-event { border: 1px solid var(--color-border); border-radius: 0.625rem; background: var(--color-surface-hover); }
  .focus-event.is-watched { background: var(--focus-watched-background); border-color: var(--focus-watched-border); }
  .focus-event-open { display: block; width: 100%; text-align: left; }
  .focus-event-type { display: flex; align-items: center; gap: 0.3rem; color: var(--color-text-muted); font-size: 0.6875rem; }
  .focus-event-title { display: block; color: var(--color-text); font-size: 0.875rem; font-weight: 600; overflow-wrap: anywhere; margin: 0.4rem 0; }
  .focus-event-subtitle { display: block; color: var(--color-text-muted); font-size: 0.75rem; line-height: 1.5; }
  .focus-event-bottom { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 0.3rem; color: var(--color-text-muted); font-size: 0.6875rem; margin-top: 0.65rem; }
  .focus-watch-button { display: flex; align-items: center; justify-content: center; width: 1.6rem; height: 1.6rem; margin-left: auto; border: 1px solid var(--color-border); border-radius: 0.4rem; flex: none; color: var(--color-watched); }
  .focus-watch-button:hover { background: var(--color-surface); color: var(--color-text); }
  .focus-watch-button[aria-pressed="true"] { background: var(--focus-watched); border-color: var(--focus-watched); color: var(--focus-watched-ink); }
  .focus-watch-button[aria-pressed="true"]:hover { background: color-mix(in srgb, var(--focus-watched) 85%, white); }
  .focus-watch-circle { width: 0.8rem; height: 0.8rem; border: 1px solid var(--color-text-muted); border-radius: 50%; }
  @media (max-width: 1100px) { .focus-week-grid { gap: 0.375rem; } }
</style>
