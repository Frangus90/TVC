let listRequest = 0;
let detailRequest = 0;
let resultsRequest = 0;
import { showError } from "./toast.svelte";
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
  nominations_date: string | null;
  status: string; // "past" | "nominated" | "upcoming"
  prediction_count: number;
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
  attempted_at: string;
  failed_ceremonies: [string, number][];
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
let detailLoading = $state(false);
let syncing = $state(false);
let syncReport = $state<SyncSummary | null>(null);
export function getAwardsSyncReport() { return syncReport; }
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
  return loading || detailLoading;
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
    const reports = await db.select<{ value: string }[]>("SELECT value FROM settings WHERE key = 'awards_sync_report'");
    syncReport = reports[0] ? JSON.parse(reports[0].value) : null;
  } catch (e) {
    logger.error("[awards] load last sync failed", e);
    showError("[awards] load last sync failed" + ": " + String(e));
  }
}

export async function setAwardType(t: AwardType) {
  if (awardType === t) return;
  awardType = t;
  clearSelectedCeremony();
  await loadCeremonies();
}

export async function loadCeremonies() {
  const request = ++listRequest;
  loading = true;
  try {
    const result = await invoke<CeremonySummary[]>("get_award_ceremonies", { awardType });
    if (request !== listRequest) return;
    ceremonies = result;
    await loadLastSync();
  } catch (error) {
    if (request === listRequest) showError(`Could not load awards: ${error}`);
  } finally {
    if (request === listRequest) loading = false;
  }
}

export async function selectCeremony(id: number) {
  clearSelectedCeremony();
  const request = ++detailRequest;
  detailLoading = true;
  try {
    const ceremony = await invoke<CeremonyDetail>("get_ceremony_detail", { ceremonyId: id });
    if (request !== detailRequest) return;
    selectedCeremony = ceremony;
    await loadResults(id);
  } catch (error) {
    if (request === detailRequest) showError(`Could not load ceremony: ${error}`);
  } finally {
    if (request === detailRequest) detailLoading = false;
  }
}

export function clearSelectedCeremony() {
  detailRequest++; resultsRequest++;
  detailLoading = false;
  selectedCeremony = null;
  predictions = {};
  score = null;
}

export function getPrediction(categoryId: number): number | undefined {
  return predictions[categoryId];
}

export function getPredictionsMap(): Record<number, number> {
  return predictions;
}

export function getScore() {
  return score;
}

async function loadResults(ceremonyId: number) {
  const request = ++resultsRequest;
  try {
    const res = await invoke<PredictionResults>("get_award_prediction_results", {
      ceremonyId,
    });
    if (request !== resultsRequest || selectedCeremony?.id !== ceremonyId) return;
    const map: Record<number, number> = {};
    for (const p of res.picks) map[p.category_id] = p.nominee_id;
    predictions = map;
    score = { correct: res.correct, total: res.total };
  } catch (e) {
    if (request !== resultsRequest) return;
    logger.error("[awards] load prediction results failed", e);
    showError("[awards] load prediction results failed" + ": " + String(e));
    predictions = {};
    score = null;
  }
}

export async function setPrediction(categoryId: number, nomineeId: number) {
  const request = detailRequest;
  try {
    await invoke("set_award_prediction", { categoryId, nomineeId });
    if (request !== detailRequest) return;
    predictions = { ...predictions, [categoryId]: nomineeId };
    if (selectedCeremony) await loadResults(selectedCeremony.id);
    await loadCeremonies();
  } catch (e) {
    logger.error("[awards] set prediction failed", e);
    showError("[awards] set prediction failed" + ": " + String(e));
  }
}

export async function clearPrediction(categoryId: number) {
  const request = detailRequest;
  try {
    await invoke("clear_award_prediction", { categoryId });
    if (request !== detailRequest) return;
    const next = { ...predictions };
    delete next[categoryId];
    predictions = next;
    if (selectedCeremony) await loadResults(selectedCeremony.id);
    await loadCeremonies();
  } catch (e) {
    logger.error("[awards] clear prediction failed", e);
    showError("[awards] clear prediction failed" + ": " + String(e));
  }
}

/** Pull fresh data from Wikipedia. `full` re-pulls 20 years; otherwise incremental. */
export async function refreshAwards(full = false, retryFailed = false): Promise<SyncSummary> {
  syncing = true;
  try {
    const summary = await invoke<SyncSummary>("sync_awards", { full, retryFailed });
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
