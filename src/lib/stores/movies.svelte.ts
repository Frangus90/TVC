import { invoke } from "@tauri-apps/api/core";

export interface TrackedMovie {
  id: number;
  title: string;
  tagline: string | null;
  poster_url: string | null;
  release_date: string | null;
  digital_release_date: string | null;
  runtime: number | null;
  status: string | null;
  scheduled_date: string | null;
  watched: boolean;
  rating: number | null;
  color: string | null;
  archived: boolean;
}

export interface MovieSearchResult {
  id: number;
  title: string;
  overview: string | null;
  poster_url: string | null;
  release_date: string | null;
  vote_average: number | null;
}

export interface CalendarMovie {
  id: number;
  title: string;
  poster_url: string | null;
  runtime: number | null;
  scheduled_date: string | null;
  digital_release_date: string | null;
  watched: boolean;
  color: string | null;
}

export interface MovieDetail {
  id: number;
  title: string;
  tagline: string | null;
  overview: string | null;
  poster_url: string | null;
  backdrop_url: string | null;
  release_date: string | null;
  digital_release_date: string | null;
  physical_release_date: string | null;
  runtime: number | null;
  status: string | null;
  genres: string | null;
  vote_average: number | null;
  scheduled_date: string | null;
  watched: boolean;
  watched_at: string | null;
  rating: number | null;
  notes: string | null;
  color: string | null;
  tags: string | null;
  archived: boolean;
  added_at: string | null;
  last_synced: string | null;
}

// Application state
let trackedMovies = $state<TrackedMovie[]>([]);
let archivedMovies = $state<TrackedMovie[]>([]);
let calendarMovies = $state<CalendarMovie[]>([]);
let movieSearchModalOpen = $state(false);
let movieSearchQuery = $state("");
let movieSearchResults = $state<MovieSearchResult[]>([]);
let movieSearchLoading = $state(false);
let currentCalendarRange = $state<{ start: string; end: string } | null>(null);

// Movie detail state
let movieDetailOpen = $state(false);
let currentMovie = $state<MovieDetail | null>(null);
let movieDetailLoading = $state(false);
let movieDetailError = $state<string | null>(null);

// Getters
export function getTrackedMovies() {
  return trackedMovies;
}

export function getArchivedMovies() {
  return archivedMovies;
}

export function getCalendarMovies() {
  return calendarMovies;
}

export function isMovieSearchModalOpen() {
  return movieSearchModalOpen;
}

export function getMovieSearchQuery() {
  return movieSearchQuery;
}

export function getMovieSearchResults() {
  return movieSearchResults;
}

export function isMovieSearchLoading() {
  return movieSearchLoading;
}

export function isMovieDetailOpen() {
  return movieDetailOpen;
}

export function getCurrentMovie() {
  return currentMovie;
}

export function isMovieDetailLoading() {
  return movieDetailLoading;
}

export function getMovieDetailError() {
  return movieDetailError;
}

// Actions
export function openMovieSearchModal() {
  movieSearchModalOpen = true;
  movieSearchQuery = "";
  movieSearchResults = [];
}

export function closeMovieSearchModal() {
  movieSearchModalOpen = false;
  movieSearchQuery = "";
  movieSearchResults = [];
}

export function setMovieSearchQuery(query: string) {
  movieSearchQuery = query;
}

export async function searchMovies(query: string): Promise<void> {
  if (!query.trim()) {
    movieSearchResults = [];
    return;
  }

  movieSearchLoading = true;
  try {
    const results = await invoke<MovieSearchResult[]>("search_movies", { query });
    movieSearchResults = results;
  } catch (error) {
    console.error("Movie search error:", error);
    movieSearchResults = [];
  } finally {
    movieSearchLoading = false;
  }
}

export async function loadTrackedMovies(): Promise<void> {
  try {
    const movies = await invoke<TrackedMovie[]>("get_tracked_movies");
    trackedMovies = movies;
  } catch (error) {
    console.error("Failed to load tracked movies:", error);
  }
}

export async function loadArchivedMovies(): Promise<void> {
  try {
    const movies = await invoke<TrackedMovie[]>("get_archived_movies");
    archivedMovies = movies;
  } catch (error) {
    console.error("Failed to load archived movies:", error);
  }
}

export async function addMovie(movie: MovieSearchResult): Promise<void> {
  try {
    await invoke("add_movie", { id: movie.id });
    await loadTrackedMovies();
  } catch (error) {
    console.error("Failed to add movie:", error);
  }
}

export async function removeMovie(movieId: number): Promise<void> {
  try {
    await invoke("remove_movie", { id: movieId });
    await loadTrackedMovies();
    calendarMovies = calendarMovies.filter((m) => m.id !== movieId);
  } catch (error) {
    console.error("Failed to remove movie:", error);
  }
}

export async function loadMoviesForRange(
  startDate: string,
  endDate: string
): Promise<void> {
  currentCalendarRange = { start: startDate, end: endDate };
  try {
    const movies = await invoke<CalendarMovie[]>("get_movies_for_range", {
      startDate,
      endDate,
    });
    calendarMovies = movies;
  } catch (error) {
    console.error("Failed to load movies for range:", error);
  }
}

