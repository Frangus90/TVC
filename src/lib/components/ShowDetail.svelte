<script lang="ts">
  import { X, Trash2, RefreshCw, ExternalLink, Star } from "lucide-svelte";
  import {
    isShowDetailOpen,
    getCurrentShow,
    getShowEpisodes,
    isLoading,
    getError,
    closeShowDetail,
    syncShowEpisodes,
    updateShowRating,
  } from "../stores/showDetail.svelte";
  import { removeShow } from "../stores/shows.svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";

  async function handleRatingChange(newRating: number) {
    const show = getCurrentShow();
    if (!show) return;

    // If clicking the same rating, clear it (toggle off)
    const ratingToSave = show.rating === newRating ? null : newRating;
    await updateShowRating(show.id, ratingToSave);
  }

  async function handleRemove() {
    const show = getCurrentShow();
    if (!show) return;

    if (confirm(`Are you sure you want to remove "${show.name}"?`)) {
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

  // Group episodes by season
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

  function getTotalCount(episodes: ReturnType<typeof getShowEpisodes>) {
    return episodes.length;
  }
</script>

{#if isShowDetailOpen()}
  {@const show = getCurrentShow()}
  {@const episodes = getShowEpisodes()}
  {@const grouped = groupEpisodesBySeason(episodes)}
  {@const watchedCount = getWatchedCount(episodes)}
  {@const totalCount = getTotalCount(episodes)}

  <!-- Backdrop -->
  <button
    class="fixed inset-0 bg-black/60 z-50"
    onclick={closeShowDetail}
    aria-label="Close modal"
  ></button>

  <!-- Modal -->
  <div
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
            <div class="flex items-center gap-1">
              {#each [1, 2, 3, 4, 5] as star}
                <button
                  onclick={() => handleRatingChange(star)}
                  class="p-1 transition-colors"
                  aria-label={`Rate ${star} stars`}
                  type="button"
                >
                  <Star
                    class="w-5 h-5 {show.rating !== null && star <= show.rating
                      ? 'fill-yellow-400 text-yellow-400'
                      : 'text-text-muted hover:text-yellow-400'}"
                  />
                </button>
              {/each}
            </div>
          </div>
        </div>
      </div>

      <!-- Actions -->
      <div class="flex items-center gap-2 p-4 border-b border-border">
        <button
          onclick={handleSync}
          class="px-3 py-1.5 text-sm bg-surface-hover hover:bg-surface-hover/80 rounded-lg transition-colors flex items-center gap-2"
        >
          <RefreshCw class="w-4 h-4" />
          Refresh Show
        </button>
        <button
          onclick={handleOpenTVDB}
          class="px-3 py-1.5 text-sm bg-surface-hover hover:bg-surface-hover/80 rounded-lg transition-colors flex items-center gap-2"
        >
          <ExternalLink class="w-4 h-4" />
          TVDB
        </button>
        <button
          onclick={handleOpenWikipedia}
          class="px-3 py-1.5 text-sm bg-surface-hover hover:bg-surface-hover/80 rounded-lg transition-colors flex items-center gap-2"
        >
          <ExternalLink class="w-4 h-4" />
          Wikipedia
        </button>
        <button
          onclick={handleRemove}
          class="px-3 py-1.5 text-sm bg-red-500/20 hover:bg-red-500/30 text-red-400 rounded-lg transition-colors flex items-center gap-2 ml-auto"
        >
          <Trash2 class="w-4 h-4" />
          Remove
        </button>
      </div>


      <!-- Episodes List -->
      <div class="flex-1 overflow-auto p-6">
        {#if episodes.length === 0}
          <div class="text-center py-10 text-text-muted">
            <p>No episodes found. Click "Refresh Show" to fetch data from TVDB.</p>
          </div>
        {:else}
          <div class="space-y-6">
            {#each grouped as [season, seasonEpisodes]}
              <div class="bg-background rounded-lg p-4">
                <h3 class="font-semibold text-lg mb-3">
                  {season === 0 ? "Specials" : `Season ${season}`}
                  <span class="text-sm text-text-muted font-normal ml-2">
                    ({seasonEpisodes.length} episodes)
                  </span>
                </h3>
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
                      {#if episode.watched}
                        <span class="text-xs text-watched">Watched</span>
                      {/if}
                    </div>
                  {/each}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .line-clamp-3 {
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>

