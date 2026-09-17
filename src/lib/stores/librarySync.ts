import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { loadTrackedShows, loadArchivedShows, refreshCalendar } from "./shows.svelte";
import { loadTrackedMovies, loadArchivedMovies, refreshMoviesCalendar, isMovieDetailOpen, getCurrentMovie, openMovieDetail } from "./movies.svelte";
import { isShowDetailOpen, getCurrentShow, openShowDetail } from "./showDetail.svelte";
import { loadTierListShows, loadTierListMovies } from "./tiers.svelte";
import { showError } from "./toast.svelte";

export type LibraryKind = "shows" | "movies";
export type SyncFrequency = "off" | "daily" | "weekly" | "monthly";

export interface SyncReport {
  kind: LibraryKind;
  source: "manual" | "automatic" | "retry";
  started_at: string;
  finished_at: string | null;
  total: number;
  succeeded: number;
  skipped: number;
  failures: { id: number; title: string; error: string }[];
  current_title: string | null;
  error: string | null;
}

export interface LibrarySyncStatus {
  last_success_at: string | null;
  kind: LibraryKind;
  frequency: SyncFrequency;
  next_sync_at: string | null;
  running: boolean;
  interrupted: boolean;
  report: SyncReport | null;
}

async function refreshLibrary(kind: LibraryKind) {
  if (kind === "shows") {
    const detailId = isShowDetailOpen() ? getCurrentShow()?.id : undefined;
    await Promise.all([loadTrackedShows(), loadArchivedShows(), loadTierListShows(), refreshCalendar()]);
    if (detailId !== undefined && isShowDetailOpen() && getCurrentShow()?.id === detailId) {
      await openShowDetail(detailId);
    }
  } else {
    const detailId = isMovieDetailOpen() ? getCurrentMovie()?.id : undefined;
    await Promise.all([loadTrackedMovies(), loadArchivedMovies(), loadTierListMovies(), refreshMoviesCalendar()]);
    if (detailId !== undefined && isMovieDetailOpen() && getCurrentMovie()?.id === detailId) {
      await openMovieDetail(detailId);
    }
  }
}

// Listen at app level so automatic sync also refreshes a library when Data is closed.
export function setupLibrarySyncListener() {
  let disposed = false;
  let unlisten: UnlistenFn | undefined;
  listen<LibraryKind>("library-sync-finished", ({ payload }) => {
    void refreshLibrary(payload).catch((error) => showError(`Could not refresh the library view: ${error}`));
  }).then((stop) => {
    if (disposed) stop();
    else unlisten = stop;
  }).catch((error) => showError(`Could not listen for sync updates: ${error}`));
  return () => {
    disposed = true;
    unlisten?.();
  };
}
