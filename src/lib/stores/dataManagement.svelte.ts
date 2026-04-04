import { invoke } from "@tauri-apps/api/core";
import { logger } from "../utils/logger";

// Types
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
}

export interface CleanupResult {
  orphaned_episodes_removed: number;
  unaired_episodes_removed: number;
}

export interface CleanupEpisodePreview {
  id: number;
  show_name: string;
  season_number: number;
  episode_number: number;
  name: string | null;
}

// State
let modalOpen = $state(false);
let activeTab = $state<"overview" | "duplicates" | "cleanup">("overview");
let loading = $state(false);
let error = $state<string | null>(null);

// Data states
let databaseStats = $state<DatabaseStats | null>(null);
let duplicates = $state<DuplicatePair[]>([]);
let orphanedEpisodes = $state<CleanupEpisodePreview[]>([]);
let unairedEpisodes = $state<CleanupEpisodePreview[]>([]);

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

export function getDuplicates() {
  return duplicates;
}

export function getOrphanedEpisodes() {
  return orphanedEpisodes;
}

export function getUnairedEpisodes() {
  return unairedEpisodes;
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

export function setActiveTab(tab: "overview" | "duplicates" | "cleanup") {
  activeTab = tab;

  // Load data for the tab
  if (tab === "overview") {
    loadDatabaseStats();
  } else if (tab === "cleanup") {
    loadDatabaseStats();
    loadCleanupPreviews();
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
    logger.error("Failed to load database stats:", err);
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
    logger.error("Failed to load duplicates:", err);
    error = err instanceof Error ? err.message : String(err);
  } finally {
    loading = false;
  }
}

export async function loadCleanupPreviews() {
  try {
    const [orphaned, unaired] = await Promise.all([
      invoke<CleanupEpisodePreview[]>("get_orphaned_episodes_preview"),
      invoke<CleanupEpisodePreview[]>("get_unaired_episodes_preview"),
    ]);
    orphanedEpisodes = orphaned;
    unairedEpisodes = unaired;
  } catch (err) {
    logger.error("Failed to load cleanup previews:", err);
  }
}

// Cleanup actions
export async function cleanupOrphaned(): Promise<number> {
  loading = true;
  error = null;

  try {
    const count = await invoke<number>("cleanup_orphaned_episodes");
    await loadDatabaseStats();
    await loadCleanupPreviews();
    return count;
  } catch (err) {
    logger.error("Failed to cleanup orphaned episodes:", err);
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
    await loadCleanupPreviews();
    return count;
  } catch (err) {
    logger.error("Failed to cleanup unaired episodes:", err);
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
    logger.error("Failed to optimize database:", err);
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
    logger.error("Failed to run full cleanup:", err);
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
    logger.error("Failed to merge duplicates:", err);
    error = err instanceof Error ? err.message : String(err);
    throw err;
  } finally {
    loading = false;
  }
}

// Helpers
export function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

