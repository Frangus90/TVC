<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import { X, Database, History, Copy, Trash2, RefreshCw, AlertTriangle, Check, CloudDownload, Download, Upload, ChevronDown } from "lucide-svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { save, open } from "@tauri-apps/plugin-dialog";
  import { writeTextFile, readTextFile } from "@tauri-apps/plugin-fs";
  import {
    isModalOpen,
    getActiveTab,
    isLoading,
    getError,
    getDatabaseStats,
    getChangeHistory,
    getHistoryStats,
    getDuplicates,
    getOrphanedEpisodes,
    getUnairedEpisodes,
    closeDataManagement,
    setActiveTab,
    cleanupOrphaned,
    cleanupUnaired,
    optimizeDatabase,
    runFullCleanup,
    clearHistory,
    mergeDuplicates,
    formatBytes,
    formatChangeType,
    formatRelativeDate,
    type DuplicatePair,
  } from "../stores/dataManagement.svelte";
  import { openConfirmDialog } from "../stores/confirmDialog.svelte";
  import { simulateDummyUpdate } from "../stores/updates.svelte";
  import type { TrackedShow, Episode } from "../stores/shows.svelte";
  import type { TrackedMovie } from "../stores/movies.svelte";

  let cleanupMessage = $state<string | null>(null);
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
  let syncingAll = $state(false);
  let exporting = $state(false);
  let importing = $state(false);
  let orphanedExpanded = $state(false);
  let unairedExpanded = $state(false);

  interface BackupData {
    version: string;
    exported_at: string;
    shows: TrackedShow[];
    episodes: Episode[];
    movies: TrackedMovie[];
  }

  interface ImportResult {
    shows_imported: number;
    episodes_imported: number;
    movies_imported: number;
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
        cleanupMessage = `Exported ${data.shows.length} shows, ${data.episodes.length} episodes, ${data.movies.length} movies`;
        setTimeout(() => (cleanupMessage = null), 5000);
      }
    } catch (err) {
      cleanupMessage = `Export failed: ${err}`;
      setTimeout(() => (cleanupMessage = null), 5000);
    } finally {
      exporting = false;
    }
  }

  async function handleImport() {
    // Ask user to select file
    const filePath = await open({
      filters: [{ name: "JSON", extensions: ["json"] }],
      multiple: false,
    });

    if (!filePath || typeof filePath !== "string") return;

    // Confirm before proceeding
    const confirmed = await openConfirmDialog({
      title: "Import Backup",
      message: "This will REPLACE all your current data with the backup. Are you sure?",
      type: "danger",
      confirmLabel: "Import",
      cancelLabel: "Cancel",
    });

    if (!confirmed) {
      return;
    }

    importing = true;
    try {
      // Read and parse the file
      const content = await readTextFile(filePath);
      const data = JSON.parse(content) as BackupData;

      // Validate structure
      if (!data.version || !data.shows || !data.episodes || !data.movies) {
        throw new Error("Invalid backup file format");
      }

      // Import via backend
      const result = await invoke<ImportResult>("import_database", { data });

      cleanupMessage = `Imported ${result.shows_imported} shows, ${result.episodes_imported} episodes, ${result.movies_imported} movies. Please restart the app.`;
      setTimeout(() => (cleanupMessage = null), 10000);
    } catch (err) {
      cleanupMessage = `Import failed: ${err}`;
      setTimeout(() => (cleanupMessage = null), 5000);
    } finally {
      importing = false;
    }
  }

  async function handleSyncAll() {
    syncingAll = true;
    try {
      const [showsSynced, moviesSynced] = await Promise.all([
        invoke<number>("sync_all_shows"),
        invoke<number>("sync_all_movies"),
      ]);
      
      const parts: string[] = [];
      if (showsSynced > 0) {
        parts.push(`${showsSynced} show${showsSynced !== 1 ? "s" : ""} from TVDB`);
      }
      if (moviesSynced > 0) {
        parts.push(`${moviesSynced} movie${moviesSynced !== 1 ? "s" : ""} from TMDB`);
      }
      
      if (parts.length > 0) {
        cleanupMessage = `Synced ${parts.join(" and ")}`;
      } else {
        cleanupMessage = "No shows or movies to sync";
      }
      setTimeout(() => (cleanupMessage = null), 3000);
    } catch (err) {
      cleanupMessage = `Sync failed: ${err}`;
      setTimeout(() => (cleanupMessage = null), 5000);
    } finally {
      syncingAll = false;
    }
  }

  async function handleCleanupOrphaned() {
    try {
      const count = await cleanupOrphaned();
      cleanupMessage = `Removed ${count} orphaned episode${count !== 1 ? "s" : ""}`;
      setTimeout(() => (cleanupMessage = null), 3000);
    } catch {
      // Error handled in store
    }
  }

  async function handleCleanupUnaired() {
    try {
      const count = await cleanupUnaired();
      cleanupMessage = `Removed ${count} unaired episode${count !== 1 ? "s" : ""}`;
      setTimeout(() => (cleanupMessage = null), 3000);
    } catch {
      // Error handled in store
    }
  }

  async function handleOptimize() {
    try {
      await optimizeDatabase();
      cleanupMessage = "Database optimized successfully";
      setTimeout(() => (cleanupMessage = null), 3000);
    } catch {
      // Error handled in store
    }
  }

  async function handleFullCleanup() {
    try {
      const result = await runFullCleanup();
      cleanupMessage = `Cleanup complete: ${result.orphaned_episodes_removed} orphaned, ${result.unaired_episodes_removed} unaired, ${result.history_entries_removed} history entries removed`;
      setTimeout(() => (cleanupMessage = null), 5000);
    } catch {
      // Error handled in store
    }
  }

  async function handleClearHistory() {
    const confirmed = await openConfirmDialog({
      title: "Clear History",
      message: "Are you sure you want to clear all change history?",
      type: "warning",
      confirmLabel: "Clear",
      cancelLabel: "Cancel",
    });

    if (confirmed) {
      try {
        const count = await clearHistory();
        cleanupMessage = `Cleared ${count} history entries`;
        setTimeout(() => (cleanupMessage = null), 3000);
      } catch {
        // Error handled in store
      }
    }
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
        cleanupMessage = `Merged: ${result.episodes_moved} moved, ${result.episodes_merged} merged`;
        setTimeout(() => (cleanupMessage = null), 3000);
      } catch {
        // Error handled in store
      }
    }
  }
