import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { logger } from "../utils/logger";
import {
  loadTrackedShows,
  loadArchivedShows,
  refreshCalendar,
} from "./shows.svelte";
import { loadTierListShows } from "./tiers.svelte";

export interface UnmigratedShow {
  id: number;
  legacy_tvdb_id: number | null;
  name: string;
  poster_url: string | null;
  first_aired: string | null;
}

export interface TmdbTvSearchResult {
  id: number;
  name: string;
  original_name: string | null;
  overview: string | null;
  poster_path: string | null;
  first_air_date: string | null;
  vote_average: number | null;
  origin_country: string[] | null;
}

export interface RemapOutcome {
  episodes_orphaned: number;
  merged_with: number | null;
}

interface PerShowResult {
  name: string;
  new_tmdb_id: number;
  episodes_orphaned: number;
  merged_with: number | null;
}

interface StartedEvent {
  total: number;
}

interface ProgressEvent {
  done: number;
  total: number;
  current_name: string;
}

interface FinishedEvent {
  mapped: number;
  quarantined: number;
  errors: string[];
  per_show: PerShowResult[];
}

let inProgress = $state(false);
let total = $state(0);
let done = $state(0);
let currentName = $state("");
let lastFinished = $state<FinishedEvent | null>(null);
let unmigratedShows = $state<UnmigratedShow[]>([]);
let unmigratedError = $state<string | null>(null);
let unmigratedLoading = $state(false);
let unmigratedRequest = 0;
let busyShows = $state<Set<number>>(new Set());

export function isMigrationInProgress(): boolean {
  return inProgress;
}

export function getMigrationTotal(): number {
  return total;
}

export function getMigrationDone(): number {
  return done;
}

export function getMigrationCurrentName(): string {
  return currentName;
}

export function getLastMigrationResult(): FinishedEvent | null {
  return lastFinished;
}

export function getUnmigratedShows(): UnmigratedShow[] {
  return unmigratedShows;
}
export function getUnmigratedShowsError() { return unmigratedError; }
export function isUnmigratedShowsLoading() { return unmigratedLoading; }
export function isUnmigratedShowBusy(id: number) { return busyShows.has(id); }

export function dismissMigrationResult() {
  lastFinished = null;
}

export async function loadUnmigratedShows(): Promise<void> {
  const request = ++unmigratedRequest;
  unmigratedLoading = true;
  unmigratedError = null;
  try {
    const rows = await invoke<UnmigratedShow[]>("get_unmigrated_shows");
    if (request === unmigratedRequest) unmigratedShows = rows;
  } catch (e) {
    if (request !== unmigratedRequest) return;
    unmigratedError = e instanceof Error ? e.message : String(e);
    logger.error("[Migration] failed to load unmigrated shows", e);
  } finally { if (request === unmigratedRequest) unmigratedLoading = false; }
}

export async function searchTmdbTv(
  query: string,
): Promise<TmdbTvSearchResult[]> {
  if (!query.trim()) return [];
  try {
    return await invoke<TmdbTvSearchResult[]>("search_tmdb_tv", { query });
  } catch (e) {
    logger.error("[Migration] search_tmdb_tv failed", e);
    throw e;
  }
}

export async function resolveUnmigratedShow(
  oldId: number,
  newTmdbId: number,
): Promise<RemapOutcome> {
  if (busyShows.has(oldId)) throw new Error("This show is already being updated.");
  busyShows = new Set(busyShows).add(oldId);
  try {
    const outcome = await invoke<RemapOutcome>("resolve_unmigrated_show", {
      oldId,
      newTmdbId,
    });
    await loadUnmigratedShows();
    await Promise.all([loadTrackedShows(), loadArchivedShows(), loadTierListShows()]);
    await refreshCalendar();
    return outcome;
  } finally {
    const next = new Set(busyShows); next.delete(oldId); busyShows = next;
  }
}

export async function deleteUnmigratedShow(id: number): Promise<void> {
  if (busyShows.has(id)) throw new Error("This show is already being updated.");
  busyShows = new Set(busyShows).add(id);
  try {
    await invoke("delete_unmigrated_show", { id });
    await loadUnmigratedShows();
    await Promise.all([loadTrackedShows(), loadArchivedShows(), loadTierListShows()]);
    await refreshCalendar();
  } finally {
    const next = new Set(busyShows); next.delete(id); busyShows = next;
  }
}

export function setupMigrationListener(): () => void {
  const unlistenFns: UnlistenFn[] = [];

  listen<StartedEvent>("tvdb_migration_started", (event) => {
    inProgress = true;
    total = event.payload.total;
    done = 0;
    currentName = "";
    lastFinished = null;
  }).then((fn) => unlistenFns.push(fn));

  listen<ProgressEvent>("tvdb_migration_progress", (event) => {
    done = event.payload.done;
    total = event.payload.total;
    currentName = event.payload.current_name;
  }).then((fn) => unlistenFns.push(fn));

  listen<FinishedEvent>("tvdb_migration_finished", (event) => {
    inProgress = false;
    lastFinished = event.payload;
    // Refresh the resolver list so any newly quarantined rows show up.
    loadUnmigratedShows();
    // The remap rewrote shows.id and episodes.show_id in place; the cached
    // frontend stores still hold the old TVDB ids, so clicking a sidebar
    // entry would SELECT WHERE id=<old> and get "Show not found". Reload
    // everything that surfaces show ids.
    loadTrackedShows().catch((e) =>
      logger.error("[Migration] reload tracked shows failed", e),
    );
    loadArchivedShows().catch((e) =>
      logger.error("[Migration] reload archived shows failed", e),
    );
    loadTierListShows().catch((e) =>
      logger.error("[Migration] reload tier-list shows failed", e),
    );
    refreshCalendar().catch((e) =>
      logger.error("[Migration] refresh calendar failed", e),
    );
  }).then((fn) => unlistenFns.push(fn));

  return () => unlistenFns.forEach((fn) => fn());
}
