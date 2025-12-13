import { invoke } from "@tauri-apps/api/core";
import Database from "@tauri-apps/plugin-sql";

export interface TrackedShow {
  id: number;
  name: string;
  poster_url: string | null;
  status: string | null;
}

export interface SearchResult {
  tvdb_id: string | null;
  id: string | null;
  name: string | null;
  slug: string | null;
  image_url: string | null;
  status: string | null;
  first_air_time: string | null;
  overview: string | null;
  network: string | null;
  year: string | null;
}

export interface Episode {
  id: number;
  show_id: number;
  show_name: string;
  season_number: number;
  episode_number: number;
  name: string | null;
  aired: string | null;
  scheduled_date: string | null;
  watched: boolean;
  poster_url: string | null;
}

export interface ShowEpisode {
  id: number;
  season_number: number;
  episode_number: number;
  name: string | null;
  aired: string | null;
  scheduled_date: string | null;
  watched: boolean;
}

let db: Database | null = null;

async function getDb(): Promise<Database> {
  if (!db) {
    db = await Database.load("sqlite:tvc.db");
    // Ensure scheduled_date column exists (migration)
    await db.execute(`
      CREATE TABLE IF NOT EXISTS _migrations (version INTEGER PRIMARY KEY);
    `);
    const migrations = await db.select<{ version: number }[]>(
      "SELECT version FROM _migrations WHERE version = 2"
    );
    if (migrations.length === 0) {
      try {
        await db.execute("ALTER TABLE episodes ADD COLUMN scheduled_date TEXT");
      } catch {
        // Column might already exist
      }
      await db.execute("INSERT OR IGNORE INTO _migrations (version) VALUES (2)");
    }
  }
  return db;
}

// Application state
let trackedShows = $state<TrackedShow[]>([]);
let searchModalOpen = $state(false);
let searchQuery = $state("");
let searchResults = $state<SearchResult[]>([]);
let searchLoading = $state(false);
let calendarEpisodes = $state<Episode[]>([]);
let currentCalendarRange = $state<{ start: string; end: string } | null>(null);

// Episode picker state
let episodePickerOpen = $state(false);
let episodePickerShow = $state<TrackedShow | null>(null);
let episodePickerEpisodes = $state<ShowEpisode[]>([]);
let episodePickerDate = $state<string | null>(null);

// Day detail state
let dayDetailOpen = $state(false);
let dayDetailDate = $state<string | null>(null);

// Getters
export function getTrackedShows() {
  return trackedShows;
}

export function isSearchModalOpen() {
  return searchModalOpen;
}

export function getSearchQuery() {
  return searchQuery;
}

export function getSearchResults() {
  return searchResults;
}

export function isSearchLoading() {
  return searchLoading;
}

export function getCalendarEpisodes() {
  return calendarEpisodes;
}

export function isEpisodePickerOpen() {
  return episodePickerOpen;
}

export function getEpisodePickerShow() {
  return episodePickerShow;
}

export function getEpisodePickerEpisodes() {
  return episodePickerEpisodes;
}

export function getEpisodePickerDate() {
  return episodePickerDate;
}

export function isDayDetailOpen() {
  return dayDetailOpen;
}

export function getDayDetailDate() {
  return dayDetailDate;
}

// Search relevance scoring
function scoreSearchResult(result: SearchResult, query: string): number {
  const name = (result.name || "").toLowerCase();
  const q = query.toLowerCase().trim();

  // Exact match = highest priority
  if (name === q) return 100;

  // Starts with query = high priority
  if (name.startsWith(q)) return 80;

  // Contains query as whole word = medium-high priority
  const wordBoundary = new RegExp(`\\b${q.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\b`, "i");
  if (wordBoundary.test(name)) return 60;

  // Contains query = medium priority
  if (name.includes(q)) return 40;

  // Default = low priority
  return 10;
}

// Actions
export function openSearchModal() {
  searchModalOpen = true;
  searchQuery = "";
  searchResults = [];
}

export function closeSearchModal() {
  searchModalOpen = false;
  searchQuery = "";
  searchResults = [];
}

export function setSearchQuery(query: string) {
  searchQuery = query;
}

