<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { RefreshCw, Tv, Film, AlertTriangle, Check, CircleX } from "lucide-svelte";
  import type { LibraryKind, LibrarySyncStatus, SyncFrequency, SyncReport } from "../../stores/librarySync";
  import { unacceptedSyncFailures, acceptedSyncFailures } from "../../stores/librarySync";
  import { formatDateTime } from "../../utils/dateFormat";
  import { setActiveTab as setDataTab } from "../../stores/dataManagement.svelte";
  import { setSidebarTab } from "../../stores/sidebar.svelte";

  let libraries = $state<LibrarySyncStatus[]>([]);
  let loadError = $state<string | null>(null);
  let actionError = $state<string | null>(null);
  let pending = $state(false);
  let saving = $state<LibraryKind | null>(null);
  let stopping = $state(false);
  let changingFailure = $state<string | null>(null);
  let failureFeedback = $state<string | null>(null);
  let failureFeedbackElement = $state<HTMLParagraphElement>();
  let savedScheduleStatusUnavailable = $state(false);
  let histories = $state<Record<LibraryKind, SyncReport[]>>({ shows: [], movies: [] });
  let historyLoading = $state<Record<LibraryKind, boolean>>({ shows: false, movies: false });
  let historyErrors = $state<Record<LibraryKind, string | null>>({ shows: null, movies: null });
  const historyOpen = new Set<LibraryKind>();
  const historyRequests = new Map<LibraryKind, Promise<void>>();
  let now = $state(Date.now());
  let disposed = false;
  let refreshPromise: Promise<void> | null = null;
  const busy = $derived(pending || libraries.some((library) => library.running));
  const stopRequested = $derived(stopping || libraries.some(library => library.stop_requested));

  function completed(report: SyncReport) {
    return report.processed ?? report.succeeded + report.skipped + report.failures.length;
  }

  function runLabel(report: SyncReport) {
    return report.source === 'retry' ? 'Retry' : report.source === 'automatic' ? 'Automatic sync' : 'Manual sync';
  }

  function showMatching() { setDataTab('unmapped'); setSidebarTab('data'); }

  function loadHistory(kind: LibraryKind): Promise<void> {
    const existing = historyRequests.get(kind);
    if (existing) return existing;
    historyLoading[kind] = true;
    historyErrors[kind] = null;
    const request = invoke<SyncReport[]>("get_library_sync_history", { kind })
      .then(result => { if (!disposed) histories[kind] = result; })
      .catch(error => { if (!disposed) historyErrors[kind] = String(error); })
      .finally(() => { historyRequests.delete(kind); if (!disposed) historyLoading[kind] = false; });
    historyRequests.set(kind, request);
    return request;
  }

  function toggleHistory(kind: LibraryKind, event: Event) {
    if ((event.currentTarget as HTMLDetailsElement).open) { historyOpen.add(kind); void loadHistory(kind); }
    else historyOpen.delete(kind);
  }

  function refresh(): Promise<void> {
    if (refreshPromise) return refreshPromise;
    refreshPromise = invoke<LibrarySyncStatus[]>("get_library_sync_status")
      .then((result) => {
        if (!disposed) {
          for (const library of result) {
            const previous = libraries.find(previous => previous.kind === library.kind)?.report;
            if (historyOpen.has(library.kind) && library.report?.finished_at &&
              (previous?.started_at !== library.report.started_at || previous?.finished_at !== library.report.finished_at)) {
              const existing = historyRequests.get(library.kind);
              if (existing) void existing.then(() => { if (!disposed && historyOpen.has(library.kind)) void loadHistory(library.kind); });
              else void loadHistory(library.kind);
            }
          }
          libraries = result; loadError = null; savedScheduleStatusUnavailable = false;
          if (!result.some(library => library.running)) stopping = false;
        }
      })
      .catch((error) => { if (!disposed) loadError = String(error); })
      .finally(() => { refreshPromise = null; });
    return refreshPromise;
  }

  async function stopSync() {
    stopping = true;
    actionError = null;
    try {
      await invoke("request_library_sync_stop");
      await refreshPromise;
      await refresh();
    } catch (error) {
      stopping = false;
      actionError = String(error);
    }
  }

  async function sync(kind: LibraryKind | null = null, retryFailed = false) {
    pending = true;
    actionError = null;
    failureFeedback = null;
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

  async function setFailureAccepted(kind: LibraryKind, report: SyncReport, failure: SyncReport['failures'][number], accepted: boolean) {
    if (busy || changingFailure) return;
    changingFailure = `${kind}:${failure.id}`;
    actionError = null;
    failureFeedback = null;
    try {
      const saved = await invoke<SyncReport>("set_library_sync_failure_accepted", {
        kind, startedAt: report.started_at, id: failure.id, accepted,
      });
      // Finish any older status read before publishing the persisted change.
      await refreshPromise;
      if (disposed) return;
      libraries = libraries.map(library => library.kind !== kind ? library : {
        ...library,
        report: library.report?.started_at === saved.started_at ? saved : library.report,
        last_full_report: library.last_full_report?.started_at === saved.started_at ? saved : library.last_full_report,
      });
      failureFeedback = accepted ? `Failure accepted for ${failure.title}. This title stays included in future full syncs.` : `Acceptance undone for ${failure.title}. You can retry this failure.`;
      // The clicked row moves to another list; keep keyboard focus in the page.
      await tick();
      if (!disposed) failureFeedbackElement?.focus();
      if (historyOpen.has(kind)) {
        await historyRequests.get(kind);
        if (!disposed) await loadHistory(kind);
      }
    } catch (error) {
      if (!disposed) actionError = `Could not ${accepted ? 'accept this failure' : 'undo acceptance'}: ${error}`;
    } finally { if (!disposed) changingFailure = null; }
  }

  async function saveSchedule(library: LibrarySyncStatus, select: HTMLSelectElement) {
    const frequency = select.value as SyncFrequency;
    // Keep showing the persisted value until saving succeeds.
    select.value = library.frequency;
    saving = library.kind;
    actionError = null;
    savedScheduleStatusUnavailable = false;
    try {
      await invoke("set_library_sync_schedule", { kind: library.kind, frequency });
      // The write succeeded even if the following status read fails.
      libraries = libraries.map(saved => saved.kind === library.kind ? { ...saved, frequency } : saved);
      select.value = frequency;
      await refreshPromise;
      await refresh();
      if (loadError) savedScheduleStatusUnavailable = true;
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

<div class="focus-sync-content space-y-5">
  <div class="space-y-2">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div class="flex flex-wrap items-center gap-x-4 gap-y-1">
        <h2 class="text-base font-semibold text-text">TMDB sync</h2>
        <p class="text-sm text-text-muted">Automatic sync runs while TVC is open or in the tray.</p>
      </div>
      <button type="button" onclick={() => sync()} disabled={busy || changingFailure !== null || saving !== null || !libraries.length || !!loadError}
        class="shrink-0 flex items-center gap-2 px-3 py-2 text-sm bg-accent/20 hover:bg-accent/30 text-accent rounded-lg disabled:opacity-50">
        <RefreshCw class="w-4 h-4 {busy ? 'animate-spin' : ''}" />
        Sync All
      </button>
      {#if libraries.some(library => library.running)}
        <button type="button" onclick={stopSync} disabled={stopRequested} class="focus-button disabled:opacity-50">
          {stopRequested ? 'Stopping after current title…' : 'Stop after current title'}
        </button>
      {/if}
    </div>
    <p class="text-xs text-text-muted">
      Shows and movies have separate schedules. After reopening, overdue syncs start within a minute once no other sync is running.
    </p>
  </div>

  {#if loadError || actionError}
    <div role="alert" class="text-sm text-error rounded-lg border border-error/30 bg-error/10 p-3 break-words">
      <CircleX size={16} class="mb-1" aria-hidden="true" />
      {#if loadError}<p>Could not load sync status: {loadError}</p>{/if}
      {#if savedScheduleStatusUnavailable}<p>Your schedule was saved. Sync status is temporarily unavailable.</p>{/if}
      {#if actionError}<p>{actionError}</p>{/if}
      {#if loadError}<button type="button" onclick={() => refresh()} class="underline mt-2">Reload status</button>{/if}
    </div>
  {/if}

  {#if failureFeedback}<p role="status" tabindex="-1" bind:this={failureFeedbackElement} class="text-sm text-text-muted">{failureFeedback}</p>{/if}

  {#if !libraries.length && !loadError}
    <div class="flex items-center justify-center gap-2 py-8 text-text-muted">
      <RefreshCw class="w-5 h-5 animate-spin" /> Loading sync status…
    </div>
  {/if}

  <div class="focus-sync-grid">
  {#each libraries as library (library.kind)}
    {@const report = library.report}
    {@const processed = report ? completed(report) : 0}
    {@const failures = unacceptedSyncFailures(report)}
    {@const accepted = acceptedSyncFailures(report)}
    <section class="focus-panel bg-surface rounded-xl border border-border p-5 space-y-4" aria-label={library.kind === 'shows' ? 'TV show sync' : 'Movie sync'}>
      <div class="flex flex-wrap items-center justify-between gap-3">
        <h3 class="font-semibold flex items-center gap-2">
          {#if library.kind === "shows"}<Tv class="w-5 h-5 text-accent" />TV shows
          {:else}<Film class="w-5 h-5 text-accent" />Movies{/if}
        </h3>
        <button type="button" onclick={() => sync(library.kind)} disabled={busy || changingFailure !== null || !!loadError}
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
      <p class="text-xs text-text-muted">Last successful full sync: {library.last_success_at ? formatDateTime(library.last_success_at) : "Never"}</p>
      {#if library.next_sync_at}
        <p class="text-xs text-text-muted">
          {#if library.running}Next scheduled sync is calculated from this run.
          {:else if new Date(library.next_sync_at).getTime() <= now}Due now — {busy ? 'waiting for the current sync.' : 'starting shortly.'}
          {:else}Next sync: {formatDateTime(library.next_sync_at)}{/if}
        </p>
      {/if}

      {#if report}
        <div class="border-t border-border pt-3 space-y-2">
          {#if report.release_country}<p class="text-xs text-text-muted">Movie release country: {report.release_country}</p>{/if}
          <div class="flex items-center gap-2 text-sm">
            {#if library.running}<RefreshCw class="w-4 h-4 animate-spin text-accent" />
            {:else if library.interrupted || report.stopped || report.error || failures.length}<AlertTriangle class="w-4 h-4 text-warning" />
            {:else}<Check class="w-4 h-4 {accepted.length ? 'text-text-muted' : 'text-success'}" />{/if}
            <span>{runLabel(report)} · {formatDateTime(report.started_at)} · {library.running ? 'Syncing' : report.stopped ? 'Stopped' : library.interrupted ? 'Interrupted' : report.error || failures.length ? 'Needs attention' : accepted.length ? 'Failures accepted' : 'Completed'}</span>
          </div>
          {#if library.running}
            <p class="text-sm text-text-muted break-words">{processed} / {report.total} processed{report.current_title ? ` · ${report.current_title}` : ''}</p>
            {#if library.stop_requested || stopping}<p class="text-xs text-text-muted">Finishing the current title before stopping.</p>{/if}
            <progress value={processed} max={Math.max(report.total, 1)} aria-label="Sync progress" class="w-full h-1.5 accent-accent"></progress>
          {/if}
          {#if report.stopped}
            <p class="text-sm text-warning">Stopped after {processed} / {report.total} titles. Sync now starts a new full run. Automatic sync waits until its next scheduled run.</p>
          {/if}
          {#if library.interrupted}
            <p class="text-sm text-warning">This run was interrupted. Sync now to finish refreshing the library.</p>
          {/if}
          <p class="text-sm text-text-muted">
            <span class="text-success">{report.succeeded} synced</span> ·
            <span class={failures.length ? 'text-error' : ''}>{report.failures.length} failed{accepted.length ? ` (${accepted.length} accepted)` : ''}</span> ·
            {report.skipped} skipped
          </p>
          {#if report.total === 0 && report.finished_at && !report.error}
            <p class="text-sm text-text-muted">{report.source === 'retry' ? 'No failed titles remain in this library.' : 'No saved titles to sync yet.'}</p>
          {/if}
          {#if report.error}<p class="text-sm text-error break-words">{report.error}</p>{/if}
          {#if report.skipped > 0}
            <details class="text-sm">
              <summary class="cursor-pointer text-text-muted py-1">Skipped titles ({report.skipped})</summary>
              {#if report.skipped_titles?.length}
                <ul class="mt-2 space-y-2 max-h-56 overflow-auto">
                  {#each report.skipped_titles as title (title.id)}
                    <li class="rounded bg-background border border-border p-2">
                      <p class="font-medium">{title.title}</p>
                      <p class="text-xs text-text-muted mt-1">{title.reason === 'needs_matching' ? 'This show needs a TMDB match before it can sync.' : 'Manual title with no TMDB record. Its saved details are kept.'}</p>
                    </li>
                  {/each}
                </ul>
                {#if report.skipped_titles.some(title => title.reason === 'needs_matching')}
                  <button type="button" onclick={showMatching} class="focus-button mt-2">Match shows in Maintenance</button>
                {/if}
              {:else}<p class="text-xs text-text-muted mt-2">This older run recorded a count without the title details.</p>{/if}
            </details>
          {/if}
          {#if failures.length}
            <details open class="text-sm">
              <summary class="cursor-pointer text-error py-1">Needs attention ({failures.length})</summary>
              <ul class="mt-2 space-y-2 max-h-56 overflow-auto">
                {#each failures as failure (failure.id)}
                  <li class="rounded bg-error/5 border border-error/20 p-2 break-words">
                    <div class="flex flex-wrap items-center justify-between gap-2">
                      <p class="font-medium">{failure.title}</p>
                      <button type="button" class="focus-button text-xs" disabled={busy || changingFailure !== null || !!loadError || !report.finished_at}
                        aria-label={`Accept failure for ${failure.title}`} onclick={() => setFailureAccepted(library.kind, report, failure, true)}>
                        <Check size={14} aria-hidden="true" />{changingFailure === `${library.kind}:${failure.id}` ? 'Saving…' : 'Accept failure'}
                      </button>
                    </div>
                    <p class="text-xs text-text-muted mt-1">{failure.error}</p>
                  </li>
                {/each}
              </ul>
            </details>
            <p class="text-xs text-text-muted">Accept a failure to clear its reminder for this run. Full syncs will still try the title again.</p>
            <button type="button" onclick={() => sync(library.kind, true)} disabled={busy || changingFailure !== null || !!loadError}
              class="px-3 py-1.5 text-sm border border-border hover:bg-surface-hover rounded disabled:opacity-50">Retry failed</button>
          {/if}
          {#if accepted.length}
            <details class="text-sm">
              <summary class="cursor-pointer text-text-muted py-1">Accepted failures ({accepted.length})</summary>
              <p class="text-xs text-text-muted mt-2">Kept for your records. Full syncs still include these titles; any new failure will need attention again.</p>
              <ul class="mt-2 space-y-2 max-h-56 overflow-auto">
                {#each accepted as failure (failure.id)}
                  <li class="rounded bg-background border border-border p-2 break-words">
                    <div class="flex flex-wrap items-center justify-between gap-2">
                      <p class="font-medium flex items-center gap-2"><Check size={14} aria-hidden="true" />{failure.title} <span class="text-xs text-text-muted">Accepted</span></p>
                      <button type="button" class="focus-button text-xs" disabled={busy || changingFailure !== null || !!loadError}
                        aria-label={`Undo acceptance for ${failure.title}`} onclick={() => setFailureAccepted(library.kind, report, failure, false)}>
                        {changingFailure === `${library.kind}:${failure.id}` ? 'Saving…' : 'Undo'}
                      </button>
                    </div>
                    <p class="text-xs text-text-muted mt-1">{failure.error}</p>
                  </li>
                {/each}
              </ul>
            </details>
          {/if}
        </div>
      {:else}
        <p class="text-sm text-text-muted">No sync recorded yet.</p>
      {/if}
      {#if report?.source === 'retry' && library.last_full_report}
        {@const full = library.last_full_report}
        <details class="border-t border-border pt-3 text-sm">
          <summary class="cursor-pointer text-text-muted">Original full-run result · {formatDateTime(full.started_at)}</summary>
          <p class="mt-2 text-text-muted">{completed(full)} / {full.total} processed · {full.succeeded} synced · {full.failures.length} failed · {full.skipped} skipped{full.stopped ? ' · Stopped' : !full.finished_at ? ' · Interrupted' : ''}</p>
          <p class="text-xs text-text-muted mt-1">These are the results before retries. The latest retry above shows which failures remain.</p>
          {#if full.stopped || !full.finished_at}<p class="text-xs text-warning mt-1">This full run ended early. Start a full sync to refresh its remaining titles.</p>{/if}
        </details>
      {/if}
      <details class="border-t border-border pt-3 text-sm" ontoggle={(event) => toggleHistory(library.kind, event)}>
        <summary class="cursor-pointer text-text-muted">Recent runs · last five</summary>
        {#if historyLoading[library.kind]}<p role="status" class="text-xs text-text-muted mt-2">Loading history…</p>{/if}
        {#if historyErrors[library.kind]}
          <p role="alert" class="text-xs text-error mt-2">Could not load history: {historyErrors[library.kind]}</p>
          <button type="button" class="focus-button mt-2" onclick={() => loadHistory(library.kind)}>Retry</button>
        {:else if !historyLoading[library.kind] && !histories[library.kind].length}
          <p class="text-xs text-text-muted mt-2">No completed runs recorded yet.</p>
        {/if}
        <div class="space-y-2 mt-3">
          {#each histories[library.kind] as run (run.started_at)}
            <details class="rounded border border-border bg-background p-3">
              <summary class="cursor-pointer">
                <span>{runLabel(run)} · {formatDateTime(run.started_at)}</span>
                <span class="text-xs {run.stopped || !run.finished_at || run.error || unacceptedSyncFailures(run).length ? 'text-warning' : acceptedSyncFailures(run).length ? 'text-text-muted' : 'text-success'}">{run.stopped ? 'Stopped' : !run.finished_at ? 'Interrupted' : run.error || unacceptedSyncFailures(run).length ? 'Needs attention' : acceptedSyncFailures(run).length ? 'Failures accepted' : 'Completed'}</span>
              </summary>
              <p class="text-xs text-text-muted mt-2">{completed(run)} / {run.total} processed · {run.succeeded} synced · {run.failures.length} failed · {run.skipped} skipped</p>
              {#if run.release_country}<p class="text-xs text-text-muted mt-1">Movie release country: {run.release_country}</p>{/if}
              {#if run.error}<p class="text-xs text-error mt-2 break-words">{run.error}</p>{/if}
              {#if run.failures.length}
                <ul class="space-y-1 mt-2 max-h-40 overflow-auto">{#each run.failures as failure}<li class="text-xs break-words"><span class="font-medium">{failure.title}</span>{failure.accepted ? ' · Accepted' : ''} · {failure.error}</li>{/each}</ul>
              {/if}
              {#if run.skipped_titles?.length}
                <ul class="space-y-1 mt-2 max-h-40 overflow-auto">{#each run.skipped_titles as title}<li class="text-xs text-text-muted"><span class="font-medium">{title.title}</span> · {title.reason === 'needs_matching' ? 'Needs a TMDB match' : 'Manual title'}</li>{/each}</ul>
              {/if}
            </details>
          {/each}
        </div>
      </details>
    </section>
  {/each}
  </div>

  <details class="text-xs text-text-muted">
    <summary class="cursor-pointer py-1">How sync works</summary>
    <p class="mt-2 leading-relaxed">
    Includes archived and tier-list titles. Manual entries and unresolved TVDB matches are skipped.
    The last five runs and the latest full result are kept. Retries only refresh failures that have not been accepted and keep the full-sync schedule.
    Accepting a failure keeps its error in history and clears its reminder for that run. Use Undo before starting another run to make it available for retry again.
    Full syncs always include accepted titles. A new failure needs attention again.
    Every day means 24 hours, every week means 7 days, and every month means one calendar month after the last full attempt.
    Movie runs keep the release country chosen when they started. Changing it applies to the next run.
    Stopping finishes the current title, skips any queued library, and postpones automatic sync until the next scheduled interval.
    </p>
  </details>
</div>

<style>
  .focus-sync-content { container-type: inline-size; }
  .focus-sync-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 20px; align-items: start; }
  @container (max-width: 740px) { .focus-sync-grid { grid-template-columns: 1fr; } }
</style>
