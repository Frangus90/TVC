<script lang="ts">
  import { format, parseISO, addDays, isToday, isTomorrow, isPast } from "date-fns";
  import { Check, Tv, Film } from "lucide-svelte";
  import {
    getCalendarEpisodes,
    loadEpisodesForRange,
    toggleEpisodeWatched,
    unscheduleEpisode,
    type Episode,
  } from "../../stores/shows.svelte";
  import {
    getCalendarMovies,
    loadMoviesForRange,
    markMovieWatched,
    openMovieDetail,
    type CalendarMovie,
  } from "../../stores/movies.svelte";

  // Load episodes and movies for the next 60 days
  $effect(() => {
    const today = new Date();
    const futureDate = addDays(today, 60);
    const startStr = format(today, "yyyy-MM-dd");
    const endStr = format(futureDate, "yyyy-MM-dd");
    loadEpisodesForRange(startStr, endStr);
    loadMoviesForRange(startStr, endStr);
  });

  // Unified calendar item type
  interface CalendarItem {
    type: "episode" | "movie";
    id: number;
    title: string;
    subtitle: string;
    watched: boolean;
    hasAired: boolean;
    posterUrl: string | null;
    displayDate: string;
    data: Episode | CalendarMovie;
  }

  // Get all calendar items (episodes + movies) - reactive
  let allItems = $derived.by((): CalendarItem[] => {
    const episodes = getCalendarEpisodes().map((ep): CalendarItem => {
      const hasAired = ep.aired ? new Date(ep.aired) <= new Date() : false;
      const title = ep.network ? `${ep.show_name} | ${ep.network}` : ep.show_name;
      return {
        type: "episode",
        id: ep.id,
        title,
        subtitle: `S${String(ep.season_number).padStart(2, "0")}E${String(ep.episode_number).padStart(2, "0")}${ep.name ? ` - ${ep.name}` : ""}`,
        watched: ep.watched,
        hasAired,
        posterUrl: ep.poster_url,
        displayDate: ep.scheduled_date || ep.aired || "",
        data: ep,
      };
    });

    const movies = getCalendarMovies().map((movie): CalendarItem => {
      const displayDate = movie.scheduled_date || movie.digital_release_date || "";
      const hasReleased = displayDate ? new Date(displayDate) <= new Date() : false;
      return {
        type: "movie",
        id: movie.id,
        title: movie.title,
        subtitle: "",
        watched: movie.watched,
        hasAired: hasReleased,
        posterUrl: movie.poster_url,
        displayDate,
        data: movie,
      };
    });

    return [...episodes, ...movies];
  });

  // Group items by date
  function groupByDate(items: CalendarItem[]): Map<string, CalendarItem[]> {
    const grouped = new Map<string, CalendarItem[]>();

    // Sort items by display date
    const sorted = [...items].sort((a, b) => a.displayDate.localeCompare(b.displayDate));

    for (const item of sorted) {
      if (!item.displayDate) continue;

      if (!grouped.has(item.displayDate)) {
        grouped.set(item.displayDate, []);
      }
      grouped.get(item.displayDate)!.push(item);
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

  async function handleUnschedule(event: MouseEvent, episode: Episode) {
    event.stopPropagation();
    if (episode.scheduled_date) {
      await unscheduleEpisode(episode.id);
    }
  }
</script>

<div class="h-full overflow-auto">
  <div class="max-w-3xl mx-auto py-4 space-y-6">
    {#if allItems.length === 0}
      <div class="text-center py-12">
        <Tv class="w-12 h-12 text-text-muted mx-auto mb-4" />
        <p class="text-text-muted">No upcoming episodes or movies</p>
        <p class="text-text-muted text-sm mt-1">Add shows or movies to start tracking!</p>
      </div>
    {:else}
      {#each [...groupByDate(allItems)] as [dateStr, items]}
        {@const isPastDate = isDatePast(dateStr)}
        <div class="space-y-2">
          <!-- Date header -->
          <h2 class="text-sm font-semibold text-text-muted uppercase tracking-wide px-2 {isPastDate ? 'opacity-50' : ''}">
            {formatDateHeader(dateStr)}
          </h2>

          <!-- Items for this date -->
          <div class="space-y-2">
            {#each items as item}
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
                class="w-full flex items-center gap-4 p-4 rounded-xl border transition-colors text-left
                  {item.watched
                    ? 'bg-watched/10 border-watched/30 text-watched'
                    : item.hasAired
                      ? 'bg-premiere/10 border-premiere/30 hover:bg-premiere/20'
                      : 'bg-surface border-border hover:bg-surface-hover'}"
                title={item.watched ? "Watched" : item.hasAired ? "Click to mark watched" : "Upcoming"}
              >
                <!-- Poster -->
                {#if item.posterUrl}
                  <img
                    src={item.posterUrl}
                    alt=""
                    class="w-12 h-18 rounded object-cover flex-shrink-0 {item.watched ? 'opacity-50' : ''}"
                  />
                {:else}
                  <div class="w-12 h-18 rounded bg-border flex items-center justify-center flex-shrink-0">
                    {#if item.type === "movie"}
                      <Film class="w-5 h-5 text-text-muted" />
                    {:else}
                      <Tv class="w-5 h-5 text-text-muted" />
                    {/if}
                  </div>
                {/if}

                <!-- Item info -->
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2">
                    {#if item.type === "movie"}
                      <Film class="w-4 h-4 text-accent flex-shrink-0" />
                    {:else}
                      <Tv class="w-4 h-4 text-accent flex-shrink-0" />
                    {/if}
                    <span class="font-semibold truncate {item.watched ? 'line-through opacity-75' : 'text-text'}">
                      {item.title}
                    </span>
                  </div>
                  {#if item.subtitle}
                    <div class="text-sm {item.watched ? 'opacity-75 line-through' : 'text-text-muted'}">
                      {item.subtitle}
                    </div>
                  {/if}
                </div>

                <!-- Status indicator -->
                <div class="flex-shrink-0">
                  {#if item.watched}
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