export async function searchShows(query: string): Promise<void> {
  if (!query.trim()) {
    searchResults = [];
    return;
  }

  searchLoading = true;
  try {
    const results = await invoke<SearchResult[]>("search_shows", { query });
    // Sort by relevance
    searchResults = results.sort((a, b) => {
      const scoreA = scoreSearchResult(a, query);
      const scoreB = scoreSearchResult(b, query);
      return scoreB - scoreA;
    });
  } catch (error) {
    console.error("Search error:", error);
    searchResults = [];
  } finally {
    searchLoading = false;
  }
}

export async function loadTrackedShows(): Promise<void> {
  try {
    const database = await getDb();
    const rows = await database.select<TrackedShow[]>(
      "SELECT id, name, poster_url, status FROM shows ORDER BY name"
    );
    trackedShows = rows;
  } catch (error) {
    console.error("Failed to load tracked shows:", error);
  }
}

export async function addShow(show: SearchResult): Promise<void> {
  const showId = parseInt(show.tvdb_id || show.id || "0");
  if (!showId) return;

  try {
    const details = await invoke<{
      id: number;
      name: string;
      slug: string | null;
      image: string | null;
      status: { name: string | null } | null;
      first_aired: string | null;
      overview: string | null;
      airs_time: string | null;
      airs_days: {
        sunday: boolean | null;
        monday: boolean | null;
        tuesday: boolean | null;
        wednesday: boolean | null;
        thursday: boolean | null;
        friday: boolean | null;
        saturday: boolean | null;
      } | null;
      average_runtime: number | null;
    }>("get_show_details", { id: showId });

    const database = await getDb();

    const airsDaysJson = details.airs_days
      ? JSON.stringify(details.airs_days)
      : null;

    await database.execute(
      `INSERT OR REPLACE INTO shows (id, name, slug, status, poster_url, first_aired, overview, airs_time, airs_days, runtime)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)`,
      [
        details.id,
        details.name,
        details.slug,
        details.status?.name,
        details.image,
        details.first_aired,
        details.overview,
        details.airs_time,
        airsDaysJson,
        details.average_runtime,
      ]
    );

    // Update UI immediately
    await loadTrackedShows();
    closeSearchModal();

    // Sync episodes in background (don't await)
    syncShowEpisodes(details.id).then(() => {
      // Refresh calendar after sync completes
      if (currentCalendarRange) {
        loadEpisodesForRange(currentCalendarRange.start, currentCalendarRange.end);
      }
    });
  } catch (error) {
    console.error("Failed to add show:", error);
  }
}

export async function removeShow(showId: number): Promise<void> {
  try {
    const database = await getDb();
    await database.execute("DELETE FROM shows WHERE id = $1", [showId]);
    await loadTrackedShows();

    // Immediately remove episodes from calendar state
    calendarEpisodes = calendarEpisodes.filter((ep) => ep.show_id !== showId);
  } catch (error) {
    console.error("Failed to remove show:", error);
  }
}

export async function syncShowEpisodes(showId: number): Promise<void> {
  try {
    const episodes = await invoke<
      {
        id: number;
        series_id: number | null;
        name: string | null;
        aired: string | null;
        runtime: number | null;
        image: string | null;
        season_number: number | null;
        episode_number: number | null;
        overview: string | null;
      }[]
    >("sync_episodes_for_show", { showId });

    const database = await getDb();

    for (const ep of episodes) {
      await database.execute(
        `INSERT OR REPLACE INTO episodes (id, show_id, season_number, episode_number, name, overview, aired, runtime, image_url)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)`,
        [
          ep.id,
          showId,
          ep.season_number,
          ep.episode_number,
          ep.name,
          ep.overview,
          ep.aired,
          ep.runtime,
          ep.image,
        ]
      );
    }

    await database.execute(
      "UPDATE shows SET last_synced = datetime('now') WHERE id = $1",
      [showId]
    );
  } catch (error) {
    console.error("Failed to sync episodes:", error);
  }
}

