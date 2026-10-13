<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { X, RefreshCw, ChevronDown, ExternalLink, Check, AlertTriangle, CircleX } from "lucide-svelte";
  import UnmigratedShowsResolver from "../UnmigratedShowsResolver.svelte";
  import { getMaintenanceSection, getError, isLoading, isDataWriting, getDatabaseStats, getDuplicates, getOrphanedEpisodes, getUnairedEpisodes, getCleanupPreviewError, isCleanupPreviewLoading, loadDatabaseStats, loadCleanupPreviews, setActiveTab, cleanupOrphaned, cleanupUnaired, optimizeDatabase, runFullCleanup, mergeDuplicates, formatBytes, type DuplicatePair } from "../../stores/dataManagement.svelte";
  import { openConfirmDialog } from "../../stores/confirmDialog.svelte";
  import { simulateDummyUpdate } from "../../stores/updates.svelte";
  import { getThemeSettings } from "../../stores/theme.svelte";
  import { getDatabaseSummaryError } from "../../stores/dataManagement.svelte";
  let cleanupMessage = $state<string | null>(null);
  let messageKind = $state<"success" | "error" | "warning">("success");
  let messageTimer: ReturnType<typeof setTimeout> | null = null;
  let cleanupPending = $state(false);
  let dummyUpdateVersion = $state("0.8.0");
  let dummyUpdateNotes = $state("### Preview update\n\n- Example release note for testing the update dialog.");
  let devFakeQuarantineCount = $state(3);
  let devMigrationRunning = $state(false);
  let advanced = $state(false);
  const section = $derived(getMaintenanceSection());
  const stats = $derived(getDatabaseStats());
  const busy = $derived(cleanupPending || isDataWriting() || devMigrationRunning);
  function showResult(message: string, kind: "success" | "error" | "warning" = "success", duration = 0) {
    if (messageTimer) clearTimeout(messageTimer);
    cleanupMessage = message; messageKind = kind;
    messageTimer = duration ? setTimeout(() => { cleanupMessage = null; messageTimer = null; }, duration) : null;
  }
  onDestroy(() => { if (messageTimer) clearTimeout(messageTimer); });
  onMount(() => { void loadDatabaseStats(); });
  function year(date: string | null) { return date?.match(/^\d{4}/)?.[0] ?? "Unknown year"; }
  async function tmdb(id: number) {
    try { await openUrl(`https://www.themoviedb.org/tv/${id}`); }
    catch (error) { showResult(`Could not open TMDB: ${String(error)}`, "error"); }
  }
  async function handleDevForceRerunMigration() {
    if (busy) return;
    const approved = await openConfirmDialog({ title: "Repeat the development matching test?", message: "This resets TVDB matching in the development database and can leave shows waiting for a match. Use a disposable development library. Existing watch history is retained by the matching flow.", type: "danger", confirmLabel: "Run development test", cancelLabel: "Cancel" });
    if (!approved) return;
    devMigrationRunning = true;
    try { await invoke<number>("dev_force_rerun_migration", { fakeQuarantineCount: devFakeQuarantineCount }); showResult("Development matching test started."); }
    catch (error) { showResult(`Matching test failed: ${String(error)}`, "error"); }
    finally { devMigrationRunning = false; }
  }
  async function confirmCleanup(kind: "orphaned" | "undated" | "all") {
    const [statsOk, previewsOk] = await Promise.all([loadDatabaseStats(), loadCleanupPreviews()]);
    if (!statsOk || !previewsOk) {
      showResult(getDatabaseSummaryError() ?? getCleanupPreviewError() ?? "Cleanup information could not load. Open Cleanup and retry the summary and previews.", "error");
      return false;
    }
    const stats = getDatabaseStats();
    if (!stats) return false;
    const orphaned = kind === "undated" ? 0 : stats.orphaned_episodes;
    const undated = kind === "orphaned" ? 0 : stats.unaired_unscheduled_episodes;
    const total = orphaned + undated;
    if (!total) { showResult("No eligible episode records to remove."); return false; }
    return openConfirmDialog({
      title: kind === "all" ? "Run full cleanup?" : "Remove unused episode records?",
      message: `Remove ${total} episode record${total === 1 ? "" : "s"}: ${orphaned} without a linked show and ${undated} without an air date? Only records without watch history, schedules, ratings, tags or change history are eligible. Tier-only and unresolved shows are protected. The preview shows at most 20 records; this action applies to all ${total}. ${kind === "all" ? "Database optimization runs separately afterwards. " : ""}This cannot be undone.`,
      type: "danger", confirmLabel: `Remove ${total} record${total === 1 ? "" : "s"}`, cancelLabel: "Keep records",
    });
  }

  async function handleCleanupOrphaned() {
    if (cleanupPending) return;
    cleanupPending = true;
    try {
      if (!await confirmCleanup("orphaned")) return;
      const count = await cleanupOrphaned();
      showResult(`Removed ${count} unused orphaned episode record${count !== 1 ? "s" : ""}`, "success", 5000);
    } catch (error) { showResult(`Cleanup failed: ${error}`, "error"); }
    finally { cleanupPending = false; }
  }

  async function handleCleanupUnaired() {
    if (cleanupPending) return;
    cleanupPending = true;
    try {
      if (!await confirmCleanup("undated")) return;
      const count = await cleanupUnaired();
      showResult(`Removed ${count} unused episode record${count !== 1 ? "s" : ""} without dates`, "success", 5000);
    } catch (error) { showResult(`Cleanup failed: ${error}`, "error"); }
    finally { cleanupPending = false; }
  }

  async function handleOptimize() {
    if (cleanupPending) return;
    cleanupPending = true;
    try {
      await optimizeDatabase();
      showResult("Database optimized successfully", "success", 5000);
    } catch (error) { showResult(`Optimization failed: ${error}`, "error"); }
    finally { cleanupPending = false; }
  }

  async function handleFullCleanup() {
    if (cleanupPending) return;
    cleanupPending = true;
    try {
      if (!await confirmCleanup("all")) return;
      const result = await runFullCleanup();
      const message = `Removed ${result.orphaned_episodes_removed} unused orphaned records and ${result.unaired_episodes_removed} unused records without dates.`;
      showResult(result.optimization_error ? `${message} Optimization did not finish: ${result.optimization_error}` : `${message} Database optimized.`, result.optimization_error ? "warning" : "success");
    } catch (error) { showResult(`Cleanup failed: ${error}`, "error"); }
    finally { cleanupPending = false; }
  }

  async function handleMerge(pair: DuplicatePair, keepFirst: boolean) {
    if (busy) return;
    cleanupPending = true;
    try {
    const keepId = keepFirst ? pair.show1_id : pair.show2_id;
    const mergeId = keepFirst ? pair.show2_id : pair.show1_id;
    const keepName = keepFirst ? pair.show1_name : pair.show2_name;

    const confirmed = await openConfirmDialog({
      title: "Merge Shows",
      message: `Keep "${keepName}" (${year(keepFirst ? pair.show1_first_aired : pair.show2_first_aired)}) and combine the other entry into it? Watched episodes and schedules are combined; values already set on the kept entry take priority. The other entry is removed. This cannot be undone.`,
      type: "warning",
      confirmLabel: "Merge",
      cancelLabel: "Cancel",
    });

    if (confirmed) {
      try {
        const result = await mergeDuplicates(keepId, mergeId);
        showResult(`Merged: ${result.episodes_moved} episodes moved, ${result.episodes_merged} combined`, "success", 5000);
      } catch (error) { showResult(`Merge failed: ${error}`, "error"); }
    }
    } finally { cleanupPending = false; }
  }
