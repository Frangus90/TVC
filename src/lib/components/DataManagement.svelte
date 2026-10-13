<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { fade, scale } from "svelte/transition";
  import { X, Database, Copy, RefreshCw, AlertTriangle, Check, CloudDownload, Download, Upload, ChevronDown, HelpCircle } from "lucide-svelte";
  import SyncTab from "./data/SyncTab.svelte";
  import ArchiveManager from "./ArchiveManager.svelte";
  import UnmigratedShowsResolver from "./UnmigratedShowsResolver.svelte";
  import {
    getUnmigratedShows,
    loadUnmigratedShows,
  } from "../stores/migration.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { save, open } from "@tauri-apps/plugin-dialog";
  import { writeTextFile, readTextFile } from "@tauri-apps/plugin-fs";
  import {
    isModalOpen,
    getActiveTab,
    isLoading,
    getError,
    getDatabaseStats,
    getDuplicates,
    getOrphanedEpisodes,
    getUnairedEpisodes,
    getCleanupPreviewError,
    isCleanupPreviewLoading,
    loadDatabaseStats,
    loadCleanupPreviews,
    closeDataManagement,
    setActiveTab,
    cleanupOrphaned,
    cleanupUnaired,
    optimizeDatabase,
    runFullCleanup,
    mergeDuplicates,
    formatBytes,
    type DuplicatePair,
  } from "../stores/dataManagement.svelte";
  import { openConfirmDialog } from "../stores/confirmDialog.svelte";
  import { simulateDummyUpdate } from "../stores/updates.svelte";
  import { parseBackup, describeBackup, type BackupData } from "../utils/backup";

  let { embedded = false }: { embedded?: boolean } = $props();

  onMount(() => {
    if (embedded) setActiveTab(getActiveTab());
  });

  let cleanupMessage = $state<string | null>(null);
  let messageKind = $state<"success" | "error" | "warning">("success");
  let messageTimer: ReturnType<typeof setTimeout> | null = null;
  let cleanupPending = $state(false);
  function showResult(message: string, kind: "success" | "error" | "warning" = "success", duration = 0) {
    if (messageTimer) clearTimeout(messageTimer);
    cleanupMessage = message;
    messageKind = kind;
    messageTimer = duration ? setTimeout(() => { cleanupMessage = null; messageTimer = null; }, duration) : null;
  }
  onDestroy(() => { if (messageTimer) clearTimeout(messageTimer); });
  let dummyUpdateVersion = $state("0.8.0");
  let dummyUpdateNotes = $state(`### Theme Settings

- You can now hide TV and movie posters in the sidebar if you prefer a cleaner look
- When you enable both compact spacing and hidden posters, the sidebar becomes a simple text list
- The "Hide Posters" option also works in the episode scheduler and show/movie picker
- Find the "Hide Posters" toggle in Theme Settings

### Better User Experience

- **Smoother Dialogs**: Confirmation dialogs now match the app's design and look much nicer
- **Faster Search**: Search now starts automatically as you type, so you don't need to press Enter. You'll also see how many results were found
- **Loading Indicators**: When the app is loading your shows and movies, you'll see helpful loading animations instead of blank screens`);
  let exporting = $state(false);
  let importing = $state(false);
  let orphanedExpanded = $state(false);
  let unairedExpanded = $state(false);
  let resolverOpen = $state(false);
  let devFakeQuarantineCount = $state(3);
  let devMigrationRunning = $state(false);
  let devMigrationError = $state<string | null>(null);

  async function handleDevForceRerunMigration() {
    devMigrationRunning = true;
    devMigrationError = null;
    try {
      await invoke<number>("dev_force_rerun_migration", {
        fakeQuarantineCount: devFakeQuarantineCount,
      });
    } catch (e) {
      devMigrationError = e instanceof Error ? e.message : String(e);
    } finally {
      devMigrationRunning = false;
    }
  }

  $effect(() => {
    if (embedded || isModalOpen()) {
      loadUnmigratedShows();
    }
  });

  interface ImportResult {
    shows_imported: number;
    episodes_imported: number;
    movies_imported: number;
    predictions_imported: number;
    quarantined: number;
    episodes_orphaned: number;
    warnings: string[];
  }

  async function handleExport() {
    exporting = true;
    try {
      // Get backup data from backend
      const data = await invoke<BackupData>("export_database");

      // Ask user where to save
      const filePath = await save({
        defaultPath: `tvc-backup-${new Date().toISOString().split("T")[0]}.json`,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });

      if (filePath) {
        await writeTextFile(filePath, JSON.stringify(data, null, 2));
        showResult(`Exported ${data.shows.length} shows, ${data.episodes.length} episodes, ${data.movies.length} movies, and ${data.personal_data?.predictions.length ?? 0} predictions, including tiers and history.`, "success", 5000);
      }
    } catch (err) {
      showResult(`Export failed: ${err}`, "error");
    } finally {
      exporting = false;
    }
  }

  async function handleImport() {
    importing = true;
    try {
      const filePath = await open({
        filters: [{ name: "JSON", extensions: ["json"] }],
        multiple: false,
      });
      if (!filePath || typeof filePath !== "string") return;
      const data = parseBackup(await readTextFile(filePath));
      const confirmed = await openConfirmDialog({
        title: "Import Backup",
        message: describeBackup(data),
        type: "danger",
        confirmLabel: "Replace from backup",
        cancelLabel: "Cancel",
      });
      if (!confirmed) return;

      // Import via backend
      const result = await invoke<ImportResult>("import_database", { data });

      let message = `Imported ${result.shows_imported} shows, ${result.episodes_imported} episodes, ${result.movies_imported} movies, and ${result.predictions_imported} predictions. Please restart the app.`;
      if (result.quarantined || result.episodes_orphaned || result.warnings.length) {
        message += ` ${result.quarantined} shows need matching; ${result.episodes_orphaned} legacy episodes could not be matched. ${result.warnings.join(" ")}`;
      }
      showResult(message, result.quarantined || result.episodes_orphaned || result.warnings.length ? "warning" : "success");
    } catch (err) {
      showResult(`Import failed: ${err}`, "error");
    } finally {
      importing = false;
    }
  }

  async function confirmCleanup(kind: "orphaned" | "undated" | "all") {
    await Promise.all([loadDatabaseStats(), loadCleanupPreviews()]);
    if (getError() || getCleanupPreviewError()) return false;
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
    const keepId = keepFirst ? pair.show1_id : pair.show2_id;
    const mergeId = keepFirst ? pair.show2_id : pair.show1_id;
    const keepName = keepFirst ? pair.show1_name : pair.show2_name;

    const confirmed = await openConfirmDialog({
      title: "Merge Shows",
      message: `Merge into "${keepName}"? The other show will be deleted.`,
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
  }
</script>

{#if embedded || isModalOpen()}
  {@const activeTab = getActiveTab()}
  {@const loading = isLoading()}
  {@const error = getError()}
  {@const stats = getDatabaseStats()}
  {@const duplicates = getDuplicates()}
  {@const orphanedEpisodesList = getOrphanedEpisodes()}
  {@const unairedEpisodesList = getUnairedEpisodes()}
  {@const unmappedCount = getUnmigratedShows().length}

  {#if !embedded}
    <!-- Backdrop -->
    <button
      type="button"
      transition:fade={{ duration: 150 }}
      class="fixed inset-0 bg-black/60 z-50"
      onclick={closeDataManagement}
      aria-label="Close modal"
    ></button>
  {/if}

  <!-- Modal or full page -->
  <div
    transition:scale={{ duration: embedded ? 0 : 200, start: 0.95, opacity: 0 }}
    class={embedded ? "focus-page focus-data-page" : "fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 bg-surface rounded-xl border border-border shadow-2xl w-[800px] max-w-[95vw] max-h-[85vh] flex flex-col"}
  >
    {#snippet sectionTabs()}
      <!-- Tabs -->
      <nav class={embedded ? "focus-tabs" : "flex border-b border-border overflow-x-auto"} aria-label="Data management sections">
        <button
          type="button"
          onclick={() => setActiveTab("overview")}
          aria-pressed={activeTab === "overview"}
          class="px-4 py-3 text-sm font-medium transition-colors {activeTab === 'overview'
            ? 'text-accent border-b-2 border-accent'
            : 'text-text-muted hover:text-text'}"
        >
          Overview
        </button>
        <button
          type="button"
          onclick={() => setActiveTab("sync")}
          aria-pressed={activeTab === "sync"}
          class="px-4 py-3 text-sm font-medium transition-colors {activeTab === 'sync'
            ? 'text-accent border-b-2 border-accent'
            : 'text-text-muted hover:text-text'}"
        >
          Sync
        </button>
        <button
          type="button"
          onclick={() => setActiveTab("duplicates")}
          aria-pressed={activeTab === "duplicates"}
          class="px-4 py-3 text-sm font-medium transition-colors {activeTab === 'duplicates'
            ? 'text-accent border-b-2 border-accent'
            : 'text-text-muted hover:text-text'}"
        >
          Duplicates
        </button>
        <button
          type="button"
          onclick={() => setActiveTab("cleanup")}
          aria-pressed={activeTab === "cleanup"}
          class="px-4 py-3 text-sm font-medium transition-colors {activeTab === 'cleanup'
            ? 'text-accent border-b-2 border-accent'
            : 'text-text-muted hover:text-text'}"
        >
          Cleanup
        </button>
        <button
          type="button"
          onclick={() => setActiveTab("archive")}
          aria-pressed={activeTab === "archive"}
          class="px-4 py-3 text-sm font-medium transition-colors {activeTab === 'archive'
            ? 'text-accent border-b-2 border-accent'
            : 'text-text-muted hover:text-text'}"
        >
          Archive
        </button>
      </nav>
    {/snippet}

    {#if embedded}
      <header class="focus-page-heading focus-page-bar">
        <h1>Data management</h1>
        {@render sectionTabs()}
      </header>
    {:else}
      <div class="flex items-center justify-between p-4 border-b border-border">
        <div class="flex items-center gap-3">
          <Database class="w-5 h-5 text-accent" />
          <h2 class="text-lg font-semibold">Data Management</h2>
        </div>
        <button type="button" onclick={closeDataManagement} class="p-2 rounded-lg hover:bg-surface-hover transition-colors" aria-label="Close">
          <X class="w-5 h-5 text-text-muted" />
        </button>
      </div>
    {/if}

    {#if !embedded}
      {@render sectionTabs()}
    {/if}

    <!-- Content -->
    <div class={embedded ? "focus-data-content" : "flex-1 overflow-auto p-4"}>
      {#if activeTab === "sync"}
        <SyncTab />
      {:else if activeTab === "archive"}
        <ArchiveManager />
      {:else if loading}
        <div class="flex items-center justify-center py-12">
          <RefreshCw class="w-6 h-6 text-accent animate-spin" />
        </div>
      {:else if error}
        <div class="text-center py-8">
          <AlertTriangle class="w-8 h-8 text-red-400 mx-auto mb-2" />
          <p class="text-red-400">{error}</p>
          <button type="button" onclick={() => setActiveTab(activeTab)} class="focus-button mt-3">Reload page</button>
        </div>
      {:else if activeTab === "overview"}
        <!-- Overview Tab -->
        {#if stats}
          <div class="grid grid-cols-2 md:grid-cols-3 gap-4">
            <div class="focus-panel bg-surface rounded-xl border border-border p-5">
              <p class="text-2xl font-bold text-text">{stats.total_shows}</p>
              <p class="text-sm text-text-muted">Shows</p>
            </div>
            <div class="focus-panel bg-surface rounded-xl border border-border p-5">
              <p class="text-2xl font-bold text-text">{stats.total_episodes}</p>
              <p class="text-sm text-text-muted">Episodes</p>
            </div>
            <div class="focus-panel bg-surface rounded-xl border border-border p-5">
              <p class="text-2xl font-bold text-text">{stats.total_movies}</p>
              <p class="text-sm text-text-muted">Movies</p>
            </div>
            <div class="focus-panel bg-surface rounded-xl border border-border p-5">
              <p class="text-2xl font-bold text-text">{formatBytes(stats.database_size_bytes)}</p>
              <p class="text-sm text-text-muted">Database Size</p>
            </div>
            <div class="focus-panel bg-surface rounded-xl border border-border p-5">
              <p class="text-2xl font-bold {stats.orphaned_episodes > 0 ? 'text-yellow-400' : 'text-text'}">
                {stats.orphaned_episodes}
              </p>
              <p class="text-sm text-text-muted">Orphaned Episodes</p>
            </div>
          </div>

          {#if unmappedCount > 0}
            <div class="mt-6 p-4 bg-accent/10 border border-accent/30 rounded-lg">
              <div class="flex items-start gap-3">
                <HelpCircle class="w-5 h-5 text-accent flex-shrink-0 mt-0.5" />
                <div class="flex-1">
                  <p class="text-sm text-text">
                    {unmappedCount} show{unmappedCount !== 1 ? "s" : ""} couldn't be matched to TMDB
                    automatically and need manual resolution.
                  </p>
                  <button
                    type="button"
                    onclick={() => (resolverOpen = true)}
                    class="text-sm text-accent hover:underline mt-1"
                  >
                    Resolve unmapped shows
                  </button>
                </div>
              </div>
            </div>
          {/if}

          {#if stats.orphaned_episodes > 0 || stats.unaired_unscheduled_episodes > 0}
            <div class="mt-6 p-4 bg-yellow-500/10 border border-yellow-500/30 rounded-lg">
              <div class="flex items-start gap-3">
                <AlertTriangle class="w-5 h-5 text-yellow-400 flex-shrink-0 mt-0.5" />
                <div>
                  <p class="text-sm text-yellow-200">
                    {#if stats.orphaned_episodes > 0}
                      {stats.orphaned_episodes} orphaned episode{stats.orphaned_episodes !== 1 ? "s" : ""} found.
                    {/if}
                    {#if stats.unaired_unscheduled_episodes > 0}
                      {stats.unaired_unscheduled_episodes} unaired/unscheduled episode{stats.unaired_unscheduled_episodes !== 1 ? "s" : ""} found.
                    {/if}
                  </p>
                  <button
                    type="button"
                    onclick={() => setActiveTab("cleanup")}
                    class="text-sm text-yellow-400 hover:text-yellow-300 underline mt-1"
                  >
                    Go to Cleanup
                  </button>
                </div>
              </div>
            </div>
          {/if}

          <!-- Export / Import Section -->
          <div class="focus-panel mt-6 p-5 bg-surface rounded-xl border border-border">
            <h3 class="text-sm font-medium text-text mb-3">Backup & Restore</h3>
            <p class="text-xs text-text-muted mb-4">
              Back up your library, watch dates, schedules, ratings, notes, tiers, award predictions,
              Plex title corrections, scrobble logs, and change history to JSON.
              Settings, integration credentials, and racing data are excluded.
            </p>
            <div class="flex gap-3">
              <button
                type="button"
                onclick={handleExport}
                disabled={exporting || importing}
                class="flex-1 px-3 py-2 text-sm bg-accent/20 hover:bg-accent/30 text-accent rounded-lg transition-colors flex items-center justify-center gap-2 disabled:opacity-50"
              >
                {#if exporting}
                  <RefreshCw class="w-4 h-4 animate-spin" />
                  Exporting...
                {:else}
                  <Download class="w-4 h-4" />
                  Export Data
                {/if}
              </button>
              <button
                type="button"
                onclick={handleImport}
                disabled={importing || exporting}
                class="flex-1 px-3 py-2 text-sm bg-surface-hover hover:bg-surface-hover/80 text-text rounded-lg transition-colors flex items-center justify-center gap-2 disabled:opacity-50"
              >
                {#if importing}
                  <RefreshCw class="w-4 h-4 animate-spin" />
                  Importing...
                {:else}
                  <Upload class="w-4 h-4" />
                  Import Data
                {/if}
              </button>
            </div>
          </div>

          <!-- Dev-only: Dummy Update Testing -->
          <!-- TEMP: Set to false to test production mode (hiding feature) -->
          {@const showDummyUpdate = import.meta.env.DEV && true}
          {#if showDummyUpdate}
            <div class="focus-panel mt-6 p-5 bg-surface rounded-xl border border-border border-dashed">
              <h3 class="text-sm font-medium text-text mb-3">Development: Test Update Modal</h3>
              <p class="text-xs text-text-muted mb-4">
                Simulate an update notification to test the update modal UI without requiring a real GitHub release.
              </p>
              <div class="space-y-3">
                <div>
                  <label for="dummy-version" class="block text-xs font-medium text-text-muted mb-1">Version</label>
                  <input
                    id="dummy-version"
                    type="text"
                    bind:value={dummyUpdateVersion}
                    placeholder="0.8.0"
                    class="w-full px-3 py-2 text-sm rounded border border-border bg-surface text-text placeholder:text-text-muted outline-none focus:ring-2 focus:ring-accent"
                  />
                </div>
                <div>
                  <label for="dummy-notes" class="block text-xs font-medium text-text-muted mb-1">Release Notes</label>
                  <textarea
                    id="dummy-notes"
                    bind:value={dummyUpdateNotes}
                    placeholder="Enter release notes here..."
                    rows="8"
                    class="w-full px-3 py-2 text-sm rounded border border-border bg-surface text-text placeholder:text-text-muted outline-none focus:ring-2 focus:ring-accent resize-y font-mono"
                  ></textarea>
                </div>
                <button
                  type="button"
                  onclick={() => simulateDummyUpdate(dummyUpdateVersion, dummyUpdateNotes)}
                  class="w-full px-3 py-2 text-sm bg-accent/20 hover:bg-accent/30 text-accent rounded-lg transition-colors flex items-center justify-center gap-2"
                >
                  <CloudDownload class="w-4 h-4" />
                  Simulate Update
                </button>
              </div>
            </div>

            <div class="focus-panel mt-6 p-5 bg-surface rounded-xl border border-border border-dashed">
              <h3 class="text-sm font-medium text-text mb-3">Development: Force TVDB&rarr;TMDB Migration</h3>
              <p class="text-xs text-text-muted mb-4">
                Resets every migrated show back to <code>unmigrated=1</code> and seeds N
                synthetic shows with negative legacy ids (so TMDB <code>/find</code> returns
                no match and they land in the quarantine bucket). Re-spawns the migration
                runner so the progress modal and resolver UI can be exercised on demand.
              </p>
              <div class="space-y-3">
                <div>
                  <label for="dev-fake-quarantine" class="block text-xs font-medium text-text-muted mb-1">
                    Fake quarantine rows
                  </label>
                  <input
                    id="dev-fake-quarantine"
                    type="number"
                    min="0"
                    max="50"
                    bind:value={devFakeQuarantineCount}
                    class="w-full px-3 py-2 text-sm rounded border border-border bg-surface text-text outline-none focus:ring-2 focus:ring-accent"
                  />
                </div>
                {#if devMigrationError}
                  <p class="text-xs text-red-400">{devMigrationError}</p>
                {/if}
                <button
                  type="button"
                  disabled={devMigrationRunning}
                  onclick={handleDevForceRerunMigration}
                  class="w-full px-3 py-2 text-sm bg-accent/20 hover:bg-accent/30 text-accent rounded-lg transition-colors flex items-center justify-center gap-2 disabled:opacity-50"
                >
                  {#if devMigrationRunning}
                    <RefreshCw class="w-4 h-4 animate-spin" />
                    Running...
                  {:else}
                    <RefreshCw class="w-4 h-4" />
                    Force Re-run Migration
                  {/if}
                </button>
              </div>
            </div>
          {/if}
        {/if}
      {:else if activeTab === "duplicates"}
        <!-- Duplicates Tab -->
        {#if duplicates.length === 0}
          <div class="text-center py-12 text-text-muted">
            <Check class="w-12 h-12 mx-auto mb-3 text-available opacity-50" />
            <p>No duplicates found</p>
            <p class="text-sm mt-1">Your library is clean!</p>
          </div>
        {:else}
          <div class="space-y-4">
            <p class="text-sm text-text-muted">
              Found {duplicates.length} potential duplicate{duplicates.length !== 1 ? "s" : ""}
            </p>
            {#each duplicates as pair}
              <div class="focus-panel bg-surface rounded-xl border border-border p-5">
                <div class="flex items-center gap-2 mb-3">
                  <Copy class="w-4 h-4 text-yellow-400" />
                  <span class="text-sm text-yellow-400">{pair.similarity_reason}</span>
                </div>
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
                  <!-- Show 1 -->
                  <div class="border border-border rounded-lg p-3">
                    <div class="flex gap-3">
                      {#if pair.show1_poster_url}
                        <img src={pair.show1_poster_url} alt="" class="w-12 h-16 rounded object-cover" />
                      {:else}
                        <div class="w-12 h-16 rounded bg-surface-hover"></div>
                      {/if}
                      <div class="flex-1">
                        <p class="font-medium text-sm truncate">{pair.show1_name}</p>
                        <p class="text-xs text-text-muted">
                          {pair.show1_episode_count} episodes, {pair.show1_watched_count} watched
                        </p>
                      </div>
                    </div>
                    <button
                      type="button"
                      onclick={() => handleMerge(pair, true)}
                      class="w-full mt-3 px-3 py-1.5 text-xs bg-accent/20 hover:bg-accent/30 text-accent rounded transition-colors"
                    >
                      Keep This One
                    </button>
                  </div>
                  <!-- Show 2 -->
                  <div class="border border-border rounded-lg p-3">
                    <div class="flex gap-3">
                      {#if pair.show2_poster_url}
                        <img src={pair.show2_poster_url} alt="" class="w-12 h-16 rounded object-cover" />
                      {:else}
                        <div class="w-12 h-16 rounded bg-surface-hover"></div>
                      {/if}
                      <div class="flex-1">
                        <p class="font-medium text-sm truncate">{pair.show2_name}</p>
                        <p class="text-xs text-text-muted">
                          {pair.show2_episode_count} episodes, {pair.show2_watched_count} watched
                        </p>
                      </div>
                    </div>
                    <button
                      type="button"
                      onclick={() => handleMerge(pair, false)}
                      class="w-full mt-3 px-3 py-1.5 text-xs bg-accent/20 hover:bg-accent/30 text-accent rounded transition-colors"
                    >
                      Keep This One
                    </button>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      {:else if activeTab === "cleanup"}
        <!-- Cleanup Tab -->
        {#if stats}
          <div class="space-y-4">
            {#if getCleanupPreviewError()}
              <div role="alert" class="p-3 rounded-lg border border-red-500/30 bg-red-500/10 text-red-400 text-sm">
                <p>{getCleanupPreviewError()}</p>
                <button type="button" onclick={() => loadCleanupPreviews()} disabled={isCleanupPreviewLoading()} class="underline mt-2">Retry preview</button>
              </div>
            {:else if isCleanupPreviewLoading()}
              <p role="status" class="text-sm text-text-muted">Loading affected episode records…</p>
            {/if}
            <!-- Orphaned Episodes -->
            <div class="focus-panel bg-surface rounded-xl border border-border p-5">
              <div class="flex flex-wrap items-center justify-between gap-4">
                <div>
                  <h3 class="font-medium">Orphaned Episodes</h3>
                  <p class="text-sm text-text-muted">Unused records whose show has been deleted; personal data is protected</p>
                </div>
                <div class="flex items-center gap-3">
                  <span class="text-lg font-bold {stats.orphaned_episodes > 0 ? 'text-yellow-400' : 'text-text-muted'}">
                    {stats.orphaned_episodes}
                  </span>
                  <button
                    type="button"
                    onclick={handleCleanupOrphaned}
                    disabled={stats.orphaned_episodes === 0 || cleanupPending || isCleanupPreviewLoading() || !!getCleanupPreviewError()}
                    class="px-3 py-1.5 text-sm bg-red-500/20 hover:bg-red-500/30 text-red-400 rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    Clean
                  </button>
                </div>
              </div>
              {#if orphanedEpisodesList.length > 0}
                <button
                  type="button"
                  onclick={() => orphanedExpanded = !orphanedExpanded}
                  class="mt-3 flex items-center gap-2 text-sm text-text-muted hover:text-text transition-colors"
                >
                  <ChevronDown class="w-4 h-4 transition-transform {orphanedExpanded ? 'rotate-180' : ''}" />
                  {orphanedExpanded ? 'Hide' : 'Show'} affected episodes
                </button>
                {#if orphanedExpanded}
                  <div class="mt-2 p-3 bg-surface rounded-lg border border-border max-h-48 overflow-y-auto">
                    <p class="text-xs text-text-muted mb-2">Showing {Math.min(orphanedEpisodesList.length, 20)} of {stats.orphaned_episodes} eligible records</p>
                    <ul class="space-y-1 text-sm">
                      {#each orphanedEpisodesList.slice(0, 20) as ep}
                        <li class="text-text-muted">
                          <span class="text-text">{ep.show_name}</span>
                          <span class="mx-1">-</span>
                          <span>S{String(ep.season_number).padStart(2, '0')}E{String(ep.episode_number).padStart(2, '0')}</span>
                          {#if ep.name}
                            <span class="mx-1">-</span>
                            <span class="text-text">{ep.name}</span>
                          {/if}
                        </li>
                      {/each}
                      {#if stats.orphaned_episodes > 20}
                        <li class="text-text-muted italic">...and {stats.orphaned_episodes - 20} more</li>
                      {/if}
                    </ul>
                  </div>
                {/if}
              {/if}
            </div>

            <!-- Unaired Episodes -->
            <div class="focus-panel bg-surface rounded-xl border border-border p-5">
              <div class="flex flex-wrap items-center justify-between gap-4">
                <div>
                  <h3 class="font-medium">Unused Episodes Without Dates</h3>
                  <p class="text-sm text-text-muted">No air date or personal data. Upcoming episodes can legitimately have no date.</p>
                </div>
                <div class="flex items-center gap-3">
                  <span class="text-lg font-bold {stats.unaired_unscheduled_episodes > 0 ? 'text-yellow-400' : 'text-text-muted'}">
                    {stats.unaired_unscheduled_episodes}
                  </span>
                  <button
                    type="button"
                    onclick={handleCleanupUnaired}
                    disabled={stats.unaired_unscheduled_episodes === 0 || cleanupPending || isCleanupPreviewLoading() || !!getCleanupPreviewError()}
                    class="px-3 py-1.5 text-sm bg-red-500/20 hover:bg-red-500/30 text-red-400 rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    Clean
                  </button>
                </div>
              </div>
              {#if unairedEpisodesList.length > 0}
                <button
                  type="button"
                  onclick={() => unairedExpanded = !unairedExpanded}
                  class="mt-3 flex items-center gap-2 text-sm text-text-muted hover:text-text transition-colors"
                >
                  <ChevronDown class="w-4 h-4 transition-transform {unairedExpanded ? 'rotate-180' : ''}" />
                  {unairedExpanded ? 'Hide' : 'Show'} affected episodes
                </button>
                {#if unairedExpanded}
                  <div class="mt-2 p-3 bg-surface rounded-lg border border-border max-h-48 overflow-y-auto">
                    <p class="text-xs text-text-muted mb-2">Showing {Math.min(unairedEpisodesList.length, 20)} of {stats.unaired_unscheduled_episodes} eligible records</p>
                    <ul class="space-y-1 text-sm">
                      {#each unairedEpisodesList.slice(0, 20) as ep}
                        <li class="text-text-muted">
                          <span class="text-text">{ep.show_name}</span>
                          <span class="mx-1">-</span>
                          <span>S{String(ep.season_number).padStart(2, '0')}E{String(ep.episode_number).padStart(2, '0')}</span>
                          {#if ep.name}
                            <span class="mx-1">-</span>
                            <span class="text-text">{ep.name}</span>
                          {/if}
                        </li>
                      {/each}
                      {#if stats.unaired_unscheduled_episodes > 20}
                        <li class="text-text-muted italic">...and {stats.unaired_unscheduled_episodes - 20} more</li>
                      {/if}
                    </ul>
                  </div>
                {/if}
              {/if}
            </div>

            <!-- Optimize -->
            <div class="focus-panel bg-surface rounded-xl border border-border p-5">
              <div class="flex flex-wrap items-center justify-between gap-4">
                <div>
                  <h3 class="font-medium">Optimize Database</h3>
                  <p class="text-sm text-text-muted">Compact stored data and keep the app running smoothly</p>
                </div>
                <button
                  type="button"
                  onclick={handleOptimize}
                  disabled={cleanupPending}
                  class="px-3 py-1.5 text-sm bg-accent/20 hover:bg-accent/30 text-accent rounded transition-colors"
                >
                  Optimize
                </button>
              </div>
            </div>

            <!-- Full Cleanup -->
            <div class="focus-panel bg-surface rounded-xl p-5 border border-red-500/30">
              <div class="flex flex-wrap items-center justify-between gap-4">
                <div>
                  <h3 class="font-medium text-red-400">Full Cleanup</h3>
                  <p class="text-sm text-text-muted">Remove eligible orphaned and undated episode records, then optimize the database</p>
                </div>
                <button
                  type="button"
                  onclick={handleFullCleanup}
                  disabled={cleanupPending || isCleanupPreviewLoading() || !!getCleanupPreviewError() || stats.orphaned_episodes + stats.unaired_unscheduled_episodes === 0}
                  class="px-3 py-1.5 text-sm bg-red-500/20 hover:bg-red-500/30 text-red-400 rounded transition-colors"
                >
                  Run Full Cleanup
                </button>
              </div>
            </div>
          </div>
        {/if}
      {/if}
    </div>

    <!-- Footer message -->
    {#if cleanupMessage}
      <div role={messageKind === "error" ? "alert" : "status"} class="mt-4 p-3 rounded-lg border text-sm flex items-start justify-between gap-3 {messageKind === 'error' ? 'border-red-500/30 bg-red-500/10 text-red-400' : messageKind === 'warning' ? 'border-yellow-500/30 bg-yellow-500/10 text-yellow-400' : 'border-available/30 bg-available/10 text-available'}">
        <span>{cleanupMessage}</span>
        <button type="button" onclick={() => { if (messageTimer) clearTimeout(messageTimer); messageTimer = null; cleanupMessage = null; }} aria-label="Dismiss result" class="shrink-0 p-1"><X size={16} /></button>
      </div>
    {/if}
  </div>

  <UnmigratedShowsResolver open={resolverOpen} onClose={() => (resolverOpen = false)} />
{/if}

<style>
  .focus-page-bar .focus-tabs { margin-bottom: 0; }
  .focus-data-page { max-width: 1280px; }
  .focus-data-content { min-width: 0; }
</style>
