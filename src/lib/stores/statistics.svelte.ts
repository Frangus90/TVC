import { invoke } from "@tauri-apps/api/core";

export interface WatchStatistics {
  total_watch_time_minutes: number;
  episodes_watched: number;
  movies_watched: number;
  shows_completed: number;
  shows_in_progress: number;
}

export interface PeriodStats {
  date: string;
  episodes_count: number;
  movies_count: number;
  total_runtime: number;
}

export interface ShowCompletion {
  show_id: number;
  show_name: string;
  poster_url: string | null;
  total_episodes: number;
  watched_episodes: number;
  completion_percentage: number;
}

export interface WatchHistoryItem {
  item_type: "episode" | "movie";
  id: number;
  name: string;
  show_name: string | null;
  season_number: number | null;
  episode_number: number | null;
  watched_at: string;
  poster_url: string | null;
  runtime: number | null;
}

// State
let statisticsModalOpen = $state(false);
let statistics = $state<WatchStatistics | null>(null);
let periodStats = $state<PeriodStats[]>([]);
let completionRates = $state<ShowCompletion[]>([]);
let watchHistory = $state<WatchHistoryItem[]>([]);
let loading = $state(false);
let activeTab = $state<"overview" | "history">("overview");

// Getters
export function isStatisticsModalOpen() {
  return statisticsModalOpen;
}

export function getStatistics() {
  return statistics;
}

export function getPeriodStats() {
  return periodStats;
}

export function getCompletionRates() {
  return completionRates;
}

export function getWatchHistory() {
  return watchHistory;
}

export function isLoading() {
  return loading;
}

export function getActiveTab() {
  return activeTab;
}

// Actions
export function openStatisticsModal() {
  statisticsModalOpen = true;
  loadAllStatistics();
}

export function closeStatisticsModal() {
  statisticsModalOpen = false;
}

export function setActiveTab(tab: "overview" | "history") {
  activeTab = tab;
}

export async function loadAllStatistics(): Promise<void> {
  loading = true;
  try {
    await Promise.all([
      loadWatchStatistics(),
      loadCompletionRates(),
      loadWatchHistory(),
    ]);
  } finally {
    loading = false;
  }
}

export async function loadWatchStatistics(): Promise<void> {
  try {
    statistics = await invoke<WatchStatistics>("get_watch_statistics");
  } catch (error) {
    console.error("Failed to load watch statistics:", error);
    statistics = null;
  }
}

export async function loadPeriodStats(
  startDate: string,
  endDate: string,
  groupBy?: "day" | "month" | "year"
): Promise<void> {
  try {
    periodStats = await invoke<PeriodStats[]>("get_episodes_watched_by_period", {
      startDate,
      endDate,
      groupBy,
    });
  } catch (error) {
    console.error("Failed to load period stats:", error);
    periodStats = [];
  }
}

export async function loadCompletionRates(): Promise<void> {
  try {
    completionRates = await invoke<ShowCompletion[]>("get_completion_rates");
  } catch (error) {
    console.error("Failed to load completion rates:", error);
    completionRates = [];
  }
}

export async function loadWatchHistory(limit?: number): Promise<void> {
  try {
    watchHistory = await invoke<WatchHistoryItem[]>("get_watch_history", { limit });
  } catch (error) {
    console.error("Failed to load watch history:", error);
    watchHistory = [];
  }
}

// Helper functions
export function formatWatchTime(minutes: number): string {
  if (minutes < 60) {
    return `${minutes}m`;
  }
  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;
  if (hours < 24) {
    return remainingMinutes > 0 ? `${hours}h ${remainingMinutes}m` : `${hours}h`;
  }
  const days = Math.floor(hours / 24);
  const remainingHours = hours % 24;
  return remainingHours > 0 ? `${days}d ${remainingHours}h` : `${days}d`;
}

export function formatRelativeDate(dateString: string): string {
  const date = new Date(dateString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

  if (diffDays === 0) {
    const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
    if (diffHours === 0) {
      const diffMinutes = Math.floor(diffMs / (1000 * 60));
      return diffMinutes <= 1 ? "Just now" : `${diffMinutes}m ago`;
    }
    return diffHours === 1 ? "1 hour ago" : `${diffHours} hours ago`;
  }
  if (diffDays === 1) return "Yesterday";
  if (diffDays < 7) return `${diffDays} days ago`;
  if (diffDays < 30) {
    const weeks = Math.floor(diffDays / 7);
    return weeks === 1 ? "1 week ago" : `${weeks} weeks ago`;
  }
  if (diffDays < 365) {
    const months = Math.floor(diffDays / 30);
    return months === 1 ? "1 month ago" : `${months} months ago`;
  }
  const years = Math.floor(diffDays / 365);
  return years === 1 ? "1 year ago" : `${years} years ago`;
}

export function groupHistoryByDate(
  history: WatchHistoryItem[]
): Map<string, WatchHistoryItem[]> {
  const groups = new Map<string, WatchHistoryItem[]>();

  for (const item of history) {
    const date = new Date(item.watched_at);
    const dateKey = date.toLocaleDateString("en-US", {
      weekday: "long",
      year: "numeric",
      month: "long",
      day: "numeric",
    });

    if (!groups.has(dateKey)) {
      groups.set(dateKey, []);
    }
    groups.get(dateKey)!.push(item);
  }

  return groups;
}