export async function markMovieWatched(
  movieId: number,
  watched: boolean
): Promise<void> {
  try {
    await invoke("mark_movie_watched", { id: movieId, watched });
    // Update local state
    calendarMovies = calendarMovies.map((m) =>
      m.id === movieId ? { ...m, watched } : m
    );
    trackedMovies = trackedMovies.map((m) =>
      m.id === movieId ? { ...m, watched } : m
    );
    if (currentMovie && currentMovie.id === movieId) {
      currentMovie = { ...currentMovie, watched };
    }
  } catch (error) {
    console.error("Failed to mark movie watched:", error);
  }
}

export async function scheduleMovie(
  movieId: number,
  date: string
): Promise<void> {
  try {
    await invoke("schedule_movie", { id: movieId, date });
    // Update local state
    trackedMovies = trackedMovies.map((m) =>
      m.id === movieId ? { ...m, scheduled_date: date } : m
    );
    if (currentMovie && currentMovie.id === movieId) {
      currentMovie = { ...currentMovie, scheduled_date: date };
    }
    // Refresh calendar
    if (currentCalendarRange) {
      await loadMoviesForRange(currentCalendarRange.start, currentCalendarRange.end);
    }
  } catch (error) {
    console.error("Failed to schedule movie:", error);
  }
}

export async function unscheduleMovie(movieId: number): Promise<void> {
  try {
    await invoke("unschedule_movie", { id: movieId });
    // Update local state
    trackedMovies = trackedMovies.map((m) =>
      m.id === movieId ? { ...m, scheduled_date: null } : m
    );
    calendarMovies = calendarMovies.map((m) =>
      m.id === movieId ? { ...m, scheduled_date: null } : m
    );
    if (currentMovie && currentMovie.id === movieId) {
      currentMovie = { ...currentMovie, scheduled_date: null };
    }
    // Refresh calendar
    if (currentCalendarRange) {
      await loadMoviesForRange(currentCalendarRange.start, currentCalendarRange.end);
    }
  } catch (error) {
    console.error("Failed to unschedule movie:", error);
  }
}

export async function archiveMovie(movieId: number): Promise<void> {
  try {
    await invoke("archive_movie", { id: movieId });
    await loadTrackedMovies();
    await loadArchivedMovies();
    calendarMovies = calendarMovies.filter((m) => m.id !== movieId);
    if (currentMovie && currentMovie.id === movieId) {
      closeMovieDetail();
    }
  } catch (error) {
    console.error("Failed to archive movie:", error);
  }
}

export async function unarchiveMovie(movieId: number): Promise<void> {
  try {
    await invoke("unarchive_movie", { id: movieId });
    await loadTrackedMovies();
    await loadArchivedMovies();
  } catch (error) {
    console.error("Failed to unarchive movie:", error);
  }
}

export async function updateMovieRating(
  movieId: number,
  rating: number | null
): Promise<void> {
  try {
    await invoke("update_movie_rating", { id: movieId, rating });
    // Update local state
    trackedMovies = trackedMovies.map((m) =>
      m.id === movieId ? { ...m, rating } : m
    );
    if (currentMovie && currentMovie.id === movieId) {
      currentMovie = { ...currentMovie, rating };
    }
  } catch (error) {
    console.error("Failed to update movie rating:", error);
  }
}

export async function syncMovie(movieId: number): Promise<void> {
  try {
    await invoke("sync_movie", { id: movieId });
    await loadTrackedMovies();
    if (currentMovie && currentMovie.id === movieId) {
      await openMovieDetail(movieId);
    }
  } catch (error) {
    console.error("Failed to sync movie:", error);
  }
}

// Movie detail functions
export async function openMovieDetail(movieId: number): Promise<void> {
  movieDetailOpen = true;
  movieDetailLoading = true;
  movieDetailError = null;

  try {
    const movie = await invoke<MovieDetail>("get_movie_details", { id: movieId });
    currentMovie = movie;
  } catch (error) {
    console.error("Failed to load movie detail:", error);
    movieDetailError = error instanceof Error ? error.message : "Failed to load movie details";
  } finally {
    movieDetailLoading = false;
  }
}

export function closeMovieDetail() {
  movieDetailOpen = false;
  currentMovie = null;
  movieDetailError = null;
}

// Helper to get movies for a specific date
export function getMoviesForDate(date: string): CalendarMovie[] {
  return calendarMovies.filter((m) => {
    const displayDate = m.scheduled_date || m.digital_release_date;
    return displayDate === date;
  });
}

// Format runtime as "Xh Ym"
export function formatRuntime(minutes: number | null): string {
  if (!minutes) return "";
  const hours = Math.floor(minutes / 60);
  const mins = minutes % 60;
  if (hours > 0 && mins > 0) return `${hours}h ${mins}m`;
  if (hours > 0) return `${hours}h`;
  return `${mins}m`;
}
