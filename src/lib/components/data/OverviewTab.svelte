<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { RefreshCw, CloudDownload, Download, HelpCircle, ArrowRight } from "lucide-svelte";
  import { getDatabaseStats, getError, isLoading, setActiveTab } from "../../stores/dataManagement.svelte";
  import { getUnmigratedShows, getUnmigratedShowsError as getUnmigratedError, loadUnmigratedShows } from "../../stores/migration.svelte";
  import { getBackupStatus, getBackupError, loadBackupStatus } from "../../stores/backups.svelte";
  import type { LibrarySyncStatus } from "../../stores/librarySync";
  import { unacceptedSyncFailures, acceptedSyncFailures } from "../../stores/librarySync";
  import { formatDateTime } from "../../utils/dateFormat";

  let sync = $state<LibrarySyncStatus[]>([]);
  let syncError = $state<string | null>(null);
  let healthLoading = $state(true);
  let healthReady = $state(false);
  let healthRequest: Promise<void> | null = null;
  const stats = $derived(getDatabaseStats());
  const backup = $derived(getBackupStatus());
  const failed = $derived(sync.reduce((sum, item) => sum + unacceptedSyncFailures(item.report).length, 0));
  const accepted = $derived(sync.reduce((sum, item) => sum + acceptedSyncFailures(item.report).length, 0));
  function loadHealth(): Promise<void> {
    if (healthRequest) return healthRequest;
    healthLoading = true;
    healthRequest = Promise.all([
      invoke<LibrarySyncStatus[]>("get_library_sync_status").then(value => { sync = value; syncError = null; }).catch(error => { syncError = `Could not load sync status: ${String(error)}`; }),
      loadBackupStatus(), loadUnmigratedShows(),
    ]).then(() => {}).finally(() => { healthLoading = false; healthReady = true; healthRequest = null; });
    return healthRequest;
  }
  onMount(() => {
    let disposed = false;
    const stops: UnlistenFn[] = [];
    void loadHealth();
    for (const event of ["library-sync-finished", "library-sync-status-changed"]) {
      listen(event, async () => { await healthRequest; if (!disposed) await loadHealth(); }).then(unlisten => { if (disposed) unlisten(); else stops.push(unlisten); }).catch(error => { syncError = `Could not listen for sync updates: ${String(error)}`; });
    }
    const timer = setInterval(() => { void loadHealth(); }, 10000);
    return () => { disposed = true; for (const stop of stops) stop(); clearInterval(timer); };
  });
</script>

<div class="space-y-5">
  {#if getError()}
    <div role="alert" class="focus-panel p-4 text-error"><p>{getError()}</p><button type="button" class="focus-button mt-3" onclick={() => setActiveTab("overview")}>Reload summary</button></div>
  {:else if stats}
    <section aria-label="Library totals" class="grid grid-cols-3 gap-3">
      {#each [{ name: "TV shows", count: stats.total_shows }, { name: "Episodes", count: stats.total_episodes }, { name: "Movies", count: stats.total_movies }] as item}
        <div class="focus-panel px-4 py-3"><p class="text-xl font-semibold">{item.count}</p><p class="text-sm text-text-muted">{item.name}</p></div>
      {/each}
    </section>
    <p class="text-xs text-text-muted">Totals include archived titles and titles kept only in tier lists.</p>
  {:else if isLoading()}
    <p class="flex items-center gap-2 text-sm text-text-muted"><RefreshCw size={16} class="animate-spin" /> Loading library summary…</p>
  {/if}

  <section aria-label="Library status" class="space-y-3">
    <div class="focus-panel p-4 flex flex-wrap items-center justify-between gap-3">
      <div class="flex items-start gap-3"><CloudDownload size={19} class="text-accent mt-0.5" /><div><h2 class="font-medium">Sync with TMDB</h2>
        {#if syncError}<p role="alert" class="text-sm text-error">{syncError}</p>
        {:else if healthLoading && !sync.length}<p class="text-sm text-text-muted">Loading sync status…</p>
        {:else}<p class="text-sm text-text-muted">{sync.some(item => item.running) ? "Your library is syncing." : sync.some(item => item.report?.error) ? "The latest sync failed. Open its results to retry." : failed ? `${failed} title${failed === 1 ? " needs" : "s need"} another sync attempt.` : sync.some(item => item.interrupted || item.report?.stopped || item.last_full_report?.stopped || item.last_full_report && !item.last_full_report.finished_at) ? "A sync stopped before finishing. Review its results." : accepted ? `${accepted} sync failure${accepted === 1 ? " is" : "s are"} accepted. ${accepted === 1 ? "This title stays" : "These titles stay"} included in future full syncs.` : sync.every(item => item.last_success_at) && sync.length === 2 ? "Both libraries have completed a full sync." : "Review sync results and choose an automatic schedule."}</p>{/if}
      </div></div>
      <button type="button" class="focus-button" onclick={() => setActiveTab("sync")}>Open sync <ArrowRight size={15} /></button>
    </div>
    <div class="focus-panel p-4 flex flex-wrap items-center justify-between gap-3">
      <div class="flex items-start gap-3"><HelpCircle size={19} class="text-accent mt-0.5" /><div><h2 class="font-medium">Match older shows</h2>
        {#if getUnmigratedError()}<p role="alert" class="text-sm text-error">{getUnmigratedError()}</p>
        {:else if !healthReady}<p class="text-sm text-text-muted">Loading matching status…</p>
        {:else}<p class="text-sm text-text-muted">{getUnmigratedShows().length ? `${getUnmigratedShows().length} show${getUnmigratedShows().length === 1 ? " needs" : "s need"} a TMDB match. Existing watch history is kept.` : "No shows are waiting for a TMDB match."}</p>{/if}
      </div></div>
      <button type="button" class="focus-button" onclick={() => setActiveTab("unmapped")}>Match shows <ArrowRight size={15} /></button>
    </div>
    <div class="focus-panel p-4 flex flex-wrap items-center justify-between gap-3">
      <div class="flex items-start gap-3"><Download size={19} class="text-accent mt-0.5" /><div><h2 class="font-medium">Backups</h2>
        {#if getBackupError()}<p role="alert" class="text-sm text-error">{getBackupError()}</p>
        {:else if !backup}<p class="text-sm text-text-muted">Loading backup status…</p>
        {:else if backup?.last_error}<p role="alert" class="text-sm text-warning">Latest backup needs attention: {backup.last_error}</p>
        {:else}<p class="text-sm text-text-muted">{backup?.last_success_at ? `Last backup: ${formatDateTime(backup.last_success_at)}.` : "No successful backup has been recorded yet."} {backup?.preferences.frequency === "off" ? "Automatic backups are off." : backup?.next_due_at ? `Next: ${formatDateTime(backup.next_due_at)}.` : ""}</p>{/if}
      </div></div>
      <button type="button" class="focus-button" onclick={() => setActiveTab("backups")}>Open backups <ArrowRight size={15} /></button>
    </div>
  </section>
  {#if syncError || getUnmigratedError() || getBackupError()}<button type="button" class="focus-button" disabled={healthLoading} onclick={loadHealth}><RefreshCw size={15} /> Retry status</button>{/if}
  <p class="text-xs text-text-muted">Archive keeps titles you have put aside. Maintenance contains matching, duplicate checks and optional cleanup tools.</p>
</div>
