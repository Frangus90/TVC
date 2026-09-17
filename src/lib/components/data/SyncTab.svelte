<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { RefreshCw, Tv, Film, AlertTriangle, Check } from "lucide-svelte";
  import type { LibraryKind, LibrarySyncStatus, SyncFrequency } from "../../stores/librarySync";
  import { formatDateTime } from "../../utils/dateFormat";

  let libraries = $state<LibrarySyncStatus[]>([]);
  let loadError = $state<string | null>(null);
  let actionError = $state<string | null>(null);
  let pending = $state(false);
  let saving = $state<LibraryKind | null>(null);
  let now = $state(Date.now());
  let disposed = false;
  let refreshPromise: Promise<void> | null = null;
  const busy = $derived(pending || libraries.some((library) => library.running));

  function refresh(): Promise<void> {
    if (refreshPromise) return refreshPromise;
    refreshPromise = invoke<LibrarySyncStatus[]>("get_library_sync_status")
      .then((result) => {
        if (!disposed) { libraries = result; loadError = null; }
      })
      .catch((error) => { if (!disposed) loadError = String(error); })
      .finally(() => { refreshPromise = null; });
    return refreshPromise;
  }

  async function sync(kind: LibraryKind | null = null, retryFailed = false) {
    pending = true;
    actionError = null;
    try {
      await invoke("run_library_sync", { kind, retryFailed });
    } catch (error) {
      actionError = String(error);
    } finally {
      await refreshPromise;
      await refresh();
      pending = false;
    }
  }

  async function saveSchedule(library: LibrarySyncStatus, select: HTMLSelectElement) {
    const frequency = select.value as SyncFrequency;
    // Keep showing the persisted value until saving succeeds.
    select.value = library.frequency;
    saving = library.kind;
    actionError = null;
    try {
      await invoke("set_library_sync_schedule", { kind: library.kind, frequency });
      await refreshPromise;
      await refresh();
    } catch (error) {
      actionError = `Schedule was not saved: ${error}`;
    } finally { saving = null; }
  }

  onMount(() => {
    void refresh();
    const timer = setInterval(() => { now = Date.now(); void refresh(); }, 2000);
    return () => { disposed = true; clearInterval(timer); };
  });
</script>

