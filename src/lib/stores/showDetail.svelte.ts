import { invoke } from "@tauri-apps/api/core";
import Database from "@tauri-apps/plugin-sql";
import type { Episode } from "./shows.svelte";

// Use separate database in dev mode to avoid breaking production data
const DB_NAME = import.meta.env.DEV ? "sqlite:tvc_dev.db" : "sqlite:tvc.db";

export interface CastMember {
  id: number;
  person_id: number | null;
  name: string;
  character_name: string | null;
  image_url: string | null;
  order_index: number;
}

export interface ShowDetail {
  id: number;
  name: string;
  slug: string | null;
  status: string | null;
  poster_url: string | null;
  first_aired: string | null;
  network: string | null;
  overview: string | null;
  airs_time: string | null;
  airs_days: string | null;
  runtime: number | null;
  added_at: string | null;
  last_synced: string | null;
  color: string | null;
  notes: string | null;
  tags: string | null;
  rating: number | null;
}

let db: Database | null = null;

async function getDb(): Promise<Database> {
  if (!db) {
    db = await Database.load(DB_NAME);
  }
  return db;
}

// Show detail state
let showDetailOpen = $state(false);
let currentShow = $state<ShowDetail | null>(null);
let showEpisodes = $state<Episode[]>([]);
let showCast = $state<CastMember[]>([]);
let castLoading = $state(false);
let loading = $state(false);
let error = $state<string | null>(null);

export function isShowDetailOpen() {
  return showDetailOpen;
}

export function getCurrentShow() {
  return currentShow;
}

export function getShowEpisodes() {
  return showEpisodes;
}

export function isLoading() {
  return loading;
}

export function getError() {
  return error;
}

export function getShowCast() {
  return showCast;
}

export function isCastLoading() {
  return castLoading;
}

export async function openShowDetail(showId: number): Promise<void> {
  showDetailOpen = true;
  loading = true;
  error = null;

  try {
    // Get show details from database
    const database = await getDb();
    const shows = await database.select<ShowDetail[]>(
      `SELECT id, name, slug, status, poster_url, first_aired, network, overview, 
       airs_time, airs_days, runtime, added_at, last_synced, color, notes, tags, rating
       FROM shows WHERE id = $1`,
      [showId]
    );

    if (shows.length === 0) {
      error = "Show not found";
      return;
    }

    currentShow = shows[0];

    // Get all episodes for this show
    const episodes = await database.select<Episode[]>(
      `SELECT e.id, e.show_id, s.name as show_name, s.network,
       COALESCE(e.season_number, 0) as season_number,
       COALESCE(e.episode_number, 0) as episode_number,
       e.name, e.aired, e.scheduled_date, e.watched = 1 as watched,
       s.poster_url
       FROM episodes e
       JOIN shows s ON e.show_id = s.id
       WHERE e.show_id = $1
       ORDER BY e.season_number, e.episode_number`,
      [showId]
    );

    showEpisodes = episodes.map((ep) => ({
      id: ep.id,
      show_id: ep.show_id,
      show_name: ep.show_name,
      network: ep.network,
      season_number: ep.season_number,
      episode_number: ep.episode_number,
      name: ep.name,
      aired: ep.aired,
      scheduled_date: ep.scheduled_date,
      watched: (ep.watched as unknown as number) === 1,
      poster_url: ep.poster_url,
    }));
  } catch (err) {
    console.error("Failed to load show detail:", err);
    error = err instanceof Error ? err.message : "Failed to load show details";
  } finally {
    loading = false;
  }
}

export function closeShowDetail() {
  showDetailOpen = false;
  currentShow = null;
  showEpisodes = [];
  showCast = [];
  error = null;
}

export async function fetchShowCast(showId: number): Promise<void> {
  castLoading = true;

  try {
    const cast = await invoke<CastMember[]>("get_show_cast", { showId });
    showCast = cast;
  } catch (err) {
    console.error("Failed to fetch show cast:", err);
    // Don't set error state for cast - it's optional
  } finally {
    castLoading = false;
  }
}

export async function syncShowEpisodes(showId: number): Promise<void> {
  loading = true;
  error = null;

  try {
    await invoke("sync_show_episodes", { showId });
    // Reload episodes
    if (currentShow) {
      await openShowDetail(showId);
    }
  } catch (err) {
    console.error("Failed to sync episodes:", err);
    error = err instanceof Error ? err.message : "Failed to sync episodes";
  } finally {
    loading = false;
  }
}

export async function updateShowRating(
  showId: number,
  rating: number | null
): Promise<void> {
  try {
    await invoke("update_show_rating", { id: showId, rating });
    // Reload show details
    if (currentShow) {
      await openShowDetail(showId);
    }
    // Also refresh tracked shows to update sidebar
    const { loadTrackedShows } = await import("./shows.svelte");
    await loadTrackedShows();
  } catch (err) {
    console.error("Failed to update show rating:", err);
    error = err instanceof Error ? err.message : "Failed to update rating";
  }
}

export async function markSeasonWatched(
  showId: number,
  seasonNumber: number,
  watched: boolean
): Promise<void> {
  try {
    await invoke("mark_season_watched", { showId, seasonNumber, watched });
    // Update local state immediately
    showEpisodes = showEpisodes.map((ep) =>
      ep.season_number === seasonNumber ? { ...ep, watched } : ep
    );
    // Update calendar episodes as well
    const { updateCalendarEpisodesWatched } = await import("./shows.svelte");
    updateCalendarEpisodesWatched(showId, watched, seasonNumber);
  } catch (err) {
    console.error("Failed to mark season watched:", err);
    error = err instanceof Error ? err.message : "Failed to mark season watched";
  }
}

export async function markShowWatched(
  showId: number,
  watched: boolean
): Promise<void> {
  try {
    await invoke("mark_show_watched", { showId, watched });
    // Update local state immediately
    showEpisodes = showEpisodes.map((ep) => ({ ...ep, watched }));
    // Update calendar episodes as well
    const { updateCalendarEpisodesWatched } = await import("./shows.svelte");
    updateCalendarEpisodesWatched(showId, watched);
  } catch (err) {
    console.error("Failed to mark show watched:", err);
    error = err instanceof Error ? err.message : "Failed to mark show watched";
  }
}

export async function markEpisodeWatched(
  episodeId: number,
  watched: boolean
): Promise<void> {
  try {
    await invoke("mark_episode_watched", { episodeId, watched });
    // Update local state immediately
    showEpisodes = showEpisodes.map((ep) =>
      ep.id === episodeId ? { ...ep, watched } : ep
    );
    // Update calendar episodes as well
    const { updateEpisodeWatched } = await import("./shows.svelte");
    updateEpisodeWatched(episodeId, watched);
  } catch (err) {
    console.error("Failed to mark episode watched:", err);
    error = err instanceof Error ? err.message : "Failed to mark episode watched";
  }
}

