/**
 * Centralized type definitions
 * Re-export all types from their respective modules for easy importing
 */

// Show-related types
export type {
  TrackedShow,
  Episode,
  ShowEpisode,
} from "../stores/shows.svelte";

export type {
  ShowDetail,
  CastMember,
} from "../stores/showDetail.svelte";

// Movie-related types
export type {
  TrackedMovie,
  MovieSearchResult,
  CalendarMovie,
  MovieDetail,
  MovieCastMember,
  MovieCrewMember,
  TrailerData,
} from "../stores/movies.svelte";

// Statistics types
export type {
  WatchStatistics,
  PeriodStatistics,
  ShowCompletion,
  WatchHistoryItem,
} from "../stores/statistics.svelte";

// Data management types
export type {
  DatabaseStats,
  ChangeHistoryItem,
  DuplicatePair,
  CleanupEpisodePreview,
} from "../stores/dataManagement.svelte";

// ARR types
export type {
  ArrServer,
  ArrServerRequest,
  ArrSystemStatus,
  LibraryItem,
  ImportItem,
  ImportRequest,
  ImportResult,
} from "../stores/arr.svelte";

// Plex types
export type {
  PlexConfig,
  PlexServerStatus,
  ScrobbleLogEntry,
} from "../stores/plex.svelte";

// Theme types
export type { ThemeSettings } from "../stores/theme.svelte";

// Toast types
export type { Toast } from "../stores/toast.svelte";
