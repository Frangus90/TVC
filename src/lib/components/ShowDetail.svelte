<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import { X, Trash2, RefreshCw, ExternalLink, Check, CheckCheck, Circle, CheckCircle, Users, List, Play } from "lucide-svelte";
  import { invoke } from "@tauri-apps/api/core";
  import {
    isShowDetailOpen,
    getCurrentShow,
    getShowEpisodes,
    isLoading,
    getError,
    closeShowDetail,
    syncShowEpisodes,
    updateShowRating,
    markSeasonWatched,
    markShowWatched,
    markEpisodeWatched,
    type CastMember,
  } from "../stores/showDetail.svelte";
  import { removeShow } from "../stores/shows.svelte";
  import { type TrailerData } from "../stores/movies.svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import CastCrew from "./CastCrew.svelte";
  import StarRating from "./StarRating.svelte";
  import { openConfirmDialog } from "../stores/confirmDialog.svelte";
  import { logger } from "../utils/logger";

  type Tab = "episodes" | "info";
  let activeTab = $state<Tab>("episodes");

  // Cast state managed locally since we fetch on demand
  let localCast = $state<CastMember[]>([]);
  let localCastLoading = $state(false);

  // Trailer state
  let localTrailer = $state<TrailerData | null>(null);
  let localTrailerLoading = $state(false);
  let localTrailerError = $state<string | null>(null);

  // Reset state when modal opens/closes
  $effect(() => {
    if (isShowDetailOpen()) {
      activeTab = "episodes";
      localCast = [];
      localCastLoading = false;
      localTrailer = null;
      localTrailerLoading = false;
      localTrailerError = null;
    }
  });

  async function handleFetchCast() {
    const show = getCurrentShow();
    if (!show) return;

    localCastLoading = true;
    try {
      // Call fetch command which fetches from API and stores in DB
      const cast = await invoke<CastMember[]>("fetch_show_cast", { showId: show.id });
      localCast = cast;
    } catch (err) {
      logger.error("Failed to fetch cast", err);
    } finally {
      localCastLoading = false;
    }
  }

  async function handleFetchTrailer() {
    const show = getCurrentShow();
    if (!show) return;

    localTrailerLoading = true;
    localTrailerError = null;
    try {
      const trailer = await invoke<TrailerData | null>("get_show_trailer", { showId: show.id });
      localTrailer = trailer;
      if (!trailer) {
        localTrailerError = "No trailer found for this show.";
      }
    } catch (err) {
      logger.error("Failed to fetch trailer", err);
      localTrailerError = err instanceof Error ? err.message : String(err);
    } finally {
      localTrailerLoading = false;
    }
  }

  async function handleRatingChange(newRating: number | null) {
    const show = getCurrentShow();
    if (!show) return;

    await updateShowRating(show.id, newRating);
  }

  async function handleRemove() {
    const show = getCurrentShow();
    if (!show) return;

    const confirmed = await openConfirmDialog({
      title: "Remove Show",
      message: `Are you sure you want to remove "${show.name}"?`,
      type: "danger",
      confirmLabel: "Remove",
      cancelLabel: "Cancel",
    });

    if (confirmed) {
      await removeShow(show.id);
      closeShowDetail();
    }
  }

  async function handleSync() {
    const show = getCurrentShow();
    if (!show) return;

    await syncShowEpisodes(show.id);
  }

  async function handleOpenTVDB() {
    const show = getCurrentShow();
    if (!show) return;

    await openUrl(`https://thetvdb.com/series/${show.slug || show.id}`);
  }

  async function handleOpenWikipedia() {
    const show = getCurrentShow();
    if (!show) return;

    await openUrl(`https://en.wikipedia.org/wiki/${encodeURIComponent(show.name)}`);
  }

  function groupEpisodesBySeason(episodes: ReturnType<typeof getShowEpisodes>) {
    const grouped = new Map<number, typeof episodes>();
    for (const ep of episodes) {
      const season = ep.season_number;
      if (!grouped.has(season)) {
        grouped.set(season, []);
      }
      grouped.get(season)!.push(ep);
    }
    return Array.from(grouped.entries()).sort((a, b) => a[0] - b[0]);
  }

  function getWatchedCount(episodes: ReturnType<typeof getShowEpisodes>) {
    return episodes.filter((ep) => ep.watched).length;
  }

  function isSeasonWatched(seasonEpisodes: ReturnType<typeof getShowEpisodes>) {
    return seasonEpisodes.length > 0 && seasonEpisodes.every((ep) => ep.watched);
  }

  function isAllWatched(episodes: ReturnType<typeof getShowEpisodes>) {
    return episodes.length > 0 && episodes.every((ep) => ep.watched);
  }

  async function handleMarkShowWatched() {
    const show = getCurrentShow();
    const episodes = getShowEpisodes();
    if (!show) return;

    const allWatched = isAllWatched(episodes);
    await markShowWatched(show.id, !allWatched);
  }

  async function handleMarkSeasonWatched(seasonNumber: number, seasonEpisodes: ReturnType<typeof getShowEpisodes>) {
    const show = getCurrentShow();
    if (!show) return;

    const allWatched = isSeasonWatched(seasonEpisodes);
    await markSeasonWatched(show.id, seasonNumber, !allWatched);
  }

  async function handleMarkEpisodeWatched(episodeId: number, currentlyWatched: boolean) {
    await markEpisodeWatched(episodeId, !currentlyWatched);
  }
