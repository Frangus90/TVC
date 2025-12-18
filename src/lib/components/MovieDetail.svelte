<script lang="ts">
  import { X, Trash2, RefreshCw, ExternalLink, Star, Eye, EyeOff, Calendar, Archive } from "lucide-svelte";
  import {
    isMovieDetailOpen,
    getCurrentMovie,
    isMovieDetailLoading,
    getMovieDetailError,
    closeMovieDetail,
    markMovieWatched,
    updateMovieRating,
    unscheduleMovie,
    archiveMovie,
    removeMovie,
    syncMovie,
    formatRuntime,
  } from "../stores/movies.svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";

  async function handleRatingChange(newRating: number) {
    const movie = getCurrentMovie();
    if (!movie) return;

    const ratingToSave = movie.rating === newRating ? null : newRating;
    await updateMovieRating(movie.id, ratingToSave);
  }

  async function handleRemove() {
    const movie = getCurrentMovie();
    if (!movie) return;

    if (confirm(`Are you sure you want to remove "${movie.title}"?`)) {
      await removeMovie(movie.id);
      closeMovieDetail();
    }
  }

  async function handleArchive() {
    const movie = getCurrentMovie();
    if (!movie) return;

    await archiveMovie(movie.id);
  }

  async function handleSync() {
    const movie = getCurrentMovie();
    if (!movie) return;

    await syncMovie(movie.id);
  }

  async function handleToggleWatched() {
    const movie = getCurrentMovie();
    if (!movie) return;

    await markMovieWatched(movie.id, !movie.watched);
  }

  async function handleOpenTMDB() {
    const movie = getCurrentMovie();
    if (!movie) return;

    await openUrl(`https://www.themoviedb.org/movie/${movie.id}`);
  }

  async function handleOpenIMDB() {
    const movie = getCurrentMovie();
    if (!movie) return;

    await openUrl(`https://www.imdb.com/find?q=${encodeURIComponent(movie.title)}`);
  }

  function formatDate(dateStr: string | null): string {
    if (!dateStr) return "Unknown";
    const date = new Date(dateStr);
    return date.toLocaleDateString("en-US", {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  }
</script>

{#if isMovieDetailOpen()}
  {@const movie = getCurrentMovie()}

  <!-- Backdrop -->
  <button
    type="button"
    class="fixed inset-0 bg-black/60 z-50"
    onclick={closeMovieDetail}
    aria-label="Close modal"
  ></button>

  <!-- Modal -->
  <div
    class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 bg-surface rounded-xl border border-border shadow-2xl w-[700px] max-w-[95vw] max-h-[90vh] flex flex-col"
  >
    {#if isMovieDetailLoading()}
      <div class="flex items-center justify-center p-12">
        <RefreshCw class="w-8 h-8 text-accent animate-spin" />
      </div>
    {:else if getMovieDetailError()}
      <div class="p-6">
        <p class="text-red-400">{getMovieDetailError()}</p>
        <button
          type="button"
          onclick={closeMovieDetail}
          class="mt-4 px-4 py-2 bg-accent text-white rounded-lg"
        >
          Close
        </button>
      </div>
    {:else if movie}
      <!-- Header -->
      <div class="flex items-start gap-4 p-6 border-b border-border">
        {#if movie.poster_url}
          <img
            src={movie.poster_url}
            alt=""
            class="w-28 h-[168px] rounded object-cover flex-shrink-0"
            loading="lazy"
            decoding="async"
          />
        {:else}
          <div class="w-28 h-[168px] rounded bg-border flex-shrink-0"></div>
        {/if}

        <div class="flex-1 min-w-0">
          <div class="flex items-start justify-between gap-4">
            <div class="flex-1">
              <h2 class="text-2xl font-bold text-text mb-1">{movie.title}</h2>
              {#if movie.tagline}
                <p class="text-sm text-text-muted italic mb-2">"{movie.tagline}"</p>
              {/if}
              <div class="flex items-center gap-3 text-sm text-text-muted">
                {#if movie.runtime}
                  <span>{formatRuntime(movie.runtime)}</span>
                {/if}
                {#if movie.status}
                  <span class="px-2 py-0.5 text-xs rounded bg-surface-hover">
                    {movie.status}
                  </span>
                {/if}
                {#if movie.watched}
                  <span class="flex items-center gap-1 text-watched">
                    <Eye class="w-3.5 h-3.5" />
                    Watched
                  </span>
                {/if}
              </div>
            </div>
            <button
              type="button"
              onclick={closeMovieDetail}
              class="p-2 rounded-lg hover:bg-surface-hover transition-colors flex-shrink-0"
              aria-label="Close"
            >
              <X class="w-5 h-5 text-text-muted" />
            </button>
          </div>

          {#if movie.genres}
            <div class="flex flex-wrap gap-1.5 mt-3">
              {#each movie.genres.split(", ") as genre}
                <span class="px-2 py-0.5 text-xs rounded bg-accent/10 text-accent">
                  {genre}
                </span>
              {/each}
            </div>
          {/if}

          {#if movie.vote_average && movie.vote_average > 0}
            <div class="flex items-center gap-2 mt-3">
              <Star class="w-4 h-4 fill-amber-400 text-amber-400" />
              <span class="text-sm text-text">{movie.vote_average.toFixed(1)}/10</span>
              <span class="text-xs text-text-muted">(TMDB)</span>
            </div>
          {/if}

          <!-- User Rating -->
          <div class="flex items-center gap-2 mt-3">
            <span class="text-sm text-text-muted">Your Rating:</span>
            <div class="flex items-center gap-1">
              {#each [1, 2, 3, 4, 5] as star}
                <button
                  onclick={() => handleRatingChange(star)}
                  class="p-0.5 transition-colors"
                  aria-label={`Rate ${star} stars`}
                  type="button"
                >
                  <Star
                    class="w-5 h-5 {movie.rating !== null && star <= movie.rating
                      ? 'fill-yellow-400 text-yellow-400'
                      : 'text-text-muted hover:text-yellow-400'}"
                  />
                </button>
              {/each}
            </div>
          </div>
        </div>
      </div>

      <!-- Release Dates -->
      <div class="grid grid-cols-3 gap-4 p-4 border-b border-border bg-background/50">
        <div>
          <p class="text-xs text-text-muted mb-1">Theatrical Release</p>
          <p class="text-sm text-text">{formatDate(movie.release_date)}</p>
        </div>
        <div>
          <p class="text-xs text-text-muted mb-1">Digital Release</p>
          <p class="text-sm text-text">{formatDate(movie.digital_release_date)}</p>
        </div>
        <div>
          <p class="text-xs text-text-muted mb-1">Physical Release</p>
          <p class="text-sm text-text">{formatDate(movie.physical_release_date)}</p>
        </div>
      </div>

      <!-- Actions -->
      <div class="flex items-center gap-2 p-4 border-b border-border">
        <button
          type="button"
          onclick={handleToggleWatched}
          class="px-3 py-1.5 text-sm bg-surface-hover hover:bg-surface-hover/80 rounded-lg transition-colors flex items-center gap-2"
        >
          {#if movie.watched}
            <EyeOff class="w-4 h-4" />
            Mark Unwatched
          {:else}
            <Eye class="w-4 h-4" />
            Mark Watched
          {/if}
        </button>
        <button
          type="button"
          onclick={handleSync}
          class="px-3 py-1.5 text-sm bg-surface-hover hover:bg-surface-hover/80 rounded-lg transition-colors flex items-center gap-2"
        >
          <RefreshCw class="w-4 h-4" />
          Refresh
        </button>
        <button
          type="button"
          onclick={handleOpenTMDB}
          class="px-3 py-1.5 text-sm bg-surface-hover hover:bg-surface-hover/80 rounded-lg transition-colors flex items-center gap-2"
        >
          <ExternalLink class="w-4 h-4" />
          TMDB
        </button>
        <button
          type="button"
          onclick={handleOpenIMDB}
          class="px-3 py-1.5 text-sm bg-surface-hover hover:bg-surface-hover/80 rounded-lg transition-colors flex items-center gap-2"
        >
          <ExternalLink class="w-4 h-4" />
          IMDB
        </button>
        <button
          type="button"
          onclick={handleArchive}
          class="px-3 py-1.5 text-sm bg-surface-hover hover:bg-surface-hover/80 rounded-lg transition-colors flex items-center gap-2 ml-auto"
        >
          <Archive class="w-4 h-4" />
          Archive
        </button>
        <button
          type="button"
          onclick={handleRemove}
          class="px-3 py-1.5 text-sm bg-red-500/20 hover:bg-red-500/30 text-red-400 rounded-lg transition-colors flex items-center gap-2"
        >
          <Trash2 class="w-4 h-4" />
          Remove
        </button>
      </div>

      <!-- Overview -->
      {#if movie.overview}
        <div class="flex-1 overflow-auto p-6">
          <h3 class="font-semibold text-text mb-3">Overview</h3>
          <p class="text-text-muted leading-relaxed">{movie.overview}</p>
        </div>
      {/if}

      <!-- Footer with Scheduling -->
      {#if movie.scheduled_date}
        <div class="p-4 border-t border-border bg-accent/5">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <Calendar class="w-4 h-4 text-accent" />
              <span class="text-sm text-text">Scheduled for {formatDate(movie.scheduled_date)}</span>
            </div>
            <button
              type="button"
              onclick={() => unscheduleMovie(movie.id)}
              class="text-xs text-text-muted hover:text-red-400"
            >
              Remove from schedule
            </button>
          </div>
        </div>
      {/if}
    {/if}
  </div>
{/if}
