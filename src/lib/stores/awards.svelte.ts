import { invoke } from "@tauri-apps/api/core";
import { logger } from "../utils/logger";

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

let awardType = $state<AwardType>("oscars");
let ceremonies = $state<CeremonySummary[]>([]);
let selectedCeremony = $state<CeremonyDetail | null>(null);
let loading = $state(false);
let syncing = $state(false);

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
  } catch (e) {
    logger.error("[awards] load ceremony detail failed", e);
  } finally {
    loading = false;
  }
}

export function clearSelectedCeremony() {
  selectedCeremony = null;
}

/** Pull fresh data from Wikipedia. `full` re-pulls 20 years; otherwise incremental. */
export async function refreshAwards(full = false): Promise<SyncSummary> {
  syncing = true;
  try {
    const summary = await invoke<SyncSummary>("sync_awards", { full });
    await loadCeremonies();
    if (selectedCeremony) {
      await selectCeremony(selectedCeremony.id);
    }
    return summary;
  } finally {
    syncing = false;
  }
}
