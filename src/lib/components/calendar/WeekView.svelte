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

  // Load episodes and movies when week changes
  $effect(() => {
    const weekStart = startOfWeek(getCurrentDate(), { weekStartsOn: 1 });
    const weekEnd = endOfWeek(getCurrentDate(), { weekStartsOn: 1 });
    const startStr = format(weekStart, "yyyy-MM-dd");
    const endStr = format(weekEnd, "yyyy-MM-dd");
    loadEpisodesForRange(startStr, endStr);
    loadMoviesForRange(startStr, endStr);
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
      const displayDate = movie.scheduled_date || movie.digital_release_date;
      if (!displayDate) return false;
      return isSameDay(parseISO(displayDate), day);
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
      return {
        type: "episode",
        id: ep.id,
        title: ep.show_name,
        subtitle: `S${String(ep.season_number).padStart(2, "0")}E${String(ep.episode_number).padStart(2, "0")}${ep.name ? ` - ${ep.name}` : ""}`,
        watched: ep.watched,
        hasAired,
        color: getShowColor(ep.show_id),
        data: ep,
      };
    });

    const movies = getMoviesForDay(day).map((movie): CalendarItem => {
      const displayDate = movie.scheduled_date || movie.digital_release_date;
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
</script>

<div class="h-full flex flex-col">
  <!-- Week grid - 7 columns -->
  <div class="flex-1 grid grid-cols-7 gap-2">
    {#each weekDays as day}
      {@const today = isToday(day)}
      {@const dayItems = getItemsForDay(day)}
      <div class="flex flex-col border border-border rounded-lg overflow-hidden bg-surface">
        <!-- Day header -->
        <div class="p-3 border-b border-border flex items-center justify-between {today ? 'bg-accent/10 ring-2 ring-accent/50' : ''}">
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
            aria-label="Schedule episode"
          >
            <Plus class="w-4 h-4 text-text-muted" />
          </button>
        </div>

        <!-- Items for this day -->
        <div class="flex-1 p-2 space-y-2 overflow-auto">
          {#each dayItems as item}
            <button
              onclick={(e) => { e.stopPropagation(); handleItemClick(item); }}
              ondblclick={() => item.type === "movie" && handleMovieClick(item.data as CalendarMovie)}
              oncontextmenu={(e) => {
                e.preventDefault();
                if (item.type === "episode") {
                  const ep = item.data as Episode;
                  if (ep.scheduled_date) handleUnschedule(e, ep);
                }
              }}
              class="w-full text-left p-2 rounded-lg text-sm transition-colors {item.watched
                ? 'bg-watched/20 text-watched'
                : item.hasAired
                  ? 'bg-premiere/20 text-premiere hover:bg-premiere/30'
                  : 'bg-upcoming/20 text-upcoming hover:bg-upcoming/30'}"
              style={item.color ? `border-left: 3px solid ${item.color}` : ''}
              title={item.watched ? "Watched" : item.hasAired ? "Click to mark watched" : "Upcoming"}
            >
              <div class="flex items-center gap-1.5 mb-1">
                {#if item.watched}
                  <Check class="w-3.5 h-3.5 flex-shrink-0" />
                {:else if item.type === "movie"}
                  <Film class="w-3.5 h-3.5 flex-shrink-0" />
                {:else}
                  <Tv class="w-3.5 h-3.5 flex-shrink-0" />
                {/if}
                <span class="font-medium truncate">{item.title}</span>
              </div>
              {#if item.subtitle}
                <div class="text-xs opacity-75 {item.watched ? 'line-through' : ''}">
                  {item.subtitle}
                </div>
              {/if}
            </button>
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