</script>

{#if isModalOpen()}
  {@const activeTab = getActiveTab()}
  {@const loading = isLoading()}
  {@const error = getError()}
  {@const stats = getDatabaseStats()}
  {@const history = getChangeHistory()}
  {@const historyStats = getHistoryStats()}
  {@const duplicates = getDuplicates()}
  {@const orphanedEpisodesList = getOrphanedEpisodes()}
  {@const unairedEpisodesList = getUnairedEpisodes()}

  <!-- Backdrop -->
  <button
    type="button"
    transition:fade={{ duration: 150 }}
    class="fixed inset-0 bg-black/60 z-50"
    onclick={closeDataManagement}
    aria-label="Close modal"
  ></button>

  <!-- Modal -->
  <div
    transition:scale={{ duration: 200, start: 0.95, opacity: 0 }}
    class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 bg-surface rounded-xl border border-border shadow-2xl w-[800px] max-w-[95vw] max-h-[85vh] flex flex-col"
  >
    <!-- Header -->
    <div class="flex items-center justify-between p-4 border-b border-border">
      <div class="flex items-center gap-3">
        <Database class="w-5 h-5 text-accent" />
        <h2 class="text-lg font-semibold">Data Management</h2>
      </div>
      <button
        type="button"
        onclick={closeDataManagement}
        class="p-2 rounded-lg hover:bg-surface-hover transition-colors"
        aria-label="Close"
      >
        <X class="w-5 h-5 text-text-muted" />
      </button>
    </div>

    <!-- Tabs -->
    <div class="flex border-b border-border">
      <button
        type="button"
        onclick={() => setActiveTab("overview")}
        class="px-4 py-3 text-sm font-medium transition-colors {activeTab === 'overview'
          ? 'text-accent border-b-2 border-accent'
          : 'text-text-muted hover:text-text'}"
      >
        Overview
      </button>
      <button
        type="button"
        onclick={() => setActiveTab("history")}
        class="px-4 py-3 text-sm font-medium transition-colors {activeTab === 'history'
          ? 'text-accent border-b-2 border-accent'
          : 'text-text-muted hover:text-text'}"
      >
        History
      </button>
      <button
        type="button"
        onclick={() => setActiveTab("duplicates")}
        class="px-4 py-3 text-sm font-medium transition-colors {activeTab === 'duplicates'
          ? 'text-accent border-b-2 border-accent'
          : 'text-text-muted hover:text-text'}"
      >
        Duplicates
      </button>
      <button
        type="button"
        onclick={() => setActiveTab("cleanup")}
        class="px-4 py-3 text-sm font-medium transition-colors {activeTab === 'cleanup'
          ? 'text-accent border-b-2 border-accent'
          : 'text-text-muted hover:text-text'}"
      >
        Cleanup
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-auto p-4">
      {#if loading}
        <div class="flex items-center justify-center py-12">
          <RefreshCw class="w-6 h-6 text-accent animate-spin" />
        </div>
      {:else if error}
        <div class="text-center py-8">
          <AlertTriangle class="w-8 h-8 text-red-400 mx-auto mb-2" />
          <p class="text-red-400">{error}</p>
        </div>
      {:else if activeTab === "overview"}
        <!-- Overview Tab -->
        {#if stats}
          <div class="grid grid-cols-2 md:grid-cols-3 gap-4">
            <div class="bg-background rounded-lg p-4">
              <p class="text-2xl font-bold text-text">{stats.total_shows}</p>
              <p class="text-sm text-text-muted">Shows</p>
            </div>
            <div class="bg-background rounded-lg p-4">
              <p class="text-2xl font-bold text-text">{stats.total_episodes}</p>
              <p class="text-sm text-text-muted">Episodes</p>
            </div>
            <div class="bg-background rounded-lg p-4">
              <p class="text-2xl font-bold text-text">{stats.total_movies}</p>
              <p class="text-sm text-text-muted">Movies</p>
            </div>
            <div class="bg-background rounded-lg p-4">
              <p class="text-2xl font-bold text-text">{formatBytes(stats.database_size_bytes)}</p>
              <p class="text-sm text-text-muted">Database Size</p>
            </div>
            <div class="bg-background rounded-lg p-4">
              <p class="text-2xl font-bold text-text">{stats.change_history_count}</p>
              <p class="text-sm text-text-muted">History Entries</p>
            </div>
            <div class="bg-background rounded-lg p-4">
              <p class="text-2xl font-bold {stats.orphaned_episodes > 0 ? 'text-yellow-400' : 'text-text'}">
                {stats.orphaned_episodes}
              </p>
              <p class="text-sm text-text-muted">Orphaned Episodes</p>
            </div>
          </div>

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
          <div class="mt-6 p-4 bg-background rounded-lg border border-border">
            <h3 class="text-sm font-medium text-text mb-3">Backup & Restore</h3>
            <p class="text-xs text-text-muted mb-4">
              Export your data to a JSON file for backup, or import from a previous backup.
            </p>
            <div class="flex gap-3">
              <button
                type="button"
                onclick={handleExport}
                disabled={exporting}
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
                disabled={importing}
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
            <div class="mt-6 p-4 bg-background rounded-lg border border-border border-dashed">
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
          {/if}
        {/if}
      {:else if activeTab === "history"}
        <!-- History Tab -->
        <div class="space-y-4">
          {#if historyStats}
            <div class="flex items-center justify-between">
              <div class="flex gap-4 text-sm text-text-muted">
                <span>{historyStats.total_changes} total</span>
                <span>{historyStats.watched_changes} watched</span>
                <span>{historyStats.schedule_changes} scheduled</span>
                <span>{historyStats.rating_changes} ratings</span>
              </div>
              {#if historyStats.total_changes > 0}
                <button
                  type="button"
                  onclick={handleClearHistory}
                  class="text-sm text-red-400 hover:text-red-300 flex items-center gap-1"
                >
                  <Trash2 class="w-3 h-3" />
                  Clear All
                </button>
              {/if}
            </div>
          {/if}

          {#if history.length === 0}
            <div class="text-center py-12 text-text-muted">
              <History class="w-12 h-12 mx-auto mb-3 opacity-50" />
              <p>No change history yet</p>
              <p class="text-sm mt-1">Changes will be tracked as you use the app</p>
            </div>
          {:else}
            <div class="space-y-2">
              {#each history as item}
                <div class="flex items-center gap-3 p-3 bg-background rounded-lg">
                  {#if item.poster_url}
                    <img src={item.poster_url} alt="" class="w-10 h-14 rounded object-cover" />
                  {:else}
                    <div class="w-10 h-14 rounded bg-surface-hover"></div>
                  {/if}
                  <div class="flex-1 min-w-0">
                    <p class="text-sm text-text truncate">
                      {item.entity_name || "Unknown"}
                      {#if item.show_name}
                        <span class="text-text-muted">({item.show_name})</span>
                      {/if}
                    </p>
                    <p class="text-xs text-text-muted">
                      {formatChangeType(item.change_type)}
                      {#if item.old_value || item.new_value}
                        : {item.old_value || "none"} → {item.new_value || "none"}
                      {/if}
                    </p>
                  </div>
                  <span class="text-xs text-text-muted">{formatRelativeDate(item.changed_at)}</span>
                </div>
              {/each}
            </div>
          {/if}
        </div>
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
              <div class="bg-background rounded-lg p-4">
                <div class="flex items-center gap-2 mb-3">
                  <Copy class="w-4 h-4 text-yellow-400" />
                  <span class="text-sm text-yellow-400">{pair.similarity_reason}</span>
                </div>
                <div class="grid grid-cols-2 gap-4">
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
            <!-- Orphaned Episodes -->
            <div class="bg-background rounded-lg p-4">
              <div class="flex items-center justify-between">
                <div>
                  <h3 class="font-medium">Orphaned Episodes</h3>
                  <p class="text-sm text-text-muted">Episodes whose show has been deleted</p>
                </div>
                <div class="flex items-center gap-3">
                  <span class="text-lg font-bold {stats.orphaned_episodes > 0 ? 'text-yellow-400' : 'text-text-muted'}">
                    {stats.orphaned_episodes}
                  </span>
                  <button
                    type="button"
                    onclick={handleCleanupOrphaned}
                    disabled={stats.orphaned_episodes === 0}
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
                      {#if orphanedEpisodesList.length > 20}
                        <li class="text-text-muted italic">...and {orphanedEpisodesList.length - 20} more</li>
                      {/if}
                    </ul>
                  </div>
                {/if}
              {/if}
            </div>

            <!-- Unaired Episodes -->
            <div class="bg-background rounded-lg p-4">
              <div class="flex items-center justify-between">
                <div>
                  <h3 class="font-medium">Unaired & Unscheduled Episodes</h3>
                  <p class="text-sm text-text-muted">Episodes with no air date and not scheduled</p>
                </div>
                <div class="flex items-center gap-3">
                  <span class="text-lg font-bold {stats.unaired_unscheduled_episodes > 0 ? 'text-yellow-400' : 'text-text-muted'}">
                    {stats.unaired_unscheduled_episodes}
                  </span>
                  <button
                    type="button"
                    onclick={handleCleanupUnaired}
                    disabled={stats.unaired_unscheduled_episodes === 0}
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
                      {#if unairedEpisodesList.length > 20}
                        <li class="text-text-muted italic">...and {unairedEpisodesList.length - 20} more</li>
                      {/if}
                    </ul>
                  </div>
                {/if}
              {/if}
            </div>

            <!-- Optimize -->
            <div class="bg-background rounded-lg p-4">
              <div class="flex items-center justify-between">
                <div>
                  <h3 class="font-medium">Optimize Database</h3>
                  <p class="text-sm text-text-muted">VACUUM and rebuild indexes for better performance</p>
                </div>
                <button
                  type="button"
                  onclick={handleOptimize}
                  class="px-3 py-1.5 text-sm bg-accent/20 hover:bg-accent/30 text-accent rounded transition-colors"
                >
                  Optimize
                </button>
              </div>
            </div>

            <!-- Sync All -->
            <div class="bg-background rounded-lg p-4 border border-accent/30">
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-3">
                  <CloudDownload class="w-5 h-5 text-accent" />
                  <div>
                    <h3 class="font-medium">Sync All</h3>
                    <p class="text-sm text-text-muted">Refresh all show data from TVDB and movie data from TMDB</p>
                  </div>
                </div>
                <button
                  type="button"
                  onclick={handleSyncAll}
                  disabled={syncingAll}
                  class="px-3 py-1.5 text-sm bg-accent/20 hover:bg-accent/30 text-accent rounded transition-colors flex items-center gap-2 disabled:opacity-50"
                >
                  {#if syncingAll}
                    <RefreshCw class="w-4 h-4 animate-spin" />
                    Syncing...
                  {:else}
                    Sync All
                  {/if}
                </button>
              </div>
            </div>

            <!-- Full Cleanup -->
            <div class="bg-background rounded-lg p-4 border border-red-500/30">
              <div class="flex items-center justify-between">
                <div>
                  <h3 class="font-medium text-red-400">Full Cleanup</h3>
                  <p class="text-sm text-text-muted">Remove all orphaned data and optimize database</p>
                </div>
                <button
                  type="button"
                  onclick={handleFullCleanup}
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
      <div class="p-3 border-t border-border bg-available/10 text-available text-sm text-center">
        {cleanupMessage}
      </div>
    {/if}
  </div>
{/if}