<div class="space-y-4">
  <div class="flex items-start justify-between gap-4">
    <div>
      <h3 class="font-semibold text-text">TMDB sync</h3>
      <p class="text-sm text-text-muted mt-1">Refresh titles, artwork, release dates, and TV episodes.</p>
    </div>
    <button type="button" onclick={() => sync()} disabled={busy || saving !== null || !libraries.length || !!loadError}
      class="shrink-0 flex items-center gap-2 px-3 py-2 text-sm bg-accent/20 hover:bg-accent/30 text-accent rounded-lg disabled:opacity-50">
      <RefreshCw class="w-4 h-4 {busy ? 'animate-spin' : ''}" />
      Sync All
    </button>
  </div>

  <p class="text-sm text-text-muted rounded-lg border border-border p-3">
    Automatic sync runs while TVC is open, including in the tray. Overdue syncs run after you reopen the app.
    Schedules are separate for TV shows and movies. When no sync is running, overdue syncs start within a minute.
  </p>

  {#if loadError || actionError}
    <div role="alert" class="text-sm text-red-400 rounded-lg border border-red-500/30 bg-red-500/10 p-3 break-words">
      {#if loadError}<p>Could not load sync status: {loadError}</p>{/if}
      {#if actionError}<p>{actionError}</p>{/if}
      {#if loadError}<button type="button" onclick={() => refresh()} class="underline mt-2">Reload status</button>{/if}
    </div>
  {/if}

  {#if !libraries.length && !loadError}
    <div class="flex items-center justify-center gap-2 py-8 text-text-muted">
      <RefreshCw class="w-5 h-5 animate-spin" /> Loading sync status…
    </div>
  {/if}

  {#each libraries as library (library.kind)}
    {@const report = library.report}
    {@const completed = report ? report.succeeded + report.skipped + report.failures.length : 0}
    <section class="bg-background rounded-lg border border-border p-4 space-y-4" aria-label={library.kind === 'shows' ? 'TV show sync' : 'Movie sync'}>
      <div class="flex flex-wrap items-center justify-between gap-3">
        <h4 class="font-medium flex items-center gap-2">
          {#if library.kind === "shows"}<Tv class="w-5 h-5 text-accent" />TV shows
          {:else}<Film class="w-5 h-5 text-accent" />Movies{/if}
        </h4>
        <button type="button" onclick={() => sync(library.kind)} disabled={busy || !!loadError}
          class="px-3 py-1.5 text-sm bg-accent/20 hover:bg-accent/30 text-accent rounded disabled:opacity-50">
          {library.running ? "Syncing…" : "Sync now"}
        </button>
      </div>

      <div class="flex flex-wrap items-center gap-x-4 gap-y-2">
        <label for="sync-{library.kind}" class="text-sm text-text-muted">Automatic sync</label>
        <select id="sync-{library.kind}" value={library.frequency}
          onchange={(event) => saveSchedule(library, event.currentTarget)} disabled={saving !== null || !!loadError}
          class="bg-surface border border-border text-text rounded-lg px-3 py-2 text-sm focus:border-accent focus:outline-none disabled:opacity-50">
          <option value="off">Off</option>
          <option value="daily">Every day</option>
          <option value="weekly">Every week</option>
          <option value="monthly">Every month</option>
        </select>
        {#if saving === library.kind}<span class="text-xs text-text-muted">Saving…</span>{/if}
      </div>
      <p class="text-xs text-text-muted">Last complete success: {library.last_success_at ? formatDateTime(library.last_success_at) : "Never"}</p>
      {#if library.next_sync_at}
        <p class="text-xs text-text-muted">
          {#if library.running}Next scheduled sync is calculated from this run.
          {:else if new Date(library.next_sync_at).getTime() <= now}Due now — {busy ? 'waiting for the current sync.' : 'starting shortly.'}
          {:else}Next sync: {formatDateTime(library.next_sync_at)}{/if}
        </p>
      {/if}

      {#if report}
        <div class="border-t border-border pt-3 space-y-2">
          <div class="flex items-center gap-2 text-sm">
            {#if library.running}<RefreshCw class="w-4 h-4 animate-spin text-accent" />
            {:else if library.interrupted || report.error || report.failures.length}<AlertTriangle class="w-4 h-4 text-yellow-400" />
            {:else}<Check class="w-4 h-4 text-available" />{/if}
            <span>{report.source === 'retry' ? 'Retry' : report.source === 'automatic' ? 'Automatic sync' : 'Manual sync'} · {formatDateTime(report.started_at)}</span>
          </div>
          {#if library.running}
            <p class="text-sm text-text-muted break-words">{completed} / {report.total} processed{report.current_title ? ` · ${report.current_title}` : ''}</p>
            <progress value={completed} max={Math.max(report.total, 1)} aria-label="Sync progress" class="w-full h-1.5 accent-accent"></progress>
          {/if}
          {#if library.interrupted}
            <p class="text-sm text-yellow-400">This run was interrupted. Sync now to finish refreshing the library.</p>
          {/if}
          <p class="text-sm text-text-muted">
            <span class="text-available">{report.succeeded} synced</span> ·
            <span class={report.failures.length ? 'text-red-400' : ''}>{report.failures.length} failed</span> ·
            {report.skipped} skipped
          </p>
          {#if report.total === 0 && report.finished_at && !report.error}
            <p class="text-sm text-text-muted">{report.source === 'retry' ? 'No failed titles remain in this library.' : 'No saved titles to sync yet.'}</p>
          {/if}
          {#if report.error}<p class="text-sm text-red-400 break-words">{report.error}</p>{/if}
          {#if report.failures.length}
            <details open class="text-sm">
              <summary class="cursor-pointer text-red-400 py-1">Failed titles ({report.failures.length})</summary>
              <ul class="mt-2 space-y-2 max-h-56 overflow-auto">
                {#each report.failures as failure (failure.id)}
                  <li class="rounded bg-red-500/5 border border-red-500/20 p-2 break-words">
                    <p class="font-medium">{failure.title}</p>
                    <p class="text-xs text-text-muted mt-1">{failure.error}</p>
                  </li>
                {/each}
              </ul>
            </details>
            <button type="button" onclick={() => sync(library.kind, true)} disabled={busy || !!loadError}
              class="px-3 py-1.5 text-sm border border-border hover:bg-surface-hover rounded disabled:opacity-50">Retry failed</button>
          {/if}
        </div>
      {:else}
        <p class="text-sm text-text-muted">No sync recorded yet.</p>
      {/if}
    </section>
  {/each}

  <p class="text-xs text-text-muted">
    Includes archived and tier-list titles. Manual entries and unresolved TVDB matches are skipped.
    The latest result for each library is saved. Retries only refresh failed titles and keep the full-sync schedule.
    Every day means 24 hours, every week means 7 days, and every month means one calendar month after the last full attempt.
  </p>
</div>
