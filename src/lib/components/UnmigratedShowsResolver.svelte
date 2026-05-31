<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import { X, Search, Trash2, Check, Loader2, AlertTriangle } from "lucide-svelte";
  import {
    getUnmigratedShows,
    loadUnmigratedShows,
    searchTmdbTv,
    resolveUnmigratedShow,
    deleteUnmigratedShow,
    type UnmigratedShow,
    type TmdbTvSearchResult,
  } from "../stores/migration.svelte";
  import { config } from "../config";
  import { logger } from "../utils/logger";

  interface Props {
    open: boolean;
    onClose?: () => void;
    /**
     * "modal" (default) renders a standalone dialog with backdrop + chrome.
     * "inline" renders only the row list, so the parent can embed it inside
     * another container (e.g. the migration completion modal).
     */
    variant?: "modal" | "inline";
  }

  let { open, onClose, variant = "modal" }: Props = $props();

  let queries = $state<Record<number, string>>({});
  let results = $state<Record<number, TmdbTvSearchResult[]>>({});
  let searching = $state<Set<number>>(new Set());
  let resolvingIds = $state<Set<number>>(new Set());
  let deletingIds = $state<Set<number>>(new Set());
  let debounceTimers: Record<number, ReturnType<typeof setTimeout>> = {};
  let lastError = $state<string | null>(null);

  $effect(() => {
    if (open) {
      loadUnmigratedShows();
    }
  });

  function handleQueryInput(showId: number, value: string) {
    queries[showId] = value;

    if (debounceTimers[showId]) clearTimeout(debounceTimers[showId]);

    if (!value.trim()) {
      results[showId] = [];
      return;
    }

    debounceTimers[showId] = setTimeout(async () => {
      searching = new Set(searching).add(showId);
      try {
        results[showId] = await searchTmdbTv(value);
      } finally {
        const next = new Set(searching);
        next.delete(showId);
        searching = next;
      }
    }, config.ui.searchDebounceMs);
  }

  async function handleResolve(show: UnmigratedShow, tmdbId: number) {
    if (resolvingIds.has(show.id)) return;
    resolvingIds = new Set(resolvingIds).add(show.id);
    lastError = null;
    try {
      await resolveUnmigratedShow(show.id, tmdbId);
      // Cleanup local UI state for this row.
      delete queries[show.id];
      delete results[show.id];
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      lastError = `Failed to resolve "${show.name}": ${msg}`;
      logger.error("[Resolver] resolve failed", e);
    } finally {
      const next = new Set(resolvingIds);
      next.delete(show.id);
      resolvingIds = next;
    }
  }

  async function handleDelete(show: UnmigratedShow) {
    if (deletingIds.has(show.id)) return;
    if (!confirm(`Delete "${show.name}" and all its episodes? This can't be undone.`)) return;
    deletingIds = new Set(deletingIds).add(show.id);
    lastError = null;
    try {
      await deleteUnmigratedShow(show.id);
      delete queries[show.id];
      delete results[show.id];
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      lastError = `Failed to delete "${show.name}": ${msg}`;
      logger.error("[Resolver] delete failed", e);
    } finally {
      const next = new Set(deletingIds);
      next.delete(show.id);
      deletingIds = next;
    }
  }

  function posterUrl(path: string | null): string | null {
    if (!path) return null;
    return `https://image.tmdb.org/t/p/w500${path}`;
  }
</script>