export async function loadEpisodesForRange(
  startDate: string,
  endDate: string
): Promise<void> {
  currentCalendarRange = { start: startDate, end: endDate };
  try {
    const database = await getDb();
    const rows = await database.select<
      {
        id: number;
        show_id: number;
        name: string | null;
        season_number: number;
        episode_number: number;
        aired: string | null;
        scheduled_date: string | null;
        watched: number;
        show_name: string;
        poster_url: string | null;
      }[]
    >(
      `SELECT e.id, e.show_id, e.name, e.season_number, e.episode_number, e.aired, e.scheduled_date, e.watched,
              s.name as show_name, s.poster_url
       FROM episodes e
       JOIN shows s ON e.show_id = s.id
       WHERE (e.aired >= $1 AND e.aired <= $2) OR (e.scheduled_date >= $1 AND e.scheduled_date <= $2)
       ORDER BY COALESCE(e.scheduled_date, e.aired), s.name`,
      [startDate, endDate]
    );

    calendarEpisodes = rows.map((row) => ({
      id: row.id,
      show_id: row.show_id,
      show_name: row.show_name,
      season_number: row.season_number,
      episode_number: row.episode_number,
      name: row.name,
      aired: row.aired,
      scheduled_date: row.scheduled_date,
      watched: row.watched === 1,
      poster_url: row.poster_url,
    }));
  } catch (error) {
    console.error("Failed to load episodes:", error);
  }
}

export async function toggleEpisodeWatched(
  episodeId: number,
  watched: boolean
): Promise<void> {
  try {
    const database = await getDb();
    await database.execute(
      `UPDATE episodes SET watched = $1, watched_at = $2 WHERE id = $3`,
      [watched ? 1 : 0, watched ? new Date().toISOString() : null, episodeId]
    );

    calendarEpisodes = calendarEpisodes.map((ep) =>
      ep.id === episodeId ? { ...ep, watched } : ep
    );
  } catch (error) {
    console.error("Failed to toggle episode watched:", error);
  }
}

// Episode scheduling functions
export async function openEpisodePicker(show: TrackedShow, date: string): Promise<void> {
  episodePickerShow = show;
  episodePickerDate = date;
  episodePickerOpen = true;

  try {
    const database = await getDb();
    const rows = await database.select<ShowEpisode[]>(
      `SELECT id, season_number, episode_number, name, aired, scheduled_date, watched
       FROM episodes
       WHERE show_id = $1
       ORDER BY season_number, episode_number`,
      [show.id]
    );
    episodePickerEpisodes = rows.map((r) => ({
      ...r,
      watched: (r.watched as unknown as number) === 1,
    }));
  } catch (error) {
    console.error("Failed to load episodes for picker:", error);
    episodePickerEpisodes = [];
  }
}

export function closeEpisodePicker() {
  episodePickerOpen = false;
  episodePickerShow = null;
  episodePickerEpisodes = [];
  episodePickerDate = null;
}

export async function scheduleEpisode(episodeId: number, date: string): Promise<void> {
  try {
    const database = await getDb();
    await database.execute(
      "UPDATE episodes SET scheduled_date = $1 WHERE id = $2",
      [date, episodeId]
    );

    // Refresh calendar
    if (currentCalendarRange) {
      await loadEpisodesForRange(currentCalendarRange.start, currentCalendarRange.end);
    }

    closeEpisodePicker();
  } catch (error) {
    console.error("Failed to schedule episode:", error);
  }
}

export async function scheduleMultipleEpisodes(episodeIds: number[], date: string): Promise<void> {
  try {
    const database = await getDb();

    // Schedule all episodes
    for (const episodeId of episodeIds) {
      await database.execute(
        "UPDATE episodes SET scheduled_date = $1 WHERE id = $2",
        [date, episodeId]
      );
    }

    // Refresh calendar
    if (currentCalendarRange) {
      await loadEpisodesForRange(currentCalendarRange.start, currentCalendarRange.end);
    }

    closeEpisodePicker();
  } catch (error) {
    console.error("Failed to schedule episodes:", error);
  }
}

export async function unscheduleEpisode(episodeId: number): Promise<void> {
  try {
    const database = await getDb();
    await database.execute(
      "UPDATE episodes SET scheduled_date = NULL WHERE id = $1",
      [episodeId]
    );

    // Remove from calendar or refresh
    if (currentCalendarRange) {
      await loadEpisodesForRange(currentCalendarRange.start, currentCalendarRange.end);
    }
  } catch (error) {
    console.error("Failed to unschedule episode:", error);
  }
}

// Day detail functions
export function openDayDetail(date: string) {
  dayDetailDate = date;
  dayDetailOpen = true;
}

export function closeDayDetail() {
  dayDetailOpen = false;
  dayDetailDate = null;
}

export function getEpisodesForDate(date: string): Episode[] {
  return calendarEpisodes.filter((ep) => {
    const displayDate = ep.scheduled_date || ep.aired;
    return displayDate === date;
  });
}
