import { invoke } from "@tauri-apps/api/core";
import { logger } from "../utils/logger";

// Types
export interface PlexConfig {
  enabled: boolean;
  port: number;
}

export interface PlexServerStatus {
  running: boolean;
  port: number | null;
}

export interface ScrobbleLogEntry {
  id: number;
  event_type: string;
  media_type: string;
  raw_title: string;
  show_name: string | null;
  season_number: number | null;
  episode_number: number | null;
  year: number | null;
  matched_entity_type: string | null;
  matched_entity_id: number | null;
  match_method: string | null;
  scrobbled_at: string;
}

// State
let modalOpen = $state(false);
let loading = $state(false);
let error = $state<string | null>(null);
let config = $state<PlexConfig>({ enabled: false, port: 9876 });
let serverStatus = $state<PlexServerStatus>({ running: false, port: null });
let scrobbleLog = $state<ScrobbleLogEntry[]>([]);

// Getters
export function isModalOpen() {
  return modalOpen;
}

export function isLoading() {
  return loading;
}

export function getError() {
  return error;
}

export function getConfig() {
  return config;
}

export function getServerStatus() {
  return serverStatus;
}

export function getScrobbleLog() {
  return scrobbleLog;
}

// Actions
export function openPlexSettings() {
  modalOpen = true;
  error = null;
  loadConfig();
  loadServerStatus();
  loadScrobbleLog();
}

export function closePlexSettings() {
  modalOpen = false;
  error = null;
}

// Data loading
export async function loadConfig() {
  loading = true;
  error = null;

  try {
    config = await invoke<PlexConfig>("get_plex_config");
  } catch (err) {
    logger.error("[Plex] Failed to load config:", err);
    error = err instanceof Error ? err.message : String(err);
  } finally {
    loading = false;
  }
}

export async function loadServerStatus() {
  try {
    serverStatus = await invoke<PlexServerStatus>("get_plex_server_status");
  } catch (err) {
    logger.error("[Plex] Failed to load server status:", err);
  }
}

export async function loadScrobbleLog() {
  try {
    scrobbleLog = await invoke<ScrobbleLogEntry[]>("get_scrobble_log", { limit: 20 });
  } catch (err) {
    logger.error("[Plex] Failed to load scrobble log:", err);
  }
}

export async function updateConfig(newConfig: PlexConfig) {
  loading = true;
  error = null;

  try {
    await invoke("update_plex_config", { config: newConfig });
    config = newConfig;
  } catch (err) {
    logger.error("[Plex] Failed to update config:", err);
    error = err instanceof Error ? err.message : String(err);
    throw err;
  } finally {
    loading = false;
  }
}

export async function startServer() {
  loading = true;
  error = null;

  try {
    await invoke("start_plex_server", { port: config.port });
    await loadServerStatus();
  } catch (err) {
    logger.error("[Plex] Failed to start server:", err);
    error = err instanceof Error ? err.message : String(err);
  } finally {
    loading = false;
  }
}

export async function stopServer() {
  loading = true;
  error = null;

  try {
    await invoke("stop_plex_server");
    await loadServerStatus();
  } catch (err) {
    logger.error("[Plex] Failed to stop server:", err);
    error = err instanceof Error ? err.message : String(err);
  } finally {
    loading = false;
  }
}

export async function toggleServer(enabled: boolean) {
  if (enabled) {
    await startServer();
  } else {
    await stopServer();
  }

  // Update config to persist the enabled state
  await updateConfig({ ...config, enabled });
}

// Computed
export function getWebhookUrl() {
  return `http://localhost:${config.port}/webhook/plex`;
}