</script>

{#if isShowDetailOpen()}
  {@const show = getCurrentShow()}
  {@const episodes = getShowEpisodes()}
  {@const grouped = groupEpisodesBySeason(episodes)}
  {@const watchedCount = getWatchedCount(episodes)}
  {@const totalCount = episodes.length}

  <!-- Backdrop -->
  <button
    type="button"
    transition:fade={{ duration: 150 }}
    class="fixed inset-0 bg-black/60 z-50"
    onclick={closeShowDetail}
    aria-label="Close modal"
  ></button>

  <!-- Modal -->
  <div
    transition:scale={{ duration: 200, start: 0.95, opacity: 0 }}
    class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 bg-surface rounded-xl border border-border shadow-2xl w-[900px] max-w-[95vw] max-h-[90vh] flex flex-col"
  >
    {#if isLoading()}
      <div class="flex items-center justify-center p-12">
        <RefreshCw class="w-8 h-8 text-accent animate-spin" />
      </div>
    {:else if getError()}
      <div class="p-6">
        <p class="text-red-400">{getError()}</p>
        <button
          type="button"
          onclick={closeShowDetail}
          class="mt-4 px-4 py-2 bg-accent text-white rounded-lg"
        >
          Close
        </button>
      </div>
    {:else if show}
      <!-- Header -->
      <div class="flex items-start gap-4 p-6 border-b border-border">
        {#if show.poster_url}
          <img
            src={show.poster_url}
            alt=""
            class="w-24 h-[144px] rounded object-cover flex-shrink-0"
            loading="lazy"
            decoding="async"
          />
        {:else}
          <div class="w-24 h-[144px] rounded bg-border flex-shrink-0"></div>
        {/if}

        <div class="flex-1 min-w-0">
          <div class="flex items-start justify-between gap-4">
            <div class="flex-1">
              <h2 class="text-2xl font-bold text-text mb-2">{show.name}</h2>
              {#if show.network || show.first_aired}
                <p class="text-sm text-text-muted mb-1">
                  {[show.network, show.first_aired].filter(Boolean).join(" - ")}
                </p>
              {/if}
              {#if show.status}
                <span class="inline-block px-2 py-1 text-xs rounded bg-surface-hover text-text-muted mb-2">
                  {show.status}
                </span>
              {/if}
            </div>
            <button
              type="button"
              onclick={closeShowDetail}
              class="p-2 rounded-lg hover:bg-surface-hover transition-colors flex-shrink-0"
              aria-label="Close"
            >
              <X class="w-5 h-5 text-text-muted" />
            </button>
          </div>

          {#if show.overview}
            <p class="text-sm text-text-muted mt-3 line-clamp-3">{show.overview}</p>
          {/if}

          <div class="flex items-center gap-4 mt-4">
            <span class="text-sm text-text-muted">
              {watchedCount}/{totalCount} episodes watched
            </span>
            {#if show.runtime}
              <span class="text-sm text-text-muted">{show.runtime} min</span>
            {/if}
          </div>

          <!-- Rating -->
          <div class="flex items-center gap-2 mt-3">
            <span class="text-sm text-text-muted">Rating:</span>
            <StarRating rating={show.rating} onRatingChange={(r) => handleRatingChange(r)} />
          </div>
        </div>
      </div>

      <!-- Actions -->
      <div class="flex items-center gap-2 p-4 border-b border-border">
        <button
          type="button"
          onclick={handleSync}
          class="px-3 py-1.5 text-sm bg-surface-hover hover:bg-surface-hover/80 rounded-lg transition-colors flex items-center gap-2"
        >
          <RefreshCw class="w-4 h-4" />
          Refresh Show
        </button>
        <button
          type="button"
          onclick={handleMarkShowWatched}
          class="px-3 py-1.5 text-sm bg-surface-hover hover:bg-surface-hover/80 rounded-lg transition-colors flex items-center gap-2"
        >
          <CheckCheck class="w-4 h-4" />
          {isAllWatched(episodes) ? "Mark All Unwatched" : "Mark All Watched"}
        </button>
        <button
          type="button"
          onclick={handleOpenTVDB}
          class="px-3 py-1.5 text-sm bg-surface-hover hover:bg-surface-hover/80 rounded-lg transition-colors flex items-center gap-2"
        >
          <ExternalLink class="w-4 h-4" />
          TVDB
        </button>
        <button
          type="button"
          onclick={handleOpenWikipedia}
          class="px-3 py-1.5 text-sm bg-surface-hover hover:bg-surface-hover/80 rounded-lg transition-colors flex items-center gap-2"
        >
          <ExternalLink class="w-4 h-4" />
          Wikipedia
        </button>
        <button
          type="button"
          onclick={handleRemove}
          class="px-3 py-1.5 text-sm bg-red-500/20 hover:bg-red-500/30 text-red-400 rounded-lg transition-colors flex items-center gap-2 ml-auto"
        >
          <Trash2 class="w-4 h-4" />
          Remove
        </button>
      </div>

      <!-- Tabs -->
      <div class="flex border-b border-border">
        <button
          type="button"
          onclick={() => (activeTab = "episodes")}
          class="flex items-center gap-2 px-6 py-3 text-sm font-medium transition-colors border-b-2 {activeTab === 'episodes'
            ? 'text-accent border-accent'
            : 'text-text-muted border-transparent hover:text-text hover:border-border'}"
        >
          <List class="w-4 h-4" />
          Episodes
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
        {#if activeTab === "episodes"}
          <!-- Episodes List -->
          <div class="p-6">
            {#if episodes.length === 0}
              <div class="text-center py-10 text-text-muted">
                <p>No episodes found. Click "Refresh Show" to fetch data from TVDB.</p>
              </div>
            {:else}
              <div class="space-y-6">
                {#each grouped as [season, seasonEpisodes]}
                  <div class="bg-background rounded-lg p-4">
                    <div class="flex items-center justify-between mb-3">
                      <h3 class="font-semibold text-lg">
                        {season === 0 ? "Specials" : `Season ${season}`}
                        <span class="text-sm text-text-muted font-normal ml-2">
                          ({getWatchedCount(seasonEpisodes)}/{seasonEpisodes.length} watched)
                        </span>
                      </h3>
                      <button
                        type="button"
                        onclick={() => handleMarkSeasonWatched(season, seasonEpisodes)}
                        class="px-2 py-1 text-xs bg-surface-hover hover:bg-surface-hover/80 rounded transition-colors flex items-center gap-1"
                      >
                        <Check class="w-3 h-3" />
                        {isSeasonWatched(seasonEpisodes) ? "Mark Unwatched" : "Mark Watched"}
                      </button>
                    </div>
                    <div class="space-y-2">
                      {#each seasonEpisodes as episode}
                        <div
                          class="flex items-center gap-3 p-2 rounded {episode.watched
                            ? 'bg-watched/10'
                            : 'hover:bg-surface-hover'}"
                        >
                          <span class="text-sm font-mono text-text-muted w-16 flex-shrink-0">
                            E{String(episode.episode_number).padStart(2, "0")}
                          </span>
                          <span class="flex-1 text-sm {episode.watched ? 'line-through text-text-muted' : 'text-text'}">
                            {episode.name || "TBA"}
                          </span>
                          {#if episode.aired}
                            <span class="text-xs text-text-muted">{episode.aired}</span>
                          {/if}
                          <button
                            type="button"
                            onclick={() => handleMarkEpisodeWatched(episode.id, episode.watched)}
                            class="p-1.5 rounded-full transition-colors {episode.watched
                              ? 'text-available bg-available/20 hover:bg-available/30'
                              : 'text-text-muted hover:text-available hover:bg-available/10'}"
                            aria-label={episode.watched ? "Mark as unwatched" : "Mark as watched"}
                          >
                            {#if episode.watched}
                              <CheckCircle class="w-4 h-4" />
                            {:else}
                              <Circle class="w-4 h-4" />
                            {/if}
                          </button>
                        </div>
                      {/each}
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {:else if activeTab === "info"}
          <!-- Info Tab with Trailer and Cast -->
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

            <!-- Cast Section -->
            <CastCrew
              cast={localCast}
              loading={localCastLoading}
              onFetch={handleFetchCast}
            />
          </div>
        {/if}
        </div>
        {/key}
      </div>
    {/if}
  </div>
{/if}

<style>
  .line-clamp-3 {
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
