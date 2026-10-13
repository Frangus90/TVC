import { invoke } from "@tauri-apps/api/core";
import type { BackupData } from "../utils/backup";

export type BackupFrequency = "off" | "daily" | "weekly" | "monthly";
export interface BackupPreferences {
  frequency: BackupFrequency;
  folder: string | null;
  retention: number;
  include_settings: boolean;
}
export interface BackupStatus {
  preferences: BackupPreferences;
  running: boolean;
  last_success_at: string | null;
  last_success_path: string | null;
  last_attempt_at: string | null;
  last_error: string | null;
  next_due_at: string | null;
}
export interface BackupWriteResult {
  path: string;
  created_at: string;
  version: string;
  warning: string | null;
}
export interface BackupImportResult {
  shows_imported: number;
  episodes_imported: number;
  movies_imported: number;
  predictions_imported: number;
  quarantined: number;
  episodes_orphaned: number;
  warnings: string[];
  settings_restored: boolean;
  restart_required: boolean;
}
let status = $state<BackupStatus | null>(null);
let loading = $state(false);
let saving = $state(false);
let error = $state<string | null>(null);
let request: Promise<boolean> | null = null;
let queue: Promise<unknown> = Promise.resolve();
let revision = 0;
let restoring = $state(false);
let restartRequired = $state(false);

export function getBackupStatus() { return status; }
export function isBackupLoading() { return loading; }
export function isBackupSaving() { return saving; }
export function getBackupError() { return error; }
export function isBackupRestoring() { return restoring; }
export function isBackupRestartRequired() { return restartRequired; }

export async function restoreBackup(data: BackupData, prepare: () => Promise<void>): Promise<BackupImportResult> {
  if (restoring || restartRequired) throw new Error("Finish the current restore by restarting TVC.");
  restoring = true;
  try {
    await prepare();
    const result = await invoke<BackupImportResult>("import_database", { data });
    restartRequired = true;
    return result;
  } finally { restoring = false; }
}

export function loadBackupStatus(): Promise<boolean> {
  if (request) return request;
  loading = true;
  const expected = revision;
  request = invoke<BackupStatus>("get_backup_status")
    .then(saved => { if (expected === revision) { status = saved; error = null; } return expected === revision; })
    .catch(cause => { if (expected === revision) error = `Could not load backup status: ${String(cause)}`; return false; })
    .finally(() => { loading = false; request = null; });
  return request;
}

export function updateBackupPreferences(updates: Partial<BackupPreferences>): Promise<void> {
  const operation = queue.then(async () => {
    saving = true;
    try {
      if (!status && !await loadBackupStatus()) throw new Error(error ?? "Backup settings are unavailable");
      revision++;
      const saved = await invoke<BackupStatus>("update_backup_preferences", { preferences: { ...status!.preferences, ...updates } });
      revision++;
      status = saved;
      error = null;
    } catch (cause) { revision++; error = `Backup settings were not saved: ${String(cause)}`; throw cause; }
    finally { saving = false; }
  });
  queue = operation.catch(() => {});
  return operation;
}

async function create(command: string, args?: Record<string, unknown>): Promise<BackupWriteResult> {
  try {
    const result = await invoke<BackupWriteResult>(command, args);
    // Finish any earlier poll so it cannot overwrite the post-operation status.
    await request;
    await loadBackupStatus();
    return result;
  } catch (cause) {
    await request;
    await loadBackupStatus();
    throw cause;
  }
}
export function createBackupFile(path: string, includeSettings: boolean) {
  return create("create_backup_file", { path, includeSettings });
}
export function createLocalBackup() { return create("create_local_backup"); }
