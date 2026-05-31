<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import { RefreshCw, Check, AlertTriangle } from "lucide-svelte";
  import {
    isMigrationInProgress,
    getMigrationTotal,
    getMigrationDone,
    getMigrationCurrentName,
    getLastMigrationResult,
    getUnmigratedShows,
    dismissMigrationResult,
  } from "../stores/migration.svelte";
  import UnmigratedShowsResolver from "./UnmigratedShowsResolver.svelte";
</script>

{#if isMigrationInProgress()}
  {@const total = getMigrationTotal()}
  {@const done = getMigrationDone()}
  {@const current = getMigrationCurrentName()}
  {@const percent = total > 0 ? Math.round((done / total) * 100) : 0}

  <!-- Blocking backdrop: the DB is being mutated, don't let the user touch anything. -->
  <div transition:fade={{ duration: 150 }} class="fixed inset-0 bg-black/70 z-[60]"></div>

  <div
    transition:scale={{ duration: 200, start: 0.95, opacity: 0 }}
    class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-[60] bg-surface rounded-xl border border-border shadow-2xl w-full max-w-lg p-6"
  >
    <div class="flex items-center gap-3 mb-4">
      <div class="w-10 h-10 rounded-full bg-accent/20 flex items-center justify-center">
        <RefreshCw class="w-5 h-5 text-accent animate-spin" />
      </div>
      <div>
        <h2 class="text-lg font-semibold text-text">Migrating Library</h2>
        <p class="text-sm text-text-muted">Rewriting shows to TMDB metadata.</p>
      </div>
    </div>

    <div class="space-y-2">
      <div class="flex items-center justify-between text-sm">
        <span class="text-text-muted">
          {done} / {total} shows
        </span>
        <span class="text-text font-medium">{percent}%</span>
      </div>
      <div class="w-full h-2 bg-background rounded-full overflow-hidden">
        <div
          class="h-full bg-accent transition-all duration-300"
          style="width: {percent}%"
        ></div>
      </div>
      {#if current}
        <p class="text-xs text-text-muted truncate">Working on: {current}</p>
      {/if}
    </div>

    <p class="text-xs text-text-muted mt-4 leading-relaxed">
      Please don't close the app. Shows TMDB can't match will be available to
      resolve manually in Data Management when this finishes.
    </p>
  </div>
{:else}
  {@const finished = getLastMigrationResult()}
  {#if finished}
    {@const remaining = getUnmigratedShows().length}
    {@const hasQuarantined = finished.quarantined > 0}
    {@const wide = hasQuarantined}
    <div transition:fade={{ duration: 150 }} class="fixed inset-0 bg-black/60 z-[60]"></div>
    <div
      transition:scale={{ duration: 200, start: 0.95, opacity: 0 }}
      class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-[60] bg-surface rounded-xl border border-border shadow-2xl w-full {wide
        ? 'max-w-3xl max-h-[90vh]'
        : 'max-w-lg'} flex flex-col"
    >
      <div class="p-6 pb-4 flex-shrink-0">
        <div class="flex items-center gap-3 mb-4">
          <div class="w-10 h-10 rounded-full bg-accent/20 flex items-center justify-center">
            {#if finished.errors.length === 0 && remaining === 0}
              <Check class="w-5 h-5 text-accent" />
            {:else}
              <AlertTriangle class="w-5 h-5 text-accent" />
            {/if}
          </div>
          <div>
            <h2 class="text-lg font-semibold text-text">Migration Complete</h2>
            <p class="text-sm text-text-muted">TVDB &rarr; TMDB</p>
          </div>
        </div>

        <ul class="text-sm text-text-muted space-y-1">
          <li>
            <span class="text-text font-medium">{finished.mapped}</span>
            shows mapped successfully
          </li>
          {#if hasQuarantined}
            <li>
              <span class="text-text font-medium">{finished.quarantined}</span>
              shows need manual resolution{remaining < finished.quarantined
                ? ` (${remaining} left)`
                : ""}
            </li>
          {/if}
          {#if finished.errors.length > 0}
            <li>
              <span class="text-text font-medium">{finished.errors.length}</span>
              transient errors — will retry next launch
            </li>
          {/if}
        </ul>

        {#if finished.per_show.some((p) => p.episodes_orphaned > 0)}
          <details class="text-sm text-text-muted mt-3">
            <summary class="cursor-pointer">Shows with lost episode state</summary>
            <ul class="mt-2 space-y-1 max-h-40 overflow-auto">
              {#each finished.per_show.filter((p) => p.episodes_orphaned > 0) as p (p.new_tmdb_id)}
                <li class="text-xs">
                  {p.name}: {p.episodes_orphaned} episode(s) couldn't be matched.
                </li>
              {/each}
            </ul>
          </details>
        {/if}
      </div>

      {#if hasQuarantined && remaining > 0}
        <div class="px-6 pb-2 flex-shrink-0">
          <p class="text-xs text-text-muted leading-relaxed">
            Pick the right TMDB show for each entry below, or delete the ones
            you no longer want. You can also finish later from Data Management.
          </p>
        </div>
        <div class="flex-1 overflow-auto px-6">
          <UnmigratedShowsResolver open={true} variant="inline" />
        </div>
      {/if}

      <div class="flex justify-end p-6 pt-4 flex-shrink-0 border-t border-border">
        <button
          type="button"
          onclick={dismissMigrationResult}
          class="px-4 py-2 text-sm font-medium bg-accent hover:bg-accent/90 text-white rounded-lg transition-colors"
        >
          {#if hasQuarantined && remaining > 0}
            Finish later
          {:else}
            Got it
          {/if}
        </button>
      </div>
    </div>
  {/if}
{/if}
