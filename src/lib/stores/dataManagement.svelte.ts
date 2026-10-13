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
  optimization_error?: string | null;
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
let activeTab = $state<"overview" | "sync" | "duplicates" | "cleanup" | "archive">("overview");
let statsLoading = $state(false);
let duplicateLoading = $state(false);
let writing = $state(false);
let statsRequest = 0;
let duplicateRequest = 0;
let previewRequest = 0;
let previewLoading = $state(false);
let previewError = $state<string | null>(null);
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
  return writing || (activeTab === "overview" && statsLoading)
    || (activeTab === "cleanup" && (statsLoading || previewLoading))
    || (activeTab === "duplicates" && duplicateLoading);
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

export function getCleanupPreviewError() { return previewError; }
export function isCleanupPreviewLoading() { return previewLoading; }

// Actions
export function openDataManagement() {
  modalOpen = true;
  activeTab = "overview";
  loadDatabaseStats();
}

export function closeDataManagement() {
  modalOpen = false;
  statsRequest++; duplicateRequest++; previewRequest++;
  statsLoading = false; duplicateLoading = false; previewLoading = false;
  error = null;
}

export function setActiveTab(tab: "overview" | "sync" | "duplicates" | "cleanup" | "archive") {
  if (tab !== activeTab) {
    error = null;
    if (!writing) {
      if (tab !== "overview" && tab !== "cleanup") { statsRequest++; statsLoading = false; }
      if (tab !== "duplicates") { duplicateRequest++; duplicateLoading = false; }
      if (tab !== "cleanup") { previewRequest++; previewLoading = false; }
    }
  }
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

// Data loading: keep failed previews distinct from a genuinely empty preview.
export async function loadDatabaseStats(): Promise<boolean> {
  const request = ++statsRequest;
  statsLoading = true;
  if (activeTab === "overview" || activeTab === "cleanup" || writing) error = null;
  try {
    const result = await invoke<DatabaseStats>("get_database_stats");
    if (request !== statsRequest) return false;
    databaseStats = result;
    return true;
  } catch (cause) {
    if (request === statsRequest) {
      databaseStats = null;
      error = `Could not load database summary: ${String(cause)}`;
      logger.error("Failed to load database stats", cause);
    }
    return false;
  } finally { if (request === statsRequest) statsLoading = false; }
}

export async function loadDuplicates(): Promise<boolean> {
  const request = ++duplicateRequest;
  duplicateLoading = true;
  error = null;
  try {
    const rows = await invoke<DuplicatePair[]>("find_duplicates");
    if (request !== duplicateRequest) return false;
    duplicates = rows;
    return true;
  } catch (cause) {
    if (request === duplicateRequest) { duplicates = []; error = `Could not load duplicate titles: ${String(cause)}`; logger.error("Failed to load duplicates", cause); }
    return false;
  } finally { if (request === duplicateRequest) duplicateLoading = false; }
}

export async function loadCleanupPreviews(): Promise<boolean> {
  const request = ++previewRequest;
  previewLoading = true;
  previewError = null;
  orphanedEpisodes = []; unairedEpisodes = [];
  try {
    const [orphaned, unaired] = await Promise.all([
      invoke<CleanupEpisodePreview[]>("get_orphaned_episodes_preview"),
      invoke<CleanupEpisodePreview[]>("get_unaired_episodes_preview"),
    ]);
    if (request !== previewRequest) return false;
    orphanedEpisodes = orphaned; unairedEpisodes = unaired;
    return true;
  } catch (cause) {
    if (request === previewRequest) {
      previewError = `Could not load cleanup previews: ${String(cause)}`;
      logger.error("Failed to load cleanup previews", cause);
    }
    return false;
  } finally { if (request === previewRequest) previewLoading = false; }
}

function beginWrite() {
  if (writing) throw new Error("Wait for the current Data operation to finish.");
  writing = true; error = null;
}
function endWrite() { writing = false; }

async function refreshLibraryViews(deletedShowId?: number, keepShowId?: number) {
  const shows = await import("./shows.svelte");
  const movies = await import("./movies.svelte");
  const tiers = await import("./tiers.svelte");
  const detail = await import("./showDetail.svelte");
  const showId = detail.isShowDetailOpen() ? detail.getCurrentShow()?.id : undefined;
  const movieId = movies.isMovieDetailOpen() ? movies.getCurrentMovie()?.id : undefined;
  const refreshed = await Promise.all([
    shows.loadTrackedShows(), shows.loadArchivedShows(), tiers.loadTierListShows(),
    movies.loadTrackedMovies(), movies.loadArchivedMovies(), tiers.loadTierListMovies(),
    shows.refreshCalendar(), movies.refreshMoviesCalendar(),
  ]);
  if (showId !== undefined && detail.isShowDetailOpen() && detail.getCurrentShow()?.id === showId) {
    await detail.openShowDetail(showId === deletedShowId && keepShowId !== undefined ? keepShowId : showId);
  }
  if (movieId !== undefined && movies.isMovieDetailOpen() && movies.getCurrentMovie()?.id === movieId) await movies.openMovieDetail(movieId);
  if (refreshed.some(result => result === false)) throw new Error("Changes were saved, but some library views could not refresh. Reload Data and try refreshing again.");
}

async function refreshAfterCleanup(summary: string) {
  const [statsOk, previewsOk] = await Promise.all([loadDatabaseStats(), loadCleanupPreviews()]);
  await refreshLibraryViews();
  if (!statsOk || !previewsOk) throw new Error(`${summary} The refreshed summary or preview could not be loaded. Retry refreshing Data.`);
}

export async function cleanupOrphaned(): Promise<number> {
  beginWrite();
  let removed: number | undefined;
  try {
    removed = await invoke<number>("cleanup_orphaned_episodes");
    await refreshAfterCleanup(`Removed ${removed} unused orphaned episodes.`);
    return removed;
  } catch (cause) {
    error = removed !== undefined ? `Removed ${removed} unused orphaned episodes, but refreshing Data failed: ${String(cause)}` : `Could not clean up orphaned episodes: ${String(cause)}`;
    logger.error(error, cause); throw new Error(error);
  } finally { endWrite(); }
}

export async function cleanupUnaired(): Promise<number> {
  beginWrite();
  let removed: number | undefined;
  try {
    removed = await invoke<number>("cleanup_unaired_episodes");
    await refreshAfterCleanup(`Removed ${removed} unused episodes without dates.`);
    return removed;
  } catch (cause) {
    error = removed !== undefined ? `Removed ${removed} unused episodes without dates, but refreshing Data failed: ${String(cause)}` : `Could not clean up unused episodes: ${String(cause)}`;
    logger.error(error, cause); throw new Error(error);
  } finally { endWrite(); }
}

export async function optimizeDatabase(): Promise<void> {
  beginWrite();
  let optimized = false;
  try {
    await invoke("optimize_database"); optimized = true;
    if (!await loadDatabaseStats()) throw new Error("Storage was optimized, but the refreshed database summary could not be loaded.");
  } catch (cause) {
    error = optimized ? String(cause) : `Could not optimize storage: ${String(cause)}`;
    logger.error(error, cause); throw new Error(error);
  } finally { endWrite(); }
}

export async function runFullCleanup(): Promise<CleanupResult> {
  beginWrite();
  let result: CleanupResult | undefined;
  try {
    result = await invoke<CleanupResult>("run_full_cleanup");
    await refreshAfterCleanup(`Cleanup removed ${result.orphaned_episodes_removed + result.unaired_episodes_removed} unused episodes.`);
    return result;
  } catch (cause) {
    error = result ? `Cleanup committed (${result.orphaned_episodes_removed + result.unaired_episodes_removed} episodes removed), but refreshing Data failed: ${String(cause)}${result.optimization_error ? ` ${result.optimization_error}` : ""}` : `Could not complete cleanup: ${String(cause)}`;
    logger.error(error, cause); throw new Error(error);
  } finally { endWrite(); }
}

export async function mergeDuplicates(keepId: number, mergeId: number): Promise<MergeResult> {
  beginWrite();
  let result: MergeResult | undefined;
  try {
    result = await invoke<MergeResult>("merge_duplicates", { keepId, mergeId });
    const [duplicatesOk, statsOk] = await Promise.all([loadDuplicates(), loadDatabaseStats()]);
    await refreshLibraryViews(result.deleted_show_id, keepId);
    if (!duplicatesOk || !statsOk) throw new Error("The refreshed duplicate list or summary could not be loaded.");
    return result;
  } catch (cause) {
    error = result ? `The shows were merged, but refreshing Data failed: ${String(cause)}` : `Could not merge shows: ${String(cause)}`;
    logger.error(error, cause); throw new Error(error);
  } finally { endWrite(); }
}

// Helpers
export function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

