import { invoke } from "@tauri-apps/api/core";
import { logger } from "../utils/logger";
import { getDatabase } from "../utils/database";

export type AwardType = "oscars" | "emmys";

export interface CeremonySummary {
  id: number;
  award_type: string;
  edition: number;
  name: string;
  year: number;
  ceremony_date: string | null;
  status: string; // "past" | "nominated" | "upcoming"
}

export interface NomineeRow {
  id: number;
  title: string;
  detail: string | null;
  is_winner: boolean | null;
}

export interface CategoryDetail {
  id: number;
  name: string;
  nominees: NomineeRow[];
}

export interface CeremonyDetail extends CeremonySummary {
  categories: CategoryDetail[];
}

export interface SyncSummary {
  ceremonies: number;
  categories: number;
  nominees: number;
  winners: number;
  errors: string[];
}

interface PredictionResults {
  picks: { category_id: number; nominee_id: number }[];
  correct: number;
  total: number;
}

let awardType = $state<AwardType>("oscars");
let ceremonies = $state<CeremonySummary[]>([]);
let selectedCeremony = $state<CeremonyDetail | null>(null);
let loading = $state(false);
let syncing = $state(false);
let lastSync = $state<string | null>(null); // ISO timestamp of the last sync
// Predictions for the selected ceremony: category_id -> nominee_id, plus score.
let predictions = $state<Record<number, number>>({});
let score = $state<{ correct: number; total: number } | null>(null);

export function getAwardType() {
  return awardType;
}
export function getCeremonies() {
  return ceremonies;
}
export function getSelectedCeremony() {
  return selectedCeremony;
}
export function isLoading() {
  return loading;
}
export function isSyncing() {
  return syncing;
}
export function getLastSync() {
  return lastSync;
}

async function loadLastSync() {
  try {
    const db = await getDatabase();
    const rows = await db.select<{ value: string }[]>(
      "SELECT value FROM settings WHERE key = 'awards_last_sync'",
    );
    lastSync = rows.length > 0 ? rows[0].value : null;
  } catch (e) {
    logger.error("[awards] load last sync failed", e);
  }
}

export async function setAwardType(t: AwardType) {
  if (awardType === t) return;
  awardType = t;
  selectedCeremony = null;
  await loadCeremonies();
}

export async function loadCeremonies() {
  loading = true;
  try {
    ceremonies = await invoke<CeremonySummary[]>("get_award_ceremonies", {
      awardType,
    });
    await loadLastSync();
  } catch (e) {
    logger.error("[awards] load ceremonies failed", e);
    ceremonies = [];
  } finally {
    loading = false;
  }
}

export async function selectCeremony(id: number) {
  loading = true;
  try {
    selectedCeremony = await invoke<CeremonyDetail>("get_ceremony_detail", {
      ceremonyId: id,
    });
    await loadResults(id);
  } catch (e) {
    logger.error("[awards] load ceremony detail failed", e);
  } finally {
    loading = false;
  }
}

export function clearSelectedCeremony() {
  selectedCeremony = null;
  predictions = {};
  score = null;
}

export function getPrediction(categoryId: number): number | undefined {
  return predictions[categoryId];
}

export function getScore() {
  return score;
}

async function loadResults(ceremonyId: number) {
  try {
    const res = await invoke<PredictionResults>("get_award_prediction_results", {
      ceremonyId,
    });
    const map: Record<number, number> = {};
    for (const p of res.picks) map[p.category_id] = p.nominee_id;
    predictions = map;
    score = { correct: res.correct, total: res.total };
  } catch (e) {
    logger.error("[awards] load prediction results failed", e);
    predictions = {};
    score = null;
  }
}

export async function setPrediction(categoryId: number, nomineeId: number) {
  try {
    await invoke("set_award_prediction", { categoryId, nomineeId });
    predictions = { ...predictions, [categoryId]: nomineeId };
    if (selectedCeremony) await loadResults(selectedCeremony.id);
  } catch (e) {
    logger.error("[awards] set prediction failed", e);
  }
}

export async function clearPrediction(categoryId: number) {
  try {
    await invoke("clear_award_prediction", { categoryId });
    const next = { ...predictions };
    delete next[categoryId];
    predictions = next;
    if (selectedCeremony) await loadResults(selectedCeremony.id);
  } catch (e) {
    logger.error("[awards] clear prediction failed", e);
  }
}

/** Pull fresh data from Wikipedia. `full` re-pulls 20 years; otherwise incremental. */
export async function refreshAwards(full = false): Promise<SyncSummary> {
  syncing = true;
  try {
    const summary = await invoke<SyncSummary>("sync_awards", { full });
    await loadCeremonies();
    await loadLastSync();
    if (selectedCeremony) {
      await selectCeremony(selectedCeremony.id);
    }
    return summary;
  } finally {
    syncing = false;
  }
}