</script>

<div class="space-y-4">
  <nav class="focus-tabs" aria-label="Maintenance sections">
    {#each [{ id: "duplicates", label: "Duplicates" }, { id: "cleanup", label: "Cleanup" }, { id: "unmapped", label: "Match shows" }] as item}
      <button type="button" aria-pressed={section === item.id} disabled={busy} class:active={section === item.id} onclick={() => setActiveTab(item.id as "duplicates" | "cleanup" | "unmapped")}>{item.label}</button>
    {/each}
  </nav>
  {#if cleanupMessage}
    <div role={messageKind === "error" ? "alert" : "status"} class="p-3 rounded-lg border text-sm flex items-start justify-between gap-3 {messageKind === 'error' ? 'border-error/30 bg-error/10 text-error' : messageKind === 'warning' ? 'border-warning/30 bg-warning/10 text-warning' : 'border-success/30 bg-success/10 text-success'}">
      <span class="flex items-start gap-2">{#if messageKind === 'error'}<CircleX size={16} class="shrink-0 mt-0.5" />{:else if messageKind === 'warning'}<AlertTriangle size={16} class="shrink-0 mt-0.5" />{:else}<Check size={16} class="shrink-0 mt-0.5" />{/if}<span>{cleanupMessage}</span></span><button type="button" aria-label="Dismiss result" onclick={() => { if (messageTimer) clearTimeout(messageTimer); cleanupMessage = null; }}><X size={16} /></button>
    </div>
  {/if}
  {#if section === "unmapped"}
    <div><h2 class="font-medium">Match shows to TMDB</h2><p class="text-sm text-text-muted mt-1">Search by title, then compare the year and description before choosing a match. Matching keeps your existing watch history.</p></div>
    <UnmigratedShowsResolver open={true} variant="inline" />
  {:else if getError()}
    <div role="alert" class="focus-panel p-4 text-error"><p>{getError()}</p><button type="button" class="focus-button mt-3" disabled={busy} onclick={() => setActiveTab(section)}>Reload maintenance</button></div>
  {:else if isLoading() && !busy}
    <p class="text-sm text-text-muted flex items-center gap-2"><RefreshCw size={16} class="animate-spin" /> Loading maintenance…</p>
  {:else if section === "duplicates"}
    <p class="text-sm text-text-muted">Compare likely duplicate shows before combining them. Choose which entry to keep.</p>
    {#if getDuplicates().length === 0}<div class="focus-panel p-6 flex items-center gap-3"><Check size={22} class="text-success" /><p>No possible duplicates found.</p></div>{/if}
    {#each getDuplicates() as pair}
      <article class="focus-panel p-4 space-y-3">
        <p class="text-sm text-text-muted">{pair.similarity_reason}</p>
        <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
          {#each [{ first: true, name: pair.show1_name, date: pair.show1_first_aired, tmdb: pair.show1_tmdb_id, episodes: pair.show1_episode_count, watched: pair.show1_watched_count, poster: pair.show1_poster_url }, { first: false, name: pair.show2_name, date: pair.show2_first_aired, tmdb: pair.show2_tmdb_id, episodes: pair.show2_episode_count, watched: pair.show2_watched_count, poster: pair.show2_poster_url }] as title}
            <div class="border border-border rounded-lg p-3 space-y-3">
              <div class="flex gap-3 items-start">
                {#if title.poster && !getThemeSettings().hidePosters}<img src={title.poster} alt="" class="w-10 h-14 rounded object-cover" />{/if}
                <div class="min-w-0"><h3 class="font-medium break-words">{title.name}</h3><p class="text-xs text-text-muted">{year(title.date)} · {title.episodes} episodes · {title.watched} watched</p></div>
              </div>
              {#if title.tmdb}<button type="button" class="text-xs text-accent flex items-center gap-1" onclick={() => tmdb(title.tmdb!)}>TMDB #{title.tmdb} <ExternalLink size={12} /></button>{/if}
              <button type="button" class="focus-button w-full justify-center" disabled={busy} onclick={() => handleMerge(pair, title.first)}>Keep this entry and merge</button>
            </div>
          {/each}
        </div>
      </article>
    {/each}
  {:else if stats}
    <p class="text-sm text-text-muted">Optional removal of unused episode records. Watch history, schedules, ratings, tags and change history are protected, along with tier-only and unmatched shows.</p>
    {#if getCleanupPreviewError()}<div role="alert" class="p-3 border border-error/30 rounded-lg text-sm text-error"><p>{getCleanupPreviewError()}</p><button type="button" class="focus-button mt-2" disabled={busy} onclick={loadCleanupPreviews}>Retry previews</button></div>{/if}
    {#each [{ kind: "orphaned", label: "Episodes without a linked show", count: stats.orphaned_episodes, note: "These records no longer belong to a show in your library.", rows: getOrphanedEpisodes(), run: handleCleanupOrphaned }, { kind: "undated", label: "Unused episodes without an air date", count: stats.unaired_unscheduled_episodes, note: "Future episodes can legitimately have no date. Remove these only if you want to tidy unused records; syncing can add them again.", rows: getUnairedEpisodes(), run: handleCleanupUnaired }] as group}
      <article class="focus-panel p-4 space-y-3">
        <div class="flex flex-wrap justify-between items-start gap-3"><div><h3 class="font-medium">{group.label} <span class="text-text-muted">({group.count})</span></h3><p class="text-sm text-text-muted mt-1">{group.note}</p></div><button type="button" class="focus-button" disabled={busy || isCleanupPreviewLoading() || !!getCleanupPreviewError() || group.count === 0} onclick={group.run}>Remove unused records</button></div>
        <details><summary class="text-sm text-accent cursor-pointer">Preview affected episodes</summary><div class="mt-2 max-h-48 overflow-auto text-sm space-y-1">
          <p class="text-xs text-text-muted mb-2">Showing {Math.min(group.rows.length, 20)} of {group.count} eligible records</p>
          {#each group.rows.slice(0, 20) as episode}<p>{episode.show_name} <span class="text-text-muted">· S{String(episode.season_number).padStart(2, "0")}E{String(episode.episode_number).padStart(2, "0")}{episode.name ? ` · ${episode.name}` : ""}</span></p>{/each}
        </div></details>
      </article>
    {/each}
  {/if}
  <div class="focus-panel p-4">
    <button type="button" class="flex items-center gap-2 font-medium text-sm" aria-expanded={advanced} onclick={() => { advanced = !advanced; if (advanced && !stats) void loadDatabaseStats(); }}><ChevronDown size={16} class={advanced ? "rotate-180" : ""} /> Advanced maintenance</button>
    {#if advanced}
      <div class="mt-4 space-y-4">
        <p class="text-sm text-text-muted">Database size: {stats ? formatBytes(stats.database_size_bytes) : "Unavailable"}</p>
        {#if getDatabaseSummaryError()}<div role="alert" class="text-sm text-error"><p>{getDatabaseSummaryError()}</p><button type="button" class="focus-button mt-2" disabled={busy} onclick={loadDatabaseStats}>Retry database summary</button></div>{/if}
        <div class="flex flex-wrap items-center justify-between gap-3"><div><h3 class="text-sm font-medium">Optimize database</h3><p class="text-xs text-text-muted">Reclaim unused space without deleting your library.</p></div><button type="button" class="focus-button" disabled={busy} onclick={handleOptimize}>Optimize</button></div>
        <div class="flex flex-wrap items-center justify-between gap-3"><div><h3 class="text-sm font-medium">Full cleanup</h3><p class="text-xs text-text-muted">Remove both eligible groups, then optimize. Review the Cleanup previews first.</p></div><button type="button" class="focus-button text-error" disabled={busy || !stats || !stats.orphaned_episodes && !stats.unaired_unscheduled_episodes} onclick={handleFullCleanup}>Run full cleanup</button></div>
        {#if import.meta.env.DEV}
          <details class="border-t border-border pt-3"><summary class="text-sm text-text-muted cursor-pointer">Development tools</summary><div class="mt-3 space-y-3">
            <div><label for="dummy-version" class="block text-xs text-text-muted mb-1">Test update version</label><input id="dummy-version" class="focus-input" bind:value={dummyUpdateVersion} /></div>
            <div><label for="dummy-notes" class="block text-xs text-text-muted mb-1">Test release notes</label><textarea id="dummy-notes" class="focus-input w-full" rows="3" bind:value={dummyUpdateNotes}></textarea></div>
            <button type="button" class="focus-button" disabled={busy} onclick={() => simulateDummyUpdate(dummyUpdateVersion, dummyUpdateNotes)}>Preview update dialog</button>
            <div><label for="dev-fake-quarantine" class="block text-xs text-text-muted mb-1">Shows to leave unmatched in test</label><input id="dev-fake-quarantine" type="number" min="0" max="20" bind:value={devFakeQuarantineCount} class="focus-input" /></div>
            <button type="button" class="focus-button" disabled={busy} onclick={handleDevForceRerunMigration}>{devMigrationRunning ? "Running matching test…" : "Repeat development matching test"}</button>
          </div></details>
        {/if}
      </div>
    {/if}
  </div>
</div>
