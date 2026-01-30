<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import {
    X,
    Clock,
    Tv,
    Film,
    CheckCircle,
    PlayCircle,
    History,
    BarChart3,
    Loader2,
  } from "lucide-svelte";
  import {
    isStatisticsModalOpen,
    closeStatisticsModal,
    getStatistics,
    getCompletionRates,
    getWatchHistory,
    isLoading,
    getActiveTab,
    setActiveTab,
    formatWatchTime,
    formatRelativeDate,
    groupHistoryByDate,
    formatDateHeader,
  } from "../stores/statistics.svelte";

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      closeStatisticsModal();
    }
  }

  function getCompletionColor(percentage: number): string {
    if (percentage === 100) return "bg-green-500";
    if (percentage >= 75) return "bg-blue-500";
    if (percentage >= 50) return "bg-yellow-500";
    if (percentage >= 25) return "bg-orange-500";
    return "bg-red-500";
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

{#if isStatisticsModalOpen()}
  <!-- Backdrop -->
  <button
    transition:fade={{ duration: 150 }}
    class="fixed inset-0 bg-black/60 z-50"
    onclick={closeStatisticsModal}
    aria-label="Close modal"
  ></button>

  <!-- Modal -->
  <div
    transition:scale={{ duration: 200, start: 0.95, opacity: 0 }}
    class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-full max-w-4xl max-h-[85vh] z-50 bg-surface rounded-xl border border-border shadow-2xl flex flex-col"
  >
    <!-- Header -->
    <div class="flex items-center justify-between p-4 border-b border-border">
      <div class="flex items-center gap-3">
        <BarChart3 class="w-6 h-6 text-accent" />
        <h2 class="text-xl font-semibold text-text">Statistics</h2>
      </div>
      <button
        type="button"
        onclick={closeStatisticsModal}
        class="p-1.5 rounded-lg hover:bg-surface-hover transition-colors"
        aria-label="Close"
      >
        <X class="w-5 h-5 text-text-muted" />
      </button>
    </div>

    <!-- Tabs -->
    <div class="flex border-b border-border">
      <button
        class="flex items-center gap-2 px-6 py-3 text-sm font-medium transition-colors {getActiveTab() ===
        'overview'
          ? 'text-accent border-b-2 border-accent'
          : 'text-text-muted hover:text-text'}"
        onclick={() => setActiveTab("overview")}
      >
        <BarChart3 class="w-4 h-4" />
        Overview
      </button>
      <button
        class="flex items-center gap-2 px-6 py-3 text-sm font-medium transition-colors {getActiveTab() ===
        'history'
          ? 'text-accent border-b-2 border-accent'
          : 'text-text-muted hover:text-text'}"
        onclick={() => setActiveTab("history")}
      >
        <History class="w-4 h-4" />
        Watch History
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-auto p-6">
      {#if isLoading()}
        <div class="flex items-center justify-center py-12">
          <Loader2 class="w-8 h-8 text-accent animate-spin" />
        </div>
      {:else if getActiveTab() === "overview"}
        <!-- Overview Tab -->
        {@const stats = getStatistics()}
        {#if stats}
          <!-- Stats Cards -->
          <div class="grid grid-cols-2 md:grid-cols-3 gap-4 mb-8">
            <div class="bg-surface-hover rounded-xl p-4">
              <div class="flex items-center gap-3 mb-2">
                <div class="p-2 bg-accent/20 rounded-lg">
                  <Clock class="w-5 h-5 text-accent" />
                </div>
                <span class="text-sm text-text-muted">Total Watch Time</span>
              </div>
              <p class="text-2xl font-bold text-text">
                {formatWatchTime(stats.total_watch_time_minutes)}
              </p>
            </div>

            <div class="bg-surface-hover rounded-xl p-4">
              <div class="flex items-center gap-3 mb-2">
                <div class="p-2 bg-blue-500/20 rounded-lg">
                  <Tv class="w-5 h-5 text-blue-500" />
                </div>
                <span class="text-sm text-text-muted">Episodes Watched</span>
              </div>
              <p class="text-2xl font-bold text-text">{stats.episodes_watched}</p>
            </div>

            <div class="bg-surface-hover rounded-xl p-4">
              <div class="flex items-center gap-3 mb-2">
                <div class="p-2 bg-purple-500/20 rounded-lg">
                  <Film class="w-5 h-5 text-purple-500" />
                </div>
                <span class="text-sm text-text-muted">Movies Watched</span>
              </div>
              <p class="text-2xl font-bold text-text">{stats.movies_watched}</p>
            </div>

            <div class="bg-surface-hover rounded-xl p-4">
              <div class="flex items-center gap-3 mb-2">
                <div class="p-2 bg-green-500/20 rounded-lg">
                  <CheckCircle class="w-5 h-5 text-green-500" />
                </div>
                <span class="text-sm text-text-muted">Shows Completed</span>
              </div>
              <p class="text-2xl font-bold text-text">{stats.shows_completed}</p>
            </div>

            <div class="bg-surface-hover rounded-xl p-4">
              <div class="flex items-center gap-3 mb-2">
                <div class="p-2 bg-yellow-500/20 rounded-lg">
                  <PlayCircle class="w-5 h-5 text-yellow-500" />
                </div>
                <span class="text-sm text-text-muted">Shows In Progress</span>
              </div>
              <p class="text-2xl font-bold text-text">{stats.shows_in_progress}</p>
            </div>
          </div>

          <!-- Completion Rates -->
          {@const completions = getCompletionRates()}
          {#if completions.length > 0}
            <div class="mb-6">
              <h3 class="text-lg font-semibold text-text mb-4">Show Progress</h3>
              <div class="space-y-3">
                {#each completions as show}
                  <div class="flex items-center gap-4">
                    {#if show.poster_url}
                      <img
                        src={show.poster_url}
                        alt=""
                        class="w-10 h-14 rounded object-cover flex-shrink-0"
                      />
                    {:else}
                      <div
                        class="w-10 h-14 rounded bg-border flex items-center justify-center flex-shrink-0"
                      >
                        <Tv class="w-4 h-4 text-text-muted" />
                      </div>
                    {/if}
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center justify-between mb-1">
                        <span class="text-sm font-medium text-text truncate">{show.show_name}</span>
                        <span class="text-xs text-text-muted">
                          {show.watched_episodes}/{show.total_episodes}
                        </span>
                      </div>
                      <div class="h-2 bg-border rounded-full overflow-hidden">
                        <div
                          class="h-full transition-all duration-300 {getCompletionColor(
                            show.completion_percentage
                          )}"
                          style="width: {show.completion_percentage}%"
                        ></div>
                      </div>
                    </div>
                    <span class="text-sm font-medium text-text-muted w-12 text-right">
                      {Math.round(show.completion_percentage)}%
                    </span>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        {:else}
          <div class="text-center text-text-muted py-12">
            No statistics available yet. Start watching some content!
          </div>
        {/if}
      {:else if getActiveTab() === "history"}
        <!-- History Tab -->
        {@const history = getWatchHistory()}
        {#if history.length > 0}
          {@const groupedHistory = groupHistoryByDate(history)}
          <div class="space-y-6">
            {#each [...groupedHistory] as [dateGroup, items]}
              <div>
                <h3 class="text-sm font-medium text-text-muted mb-3">{formatDateHeader(dateGroup)}</h3>
                <div class="space-y-2">
                  {#each items as item}
                    <div
                      class="flex items-center gap-4 p-3 bg-surface-hover rounded-lg hover:bg-border/50 transition-colors"
                    >
                      {#if item.poster_url}
                        <img
                          src={item.poster_url}
                          alt=""
                          class="w-10 h-14 rounded object-cover flex-shrink-0"
                        />
                      {:else}
                        <div
                          class="w-10 h-14 rounded bg-border flex items-center justify-center flex-shrink-0"
                        >
                          {#if item.item_type === "movie"}
                            <Film class="w-4 h-4 text-text-muted" />
                          {:else}
                            <Tv class="w-4 h-4 text-text-muted" />
                          {/if}
                        </div>
                      {/if}
                      <div class="flex-1 min-w-0">
                        <p class="font-medium text-text truncate">
                          {#if item.item_type === "episode" && item.show_name}
                            {item.show_name}
                          {:else}
                            {item.name}
                          {/if}
                        </p>
                        <p class="text-sm text-text-muted">
                          {#if item.item_type === "episode"}
                            S{String(item.season_number).padStart(2, "0")}E{String(
                              item.episode_number
                            ).padStart(2, "0")}
                            {#if item.name}
                              - {item.name}
                            {/if}
                          {:else}
                            Movie
                          {/if}
                        </p>
                      </div>
                      <div class="text-right flex-shrink-0">
                        <p class="text-sm text-text-muted">
                          {formatRelativeDate(item.watched_at)}
                        </p>
                        {#if item.runtime}
                          <p class="text-xs text-text-muted">{item.runtime}m</p>
                        {/if}
                      </div>
                    </div>
                  {/each}
                </div>
              </div>
            {/each}
          </div>
        {:else}
          <div class="text-center text-text-muted py-12">
            <History class="w-12 h-12 mx-auto mb-4 opacity-50" />
            <p>No watch history yet.</p>
            <p class="text-sm mt-1">Mark episodes and movies as watched to build your history.</p>
          </div>
        {/if}
      {/if}
    </div>

    <!-- Footer -->
    <div class="p-4 border-t border-border">
      <button
        type="button"
        onclick={closeStatisticsModal}
        class="w-full py-2.5 text-text-muted hover:bg-surface-hover rounded-lg transition-colors"
      >
        Close
      </button>
    </div>
  </div>
{/if}
