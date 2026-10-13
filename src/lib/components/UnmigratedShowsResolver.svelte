<script lang="ts">
  import { onDestroy, untrack } from "svelte";
  import { openConfirmDialog } from "../stores/confirmDialog.svelte";
  import { modalFocus } from "../utils/modalFocus";
  import { fade, scale } from "svelte/transition";
  import { X, Search, Trash2, Check, Loader2, AlertTriangle } from "lucide-svelte";
  import {
    getUnmigratedShows,
    getUnmigratedShowsError,
    isUnmigratedShowsLoading,
    isUnmigratedShowBusy,
    loadUnmigratedShows,
    searchTmdbTv,
    resolveUnmigratedShow,
    deleteUnmigratedShow,
    type UnmigratedShow,
    type TmdbTvSearchResult,
  } from "../stores/migration.svelte";
  import { config } from "../config";
  import { logger } from "../utils/logger";
  import { formatDate } from "../utils/dateFormat";

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

  let searchErrors = $state<Record<number, string>>({});
  const requests: Record<number, number> = {};
  let epoch = 0;

  function rowBusy(id: number) { return resolvingIds.has(id) || deletingIds.has(id) || isUnmigratedShowBusy(id); }
  function invalidateRow(id: number) {
    requests[id] = (requests[id] ?? 0) + 1;
    clearTimeout(debounceTimers[id]);
    delete debounceTimers[id];
    searching = new Set([...searching].filter(value => value !== id));
    delete results[id];
    delete searchErrors[id];
  }
  function invalidateAll() {
    epoch++;
    for (const id of Object.keys(requests)) invalidateRow(Number(id));
    queries = {}; results = {}; searchErrors = {}; searching = new Set();
  }
  function closeResolver() { invalidateAll(); onClose?.(); }
  onDestroy(invalidateAll);
  $effect(() => {
    const visible = open;
    untrack(() => { if (visible) { lastError = null; void loadUnmigratedShows(); } });
    return () => untrack(invalidateAll);
  });

  function handleQueryInput(showId: number, value: string) {
    if (rowBusy(showId)) return;
    queries[showId] = value;
    invalidateRow(showId);
    if (!value.trim()) return;
    const request = requests[showId];
    const generation = epoch;
    searching = new Set(searching).add(showId);
    debounceTimers[showId] = setTimeout(async () => {
      delete debounceTimers[showId];
      if (!open || generation !== epoch || request !== requests[showId] || rowBusy(showId)) return;
      try {
        const rows = await searchTmdbTv(value.trim());
        if (open && generation === epoch && request === requests[showId] && !rowBusy(showId)) results[showId] = rows;
      } catch (cause) {
        if (open && generation === epoch && request === requests[showId]) searchErrors[showId] = `Search failed: ${cause instanceof Error ? cause.message : String(cause)}`;
      } finally {
        if (generation === epoch && request === requests[showId]) searching = new Set([...searching].filter(value => value !== showId));
      }
    }, config.ui.searchDebounceMs);
  }

  async function handleResolve(show: UnmigratedShow, tmdbId: number) {
    if (rowBusy(show.id)) return;
    resolvingIds = new Set(resolvingIds).add(show.id);
    invalidateRow(show.id);
    lastError = null;
    const generation = epoch;
    try {
      await resolveUnmigratedShow(show.id, tmdbId);
      delete queries[show.id];
    } catch (cause) {
      if (open && generation === epoch) lastError = `Failed to resolve “${show.name}”: ${cause instanceof Error ? cause.message : String(cause)}`;
      logger.error("[Resolver] resolve failed", cause);
    } finally {
      resolvingIds = new Set([...resolvingIds].filter(id => id !== show.id));
    }
  }

  async function handleDelete(show: UnmigratedShow) {
    if (rowBusy(show.id)) return;
    deletingIds = new Set(deletingIds).add(show.id);
    invalidateRow(show.id);
    lastError = null;
    const generation = epoch;
    try {
      const confirmed = await openConfirmDialog({
        title: "Delete unmapped show?",
        message: `Delete “${show.name}” and all its episodes, including saved watched history and schedules? This cannot be undone.`,
        type: "danger", confirmLabel: "Delete show", cancelLabel: "Keep show",
      });
      if (!confirmed || !open || generation !== epoch) return;
      await deleteUnmigratedShow(show.id);
      delete queries[show.id];
    } catch (cause) {
      if (open && generation === epoch) lastError = `Failed to delete “${show.name}”: ${cause instanceof Error ? cause.message : String(cause)}`;
      logger.error("[Resolver] delete failed", cause);
    } finally {
      deletingIds = new Set([...deletingIds].filter(id => id !== show.id));
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

  {#if getUnmigratedShowsError()}
    <div role="alert" class="p-3 rounded bg-red-500/10 border border-red-500/30 text-sm text-red-400">
      Could not load unmapped shows: {getUnmigratedShowsError()}
      <button type="button" class="underline ml-2" onclick={() => loadUnmigratedShows()}>Retry</button>
    </div>
  {/if}
  {#if isUnmigratedShowsLoading() && !getUnmigratedShows().length}<p role="status" class="text-sm text-text-muted">Loading unmapped shows…</p>{/if}
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
                &middot; First aired: {formatDate(show.first_aired)}
              {/if}
            </p>

            <div class="mt-3 relative">
              <Search class="absolute left-2 top-2.5 w-4 h-4 text-text-muted pointer-events-none" />
              <input
                type="search"
                disabled={rowBusy(show.id)}
                aria-label={`Find a TMDB match for ${show.name}`}
                placeholder="Search TMDB for the right show…"
                value={queries[show.id] ?? show.name}
                oninput={(e) => handleQueryInput(show.id, (e.target as HTMLInputElement).value)}
                onkeydown={(e) => { if (e.key === 'Enter') handleQueryInput(show.id, e.currentTarget.value); }}
                class="w-full pl-8 pr-20 py-2 bg-surface border border-border rounded-md text-sm text-text placeholder-text-muted focus:outline-none focus:border-accent"
              />
              <button type="button" disabled={rowBusy(show.id) || searching.has(show.id)} onclick={() => handleQueryInput(show.id, queries[show.id] ?? show.name)} class="absolute right-1 top-1 bottom-1 px-2 text-xs text-accent hover:bg-accent/10 rounded disabled:opacity-50">Search</button>
            </div>

            {#if searching.has(show.id)}
              <div class="mt-2 flex items-center gap-2 text-xs text-text-muted">
                <Loader2 class="w-3 h-3 animate-spin" /> Searching…
              </div>
            {/if}

            {#if searchErrors[show.id]}
              <p role="alert" class="text-xs text-red-400 mt-2">{searchErrors[show.id]} <button type="button" class="underline" onclick={() => handleQueryInput(show.id, queries[show.id] ?? show.name)}>Retry search</button></p>
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
                        {r.first_air_date ? formatDate(r.first_air_date) : "Unknown date"}
                        &middot; TMDB id {r.id}
                      </p>
                    </div>
                    <button
                      type="button"
                      disabled={rowBusy(show.id)}
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
            {:else if queries[show.id]?.trim() && results[show.id]?.length === 0 && !searching.has(show.id) && !searchErrors[show.id]}
              <p class="mt-2 text-xs text-text-muted">No TMDB matches.</p>
            {/if}
          </div>

          <button
            type="button"
            disabled={rowBusy(show.id)}
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
      {#if !isUnmigratedShowsLoading() && !getUnmigratedShowsError()}
      <p class="text-sm text-text-muted text-center py-8">
        No shows need resolving. You're all set.
      </p>
      {/if}
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
      onclick={closeResolver}
      aria-label="Close resolver"
    ></button>

    <div
      transition:scale={{ duration: 200, start: 0.95, opacity: 0 }}
      use:modalFocus={closeResolver}
      role="dialog" aria-modal="true" aria-labelledby="unmapped-resolver-title"
      class="focus-dialog fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 bg-surface rounded-xl border border-border shadow-2xl w-full max-w-3xl max-h-[90vh] flex flex-col"
    >
      <div class="flex items-center justify-between p-5 border-b border-border">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-full bg-accent/20 flex items-center justify-center">
            <AlertTriangle class="w-5 h-5 text-accent" />
          </div>
          <div>
            <h2 id="unmapped-resolver-title" class="text-lg font-semibold text-text">Resolve Unmapped Shows</h2>
            <p class="text-sm text-text-muted">
              Pick the right TMDB show for each entry, or delete it.
            </p>
          </div>
        </div>
        <button
          type="button"
          onclick={closeResolver}
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
