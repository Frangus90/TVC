import { invoke } from "@tauri-apps/api/core";

// Types
export interface ChangeHistoryItem {
  id: number;
  entity_type: string;
  entity_id: number;
  change_type: string;
  old_value: string | null;
  new_value: string | null;
  changed_at: string;
  user_action: string | null;
  entity_name: string | null;
  show_name: string | null;
  poster_url: string | null;
}

export interface ChangeHistoryStats {
  total_changes: number;
  watched_changes: number;
  schedule_changes: number;
  rating_changes: number;
}

export interface DuplicatePair {
  show1_id: number;
  show1_name: string;
  show1_episode_count: number;
  show1_watched_count: number;
  show1_poster_url: string | null;
  show2_id: number;
  show2_name: string;
  show2_episode_count: number;
  show2_watched_count: number;
  show2_poster_url: string | null;
  similarity_reason: string;
}

export interface MergeResult {
  episodes_moved: number;
  episodes_merged: number;
  deleted_show_id: number;
}

export interface DatabaseStats {
  total_shows: number;
  total_episodes: number;
  total_movies: number;
  orphaned_episodes: number;
  unaired_unscheduled_episodes: number;
  database_size_bytes: number;
  change_history_count: number;
}

export interface CleanupResult {
  orphaned_episodes_removed: number;
  unaired_episodes_removed: number;
  history_entries_removed: number;
}

// State
let modalOpen = $state(false);
let activeTab = $state<"overview" | "history" | "duplicates" | "cleanup">("overview");
let loading = $state(false);
let error = $state<string | null>(null);

// Data states
let databaseStats = $state<DatabaseStats | null>(null);
let changeHistory = $state<ChangeHistoryItem[]>([]);
let historyStats = $state<ChangeHistoryStats | null>(null);
let duplicates = $state<DuplicatePair[]>([]);

// Getters
export function isModalOpen() {
  return modalOpen;
}

export function getActiveTab() {
  return activeTab;
}

export function isLoading() {
  return loading;
}

export function getError() {
  return error;
}

export function getDatabaseStats() {
  return databaseStats;
}

export function getChangeHistory() {
  return changeHistory;
}

export function getHistoryStats() {
  return historyStats;
}

export function getDuplicates() {
  return duplicates;
}

// Actions
export function openDataManagement() {
  modalOpen = true;
  activeTab = "overview";
  loadDatabaseStats();
}

export function closeDataManagement() {
  modalOpen = false;
  error = null;
}

export function setActiveTab(tab: "overview" | "history" | "duplicates" | "cleanup") {
  activeTab = tab;

  // Load data for the tab
  if (tab === "overview" || tab === "cleanup") {
    loadDatabaseStats();
  } else if (tab === "history") {
    loadChangeHistory();
  } else if (tab === "duplicates") {
    loadDuplicates();
  }
}

// Data loading
export async function loadDatabaseStats() {
  loading = true;
  error = null;

  try {
    databaseStats = await invoke<DatabaseStats>("get_database_stats");
  } catch (err) {
    console.error("Failed to load database stats:", err);
    error = err instanceof Error ? err.message : String(err);
  } finally {
    loading = false;
  }
}

export async function loadChangeHistory(limit = 100) {
  loading = true;
  error = null;

  try {
    const [history, stats] = await Promise.all([
      invoke<ChangeHistoryItem[]>("get_change_history", { limit }),
      invoke<ChangeHistoryStats>("get_change_history_stats"),
    ]);
    changeHistory = history;
    historyStats = stats;
  } catch (err) {
    console.error("Failed to load change history:", err);
    error = err instanceof Error ? err.message : String(err);
  } finally {
    loading = false;
  }
}

export async function loadDuplicates() {
  loading = true;
  error = null;

  try {
    duplicates = await invoke<DuplicatePair[]>("find_duplicates");
  } catch (err) {
    console.error("Failed to load duplicates:", err);
    error = err instanceof Error ? err.message : String(err);
  } finally {
    loading = false;
  }
}

// Cleanup actions
export async function cleanupOrphaned(): Promise<number> {
  loading = true;
  error = null;

  try {
    const count = await invoke<number>("cleanup_orphaned_episodes");
    await loadDatabaseStats();
    return count;
  } catch (err) {
    console.error("Failed to cleanup orphaned episodes:", err);
    error = err instanceof Error ? err.message : String(err);
    throw err;
  } finally {
    loading = false;
  }
}

export async function cleanupUnaired(): Promise<number> {
  loading = true;
  error = null;

  try {
    const count = await invoke<number>("cleanup_unaired_episodes");
    await loadDatabaseStats();
    return count;
  } catch (err) {
    console.error("Failed to cleanup unaired episodes:", err);
    error = err instanceof Error ? err.message : String(err);
    throw err;
  } finally {
    loading = false;
  }
}

export async function optimizeDatabase(): Promise<void> {
  loading = true;
  error = null;

  try {
    await invoke("optimize_database");
    await loadDatabaseStats();
  } catch (err) {
    console.error("Failed to optimize database:", err);
    error = err instanceof Error ? err.message : String(err);
    throw err;
  } finally {
    loading = false;
  }
}

export async function runFullCleanup(): Promise<CleanupResult> {
  loading = true;
  error = null;

  try {
    const result = await invoke<CleanupResult>("run_full_cleanup");
    await loadDatabaseStats();
    return result;
  } catch (err) {
    console.error("Failed to run full cleanup:", err);
    error = err instanceof Error ? err.message : String(err);
    throw err;
  } finally {
    loading = false;
  }
}

export async function clearHistory(): Promise<number> {
  loading = true;
  error = null;

  try {
    const count = await invoke<number>("clear_change_history");
    await loadChangeHistory();
    return count;
  } catch (err) {
    console.error("Failed to clear history:", err);
    error = err instanceof Error ? err.message : String(err);
    throw err;
  } finally {
    loading = false;
  }
}

// Duplicate actions
export async function mergeDuplicates(keepId: number, mergeId: number): Promise<MergeResult> {
  loading = true;
  error = null;

  try {
    const result = await invoke<MergeResult>("merge_duplicates", { keepId, mergeId });
    await loadDuplicates();
    return result;
  } catch (err) {
    console.error("Failed to merge duplicates:", err);
    error = err instanceof Error ? err.message : String(err);
    throw err;
  } finally {
    loading = false;
  }
}

// Helpers
export function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

export function formatChangeType(type: string): string {
  switch (type) {
    case "watched": return "Marked Watched";
    case "scheduled": return "Scheduled";
    case "unscheduled": return "Unscheduled";
    case "rating": return "Rating Changed";
    default: return type;
  }
}

export function formatRelativeDate(dateStr: string): string {
  const date = new Date(dateStr + "Z");
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffMins < 1) return "Just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;

  return date.toLocaleDateString();
}
