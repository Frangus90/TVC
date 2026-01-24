<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import { X, Trash2, RefreshCw, ExternalLink, Star, Eye, EyeOff, Calendar, Archive, Play, Users, FileText } from "lucide-svelte";
  import { invoke } from "@tauri-apps/api/core";
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
    type MovieCastMember,
    type MovieCrewMember,
    type TrailerData,
  } from "../stores/movies.svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import CastCrew from "./CastCrew.svelte";
  import StarRating from "./StarRating.svelte";
  import { openConfirmDialog } from "../stores/confirmDialog.svelte";
  import { logger } from "../utils/logger";

  type Tab = "overview" | "info";
  let activeTab = $state<Tab>("overview");

  // Cast/crew state managed locally
  let localCast = $state<MovieCastMember[]>([]);
  let localCrew = $state<MovieCrewMember[]>([]);
  let localCastLoading = $state(false);

  // Trailer state
  let localTrailer = $state<TrailerData | null>(null);
  let localTrailerLoading = $state(false);
  let localTrailerError = $state<string | null>(null);

  // Reset state when modal opens/closes
  $effect(() => {
    if (isMovieDetailOpen()) {
      activeTab = "overview";
      localCast = [];
      localCrew = [];
      localCastLoading = false;
      localTrailer = null;
      localTrailerLoading = false;
      localTrailerError = null;
    }
  });

  async function handleFetchCastCrew() {
    const movie = getCurrentMovie();
    if (!movie) return;

    localCastLoading = true;
    try {
      const result = await invoke<{ cast: MovieCastMember[]; crew: MovieCrewMember[] }>(
        "fetch_movie_cast_crew",
        { movieId: movie.id }
      );
      localCast = result.cast;
      localCrew = result.crew;
    } catch (err) {
      logger.error("Failed to fetch cast/crew", err);
    } finally {
      localCastLoading = false;
    }
  }

  async function handleFetchTrailer() {
    const movie = getCurrentMovie();
    if (!movie) return;

    localTrailerLoading = true;
    localTrailerError = null;
    try {
      const trailer = await invoke<TrailerData | null>("get_movie_trailer", { movieId: movie.id });
      localTrailer = trailer;
      if (!trailer) {
        localTrailerError = "No trailer found for this movie.";
      }
    } catch (err) {
      logger.error("Failed to fetch trailer", err);
      localTrailerError = err instanceof Error ? err.message : String(err);
    } finally {
      localTrailerLoading = false;
    }
  }

  async function handleRatingChange(newRating: number | null) {
    const movie = getCurrentMovie();
    if (!movie) return;

    await updateMovieRating(movie.id, newRating);
  }

  async function handleRemove() {
    const movie = getCurrentMovie();
    if (!movie) return;

    const confirmed = await openConfirmDialog({
      title: "Remove Movie",
      message: `Are you sure you want to remove "${movie.title}"?`,
      type: "danger",
      confirmLabel: "Remove",
      cancelLabel: "Cancel",
    });

    if (confirmed) {
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
    transition:fade={{ duration: 150 }}
    class="fixed inset-0 bg-black/60 z-50"
    onclick={closeMovieDetail}
    aria-label="Close modal"
  ></button>

  <!-- Modal -->
  <div
    transition:scale={{ duration: 200, start: 0.95, opacity: 0 }}
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
            <StarRating rating={movie.rating} onRatingChange={(r) => handleRatingChange(r)} />
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

      <!-- Tabs -->
      <div class="flex border-b border-border">
        <button
          type="button"
          onclick={() => (activeTab = "overview")}
          class="flex items-center gap-2 px-6 py-3 text-sm font-medium transition-colors border-b-2 {activeTab === 'overview'
            ? 'text-accent border-accent'
            : 'text-text-muted border-transparent hover:text-text hover:border-border'}"
        >
          <FileText class="w-4 h-4" />
          Overview
        </button>
        <button
          type="button"
          onclick={() => (activeTab = "info")}
          class="flex items-center gap-2 px-6 py-3 text-sm font-medium transition-colors border-b-2 {activeTab === 'info'
            ? 'text-accent border-accent'
            : 'text-text-muted border-transparent hover:text-text hover:border-border'}"
        >
          <Users class="w-4 h-4" />
          Extra Info
        </button>
      </div>

      <!-- Tab Content -->
      <div class="flex-1 overflow-auto">
        {#key activeTab}
        <div in:fade={{ duration: 150 }}>
        {#if activeTab === "overview"}
          <!-- Overview Tab -->
          <div class="p-6">
            {#if movie.overview}
              <h3 class="font-semibold text-text mb-3">Synopsis</h3>
              <p class="text-text-muted leading-relaxed">{movie.overview}</p>
            {:else}
              <p class="text-text-muted text-center py-8">No synopsis available.</p>
            {/if}
          </div>
        {:else if activeTab === "info"}
          <!-- Extra Info Tab with Trailer and Cast & Crew -->
          <div class="p-6 space-y-6">
            <!-- Trailer Section -->
            <div>
              <div class="flex items-center justify-between mb-3">
                <h3 class="font-semibold text-text">Trailer</h3>
                <button
                  type="button"
                  onclick={handleFetchTrailer}
                  disabled={localTrailerLoading}
                  class="px-2 py-1 text-xs bg-surface-hover hover:bg-surface-hover/80 rounded transition-colors flex items-center gap-1.5 disabled:opacity-50"
                >
                  <RefreshCw class="w-3 h-3 {localTrailerLoading ? 'animate-spin' : ''}" />
                  {localTrailer ? "Refresh" : "Load Trailer"}
                </button>
              </div>
              {#if localTrailerLoading}
                <div class="flex items-center justify-center py-6">
                  <RefreshCw class="w-5 h-5 text-accent animate-spin" />
                </div>
              {:else if localTrailer}
                <button
                  type="button"
                  onclick={() => localTrailer && openUrl(localTrailer.url)}
                  class="flex items-center gap-3 w-full p-3 bg-background rounded-lg hover:bg-surface-hover transition-colors"
                >
                  <div class="w-10 h-10 rounded-full bg-red-500 flex items-center justify-center flex-shrink-0">
                    <Play class="w-5 h-5 text-white fill-white" />
                  </div>
                  <div class="text-left">
                    <p class="text-sm font-medium text-text">{localTrailer.name}</p>
                    <p class="text-xs text-text-muted">Watch on {localTrailer.site}</p>
                  </div>
                </button>
              {:else if localTrailerError}
                <p class="text-sm text-red-400 text-center py-4 bg-red-500/10 rounded-lg">
                  {localTrailerError}
                </p>
              {:else}
                <p class="text-sm text-text-muted text-center py-4 bg-background rounded-lg">
                  Click "Load Trailer" to fetch trailer from TMDB.
                </p>
              {/if}
            </div>

            <!-- Cast & Crew Section -->
            <CastCrew
              cast={localCast}
              crew={localCrew}
              loading={localCastLoading}
              onFetch={handleFetchCastCrew}
            />
          </div>
        {/if}
        </div>
        {/key}
      </div>

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
