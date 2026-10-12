<script lang="ts">
  import { format, parseISO, addDays, isPast, isToday } from "date-fns";
  import { Check, Tv, Film } from "lucide-svelte";
  import { getCalendarFilter } from "../../stores/calendar.svelte";
  import { getNow } from "../../stores/now.svelte";
  import { logger } from "../../utils/logger";
  import { formatDateHeader } from "../../utils/dateFormat";
  import { getThemeSettings } from "../../stores/theme.svelte";
  import { openShowDetail } from "../../stores/showDetail.svelte";
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
    const today = getNow();
    const futureDate = addDays(today, 60);
    const startStr = format(today, "yyyy-MM-dd");
    const endStr = format(futureDate, "yyyy-MM-dd");
    // Load episodes and movies in parallel for better performance
    Promise.all([
      loadEpisodesForRange(startStr, endStr),
      loadMoviesForRange(startStr, endStr),
    ]).catch((error) => {
      logger.error("Failed to load calendar data", error);
    });
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
    const filter = getCalendarFilter();

    const episodes = filter === "movies" ? [] : getCalendarEpisodes().map((ep): CalendarItem => {
      const hasAired = ep.aired ? new Date(ep.aired) <= getNow() : false;
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

    const movies = filter === "shows" ? [] : getCalendarMovies()
      .filter((movie) => movie.scheduled_date) // Only include movies with scheduled_date
      .map((movie): CalendarItem => {
        // Only use scheduled_date (not digital_release_date)
        const displayDate = movie.scheduled_date || "";
        const hasReleased = displayDate ? new Date(displayDate) <= getNow() : false;
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

<div class="focus-agenda h-full overflow-auto">
  <div class="focus-agenda-list py-2">
    {#if allItems.length === 0}
      <div class="text-center py-12">
        <Tv class="w-12 h-12 text-text-muted mx-auto mb-4" />
        <p class="text-text-muted">No upcoming episodes or movies</p>
        <p class="text-text-muted text-sm mt-1">Add shows or movies to start tracking!</p>
      </div>
    {:else}
      {#each [...groupByDate(allItems)] as [dateStr, items]}
        {@const isPastDate = isDatePast(dateStr)}
        <section class="focus-agenda-day">
          <!-- Date header -->
          <h2 class="focus-agenda-date text-sm font-semibold text-text-muted {isPastDate ? 'opacity-50' : ''}">
            {formatDateHeader(dateStr)}
          </h2>

          <!-- Items for this date -->
          <div class="space-y-2">
            {#each items as item}
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div
                oncontextmenu={(e) => {
                  e.preventDefault();
                  if (item.type === "episode") {
                    const ep = item.data as Episode;
                    if (ep.scheduled_date) handleUnschedule(e, ep);
                  }
                }}
                class="focus-agenda-item flex items-center gap-4 p-3 text-left {item.watched ? 'is-watched' : ''}"
              >
                <!-- Poster -->
                {#if !getThemeSettings().hidePosters}
                {#if item.posterUrl}
                  <img
                    src={item.posterUrl}
                    alt=""
                    class="w-10 h-15 rounded-md object-cover flex-shrink-0"
                    loading="lazy"
                    decoding="async"
                  />
                {:else}
                  <div class="w-10 h-15 rounded-md bg-surface-hover flex items-center justify-center flex-shrink-0">
                    {#if item.type === "movie"}
                      <Film class="w-5 h-5 text-text-muted" />
                    {:else}
                      <Tv class="w-5 h-5 text-text-muted" />
                    {/if}
                  </div>
                {/if}
                {/if}

                <!-- Item info -->
                <button type="button" class="focus-agenda-open flex-1 min-w-0 text-left" onclick={() => item.type === "movie" ? handleMovieClick(item.data as CalendarMovie) : openShowDetail((item.data as Episode).show_id)}>
                  <div class="flex items-center gap-2">
                    {#if item.type === "movie"}
                      <Film class="w-4 h-4 text-accent flex-shrink-0" />
                    {:else}
                      <Tv class="w-4 h-4 text-accent flex-shrink-0" />
                    {/if}
                    <span class="font-semibold text-text">
                      {item.title}
                    </span>
                  </div>
                  {#if item.subtitle}
                    <div class="text-sm text-text-muted mt-1">
                      {item.subtitle}
                    </div>
                  {/if}
                </button>

                <!-- Status indicator -->
                <span class="focus-agenda-state text-xs text-text-muted">{item.watched ? 'Watched' : item.hasAired ? 'Available' : 'Upcoming'}</span>
                <button type="button" class="focus-watch-button flex-shrink-0" aria-pressed={item.watched} aria-label="{item.watched ? 'Mark unwatched' : 'Mark watched'}: {item.title}" onclick={() => handleItemClick(item)}>
                  {#if item.watched}
                    <Check class="w-4 h-4 text-watched" />
                  {:else}
                    <span class="focus-watch-circle"></span>
                  {/if}
                </button>
              </div>
            {/each}
          </div>
        </section>
      {/each}
    {/if}
  </div>
</div>

<style>
  .focus-agenda-list { max-width: 72rem; }
  .focus-agenda-day { display: grid; grid-template-columns: 9rem minmax(0, 1fr); gap: 1.5rem; padding: 1.4rem 0; border-bottom: 1px solid var(--color-border); }
  .focus-agenda-date { padding-top: 0.75rem; }
  .focus-agenda-item { border: 1px solid var(--color-border); border-radius: 0.75rem; background: var(--color-surface); }
  .focus-agenda-item.is-watched { background: color-mix(in srgb, var(--color-watched) 5%, var(--color-surface)); }
  .focus-agenda-open:hover { color: var(--color-accent); }
  .focus-watch-button { display: flex; align-items: center; justify-content: center; width: 2rem; height: 2rem; border: 1px solid var(--color-border); border-radius: 0.5rem; }
  .focus-watch-button:hover { background: var(--color-surface-hover); }
  .focus-watch-circle { width: 0.9rem; height: 0.9rem; border: 1px solid var(--color-text-muted); border-radius: 50%; }
  :global(.compact-spacing) .focus-agenda-day { padding: 0.75rem 0; }
  @media (max-width: 1050px) { .focus-agenda-day { grid-template-columns: 1fr; gap: 0.75rem; } .focus-agenda-date { padding: 0; } .focus-agenda-state { display: none; } }
</style>
