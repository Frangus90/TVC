import { invoke } from "@tauri-apps/api/core";
import { logger } from "../utils/logger";

export interface Tier {
  id: number;
  position: number;
  name: string;
  color: string;
  created_at: string;
}

export interface TierListShow {
  id: number;
  name: string;
  poster_url: string | null;
  tier_id: number | null;
  rank_order: number | null;
  tier_only: boolean;
}

export interface TierListMovie {
  id: number;
  title: string;
  poster_url: string | null;
  tier_id: number | null;
  rank_order: number | null;
  tier_only: boolean;
}

export interface DeleteTierResult {
  affected_shows: number;
  affected_movies: number;
}

// State
let tiers = $state<Tier[]>([]);
let tierPreset = $state<string>("10-star");
let tierListShows = $state<TierListShow[]>([]);
let tierListMovies = $state<TierListMovie[]>([]);
let tiersLoading = $state(false);
let tierSearchModalOpen = $state(false);

// Getters
export function getTiers(): Tier[] {
  return tiers;
}

export function getTierPreset(): string {
  return tierPreset;
}

export function getTierById(id: number): Tier | undefined {
  return tiers.find((t) => t.id === id);
}

export function getTierListShows(): TierListShow[] {
  return tierListShows;
}

export function getTierListMovies(): TierListMovie[] {
  return tierListMovies;
}

export function isTiersLoading(): boolean {
  return tiersLoading;
}

export function isTierSearchModalOpen(): boolean {
  return tierSearchModalOpen;
}

// Actions
export function openTierSearchModal() {
  tierSearchModalOpen = true;
}

export function closeTierSearchModal() {
  tierSearchModalOpen = false;
}

export async function loadTiers(): Promise<void> {
  tiersLoading = true;
  try {
    tiers = await invoke<Tier[]>("get_tiers");
  } catch (error) {
    logger.error("Failed to load tiers", error);
  } finally {
    tiersLoading = false;
  }
}

export async function loadTierPreset(): Promise<void> {
  try {
    tierPreset = await invoke<string>("get_tier_preset");
  } catch (error) {
    logger.error("Failed to load tier preset", error);
  }
}

export async function loadTierListShows(): Promise<void> {
  try {
    tierListShows = await invoke<TierListShow[]>("get_tier_list_shows");
  } catch (error) {
    logger.error("Failed to load tier list shows", error);
  }
}

export async function loadTierListMovies(): Promise<void> {
  try {
    tierListMovies = await invoke<TierListMovie[]>("get_tier_list_movies");
  } catch (error) {
    logger.error("Failed to load tier list movies", error);
  }
}

export async function createTier(name: string, color: string): Promise<Tier | null> {
  try {
    const tier = await invoke<Tier>("create_tier", { name, color });
    await loadTiers();
    return tier;
  } catch (error) {
    logger.error("Failed to create tier", error);
    return null;
  }
}

export async function updateTierInfo(
  id: number,
  name?: string,
  color?: string
): Promise<void> {
  try {
    await invoke("update_tier", {
      id,
      name: name ?? null,
      color: color ?? null,
    });
    await loadTiers();
  } catch (error) {
    logger.error("Failed to update tier", error);
  }
}

export async function deleteTier(id: number): Promise<DeleteTierResult | null> {
  try {
    const result = await invoke<DeleteTierResult>("delete_tier", { id });
    await loadTiers();
    await loadTierListShows();
    await loadTierListMovies();
    return result;
  } catch (error) {
    logger.error("Failed to delete tier", error);
    return null;
  }
}

export async function reorderTiers(tierIds: number[]): Promise<void> {
  try {
    await invoke("reorder_tiers", { tierIds });
    await loadTiers();
  } catch (error) {
    logger.error("Failed to reorder tiers", error);
  }
}

export async function applyPreset(preset: string): Promise<void> {
  try {
    await invoke("apply_tier_preset", { preset });
    tierPreset = preset;
    await loadTiers();
    await loadTierListShows();
    await loadTierListMovies();
  } catch (error) {
    logger.error("Failed to apply tier preset", error);
  }
}

// Tier assignment
export async function updateShowTier(
  showId: number,
  tierId: number | null
): Promise<void> {
  try {
    await invoke("update_show_tier", { id: showId, tierId });
    await loadTierListShows();
  } catch (error) {
    logger.error("Failed to update show tier", error);
  }
}

export async function updateMovieTier(
  movieId: number,
  tierId: number | null
): Promise<void> {
  try {
    await invoke("update_movie_tier", { id: movieId, tierId });
    await loadTierListMovies();
  } catch (error) {
    logger.error("Failed to update movie tier", error);
  }
}

// Tier-only item management
export async function addShowTierOnly(
  id: number,
  tierId: number | null
): Promise<void> {
  try {
    await invoke("add_show_tier_only", { id, tierId });
    await loadTierListShows();
  } catch (error) {
    logger.error("Failed to add show to tier list", error);
  }
}

export async function addMovieTierOnly(
  id: number,
  tierId: number | null
): Promise<void> {
  try {
    await invoke("add_movie_tier_only", { id, tierId });
    await loadTierListMovies();
  } catch (error) {
    logger.error("Failed to add movie to tier list", error);
  }
}

export async function addManualShow(
  title: string,
  posterUrl: string | null,
  tierId: number | null
): Promise<number | null> {
  try {
    const id = await invoke<number>("add_manual_show", {
      title,
      posterUrl,
      tierId,
    });
    await loadTierListShows();
    return id;
  } catch (error) {
    logger.error("Failed to add manual show", error);
    return null;
  }
}

export async function addManualMovie(
  title: string,
  posterUrl: string | null,
  tierId: number | null
): Promise<number | null> {
  try {
    const id = await invoke<number>("add_manual_movie", {
      title,
      posterUrl,
      tierId,
    });
    await loadTierListMovies();
    return id;
  } catch (error) {
    logger.error("Failed to add manual movie", error);
    return null;
  }
}

export async function promoteShowToTracked(id: number): Promise<void> {
  try {
    await invoke("promote_show_to_tracked", { id });
    await loadTierListShows();
    // Also refresh tracked shows since it's now tracked
    const { loadTrackedShows } = await import("./shows.svelte");
    await loadTrackedShows();
  } catch (error) {
    logger.error("Failed to promote show to tracked", error);
  }
}

export async function promoteMovieToTracked(id: number): Promise<void> {
  try {
    await invoke("promote_movie_to_tracked", { id });
    await loadTierListMovies();
    const { loadTrackedMovies } = await import("./movies.svelte");
    await loadTrackedMovies();
  } catch (error) {
    logger.error("Failed to promote movie to tracked", error);
  }
}

export async function demoteShowToTierOnly(id: number): Promise<void> {
  try {
    await invoke("demote_show_to_tier_only", { id });
    await loadTierListShows();
    const { loadTrackedShows } = await import("./shows.svelte");
    await loadTrackedShows();
  } catch (error) {
    logger.error("Failed to demote show to tier only", error);
  }
}

export async function demoteMovieToTierOnly(id: number): Promise<void> {
  try {
    await invoke("demote_movie_to_tier_only", { id });
    await loadTierListMovies();
    const { loadTrackedMovies } = await import("./movies.svelte");
    await loadTrackedMovies();
  } catch (error) {
    logger.error("Failed to demote movie to tier only", error);
  }
}