{#snippet rowList()}
  {#if lastError}
    <div class="p-3 rounded bg-red-500/10 border border-red-500/30 text-sm text-red-400">
      {lastError}
    </div>
  {/if}

  <div class="space-y-4">
    {#each getUnmigratedShows() as show (show.id)}
      <div class="border border-border rounded-lg p-4 bg-background">
        <div class="flex gap-4">
          {#if show.poster_url}
            <img
              src={show.poster_url}
              alt=""
              class="w-16 h-24 object-cover rounded flex-shrink-0"
              loading="lazy"
            />
          {:else}
            <div class="w-16 h-24 bg-surface-hover rounded flex-shrink-0"></div>
          {/if}

          <div class="flex-1 min-w-0">
            <p class="font-medium text-text truncate">{show.name}</p>
            <p class="text-xs text-text-muted">
              Legacy TVDB id: {show.legacy_tvdb_id ?? "unknown"}
              {#if show.first_aired}
                &middot; First aired: {show.first_aired}
              {/if}
            </p>

            <div class="mt-3 relative">
              <Search class="absolute left-2 top-2.5 w-4 h-4 text-text-muted pointer-events-none" />
              <input
                type="text"
                placeholder="Search TMDB for the right show…"
                value={queries[show.id] ?? show.name}
                oninput={(e) => handleQueryInput(show.id, (e.target as HTMLInputElement).value)}
                class="w-full pl-8 pr-3 py-2 bg-surface border border-border rounded-md text-sm text-text placeholder-text-muted focus:outline-none focus:border-accent"
              />
            </div>

            {#if searching.has(show.id)}
              <div class="mt-2 flex items-center gap-2 text-xs text-text-muted">
                <Loader2 class="w-3 h-3 animate-spin" /> Searching…
              </div>
            {/if}

            {#if results[show.id] && results[show.id].length > 0}
              <ul class="mt-3 max-h-60 overflow-auto border border-border rounded divide-y divide-border">
                {#each results[show.id] as r (r.id)}
                  <li class="p-2 flex gap-3 items-center">
                    {#if r.poster_path}
                      <img
                        src={posterUrl(r.poster_path)}
                        alt=""
                        class="w-10 h-14 object-cover rounded flex-shrink-0"
                        loading="lazy"
                      />
                    {:else}
                      <div class="w-10 h-14 bg-surface-hover rounded flex-shrink-0"></div>
                    {/if}
                    <div class="flex-1 min-w-0">
                      <p class="text-sm text-text truncate">{r.name}</p>
                      <p class="text-xs text-text-muted truncate">
                        {r.first_air_date ?? "Unknown date"}
                        &middot; TMDB id {r.id}
                      </p>
                    </div>
                    <button
                      type="button"
                      disabled={resolvingIds.has(show.id)}
                      onclick={() => handleResolve(show, r.id)}
                      class="px-3 py-1.5 text-xs font-medium bg-accent hover:bg-accent/90 text-white rounded transition-colors flex items-center gap-1 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      {#if resolvingIds.has(show.id)}
                        <Loader2 class="w-3 h-3 animate-spin" />
                      {:else}
                        <Check class="w-3 h-3" />
                      {/if}
                      Resolve
                    </button>
                  </li>
                {/each}
              </ul>
            {:else if queries[show.id] && !searching.has(show.id)}
              <p class="mt-2 text-xs text-text-muted">No TMDB matches.</p>
            {/if}
          </div>

          <button
            type="button"
            disabled={deletingIds.has(show.id)}
            onclick={() => handleDelete(show)}
            class="p-2 rounded-lg hover:bg-red-500/10 text-text-muted hover:text-red-400 transition-colors self-start disabled:opacity-50"
            aria-label="Delete show"
            title="Delete this show"
          >
            {#if deletingIds.has(show.id)}
              <Loader2 class="w-4 h-4 animate-spin" />
            {:else}
              <Trash2 class="w-4 h-4" />
            {/if}
          </button>
        </div>
      </div>
    {:else}
      <p class="text-sm text-text-muted text-center py-8">
        No shows need resolving. You're all set.
      </p>
    {/each}
  </div>
{/snippet}

{#if open}
  {#if variant === "inline"}
    {@render rowList()}
  {:else}
    <button
      type="button"
      transition:fade={{ duration: 150 }}
      class="fixed inset-0 bg-black/60 z-50"
      onclick={onClose}
      aria-label="Close resolver"
    ></button>

    <div
      transition:scale={{ duration: 200, start: 0.95, opacity: 0 }}
      class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 bg-surface rounded-xl border border-border shadow-2xl w-full max-w-3xl max-h-[90vh] flex flex-col"
    >
      <div class="flex items-center justify-between p-5 border-b border-border">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-full bg-accent/20 flex items-center justify-center">
            <AlertTriangle class="w-5 h-5 text-accent" />
          </div>
          <div>
            <h2 class="text-lg font-semibold text-text">Resolve Unmapped Shows</h2>
            <p class="text-sm text-text-muted">
              Pick the right TMDB show for each entry, or delete it.
            </p>
          </div>
        </div>
        <button
          type="button"
          onclick={onClose}
          class="p-2 rounded-lg hover:bg-surface-hover transition-colors"
          aria-label="Close"
        >
          <X class="w-5 h-5 text-text-muted" />
        </button>
      </div>

      <div class="flex-1 overflow-auto p-5">
        {@render rowList()}
      </div>
    </div>
  {/if}
{/if}
