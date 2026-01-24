import { invoke } from "@tauri-apps/api/core";
import { logger } from "../utils/logger";
import { loadTrackedShows } from "./shows.svelte";
import { loadTrackedMovies } from "./movies.svelte";

// Types
export interface ArrServer {
  id: number;
  name: string;
  type: "sonarr" | "radarr";
  base_url: string;
  api_key: string;
  is_active: boolean;
  auto_sync_enabled: boolean;
  sync_interval_hours: number;
  last_synced: string | null;
  added_at: string | null;
}

export interface ArrServerRequest {
  name: string;
  type: "sonarr" | "radarr";
  base_url: string;
  api_key: string;
}

export interface ArrSystemStatus {
  version: string;
  app_name: string | null;
}

export interface LibraryItem {
  arr_id: number;
  title: string;
  year: number | null;
  poster_url: string | null;
  monitored: boolean;
  tvdb_id: number | null;
  tmdb_id: number | null;
  already_tracked: boolean;
}

export interface ImportItem {
  arr_id: number;
  tvdb_id: number | null;
  tmdb_id: number | null;
}

export interface ImportRequest {
  server_id: number;
  items: ImportItem[];
}

export interface ImportResult {
  total: number;
  imported: number;
  skipped: number;
  failed: number;
  errors: string[];
}

// State
let modalOpen = $state(false);
let activeTab = $state<"servers" | "import">("servers");
let loading = $state(false);
let error = $state<string | null>(null);
let successMessage = $state<string | null>(null);

// Server state
let servers = $state<ArrServer[]>([]);
let editingServer = $state<ArrServer | null>(null);
let testingConnection = $state(false);
let testResult = $state<{ success: boolean; message: string } | null>(null);

// Import state
let selectedServer = $state<ArrServer | null>(null);
let libraryItems = $state<LibraryItem[]>([]);
let selectedItems = $state<Set<number>>(new Set());
let importing = $state(false);
let importResult = $state<ImportResult | null>(null);

// Getters
export function isModalOpen() {
  return modalOpen;
}

export function getActiveTab() {
  return activeTab;
}

export function isLoading() {
  return loading;
}

export function getError() {
  return error;
}

export function getSuccessMessage() {
  return successMessage;
}

export function getServers() {
  return servers;
}

export function getEditingServer() {
  return editingServer;
}

export function isTestingConnection() {
  return testingConnection;
}

export function getTestResult() {
  return testResult;
}

export function getSelectedServer() {
  return selectedServer;
}

export function getLibraryItems() {
  return libraryItems;
}

export function getSelectedItems() {
  return selectedItems;
}

export function isImporting() {
  return importing;
}

export function getImportResult() {
  return importResult;
}

// Actions
export function openArrSettings() {
  modalOpen = true;
  activeTab = "servers";
  error = null;
  successMessage = null;
  loadServers();
}

export function closeArrSettings() {
  modalOpen = false;
  error = null;
  successMessage = null;
  editingServer = null;
  testResult = null;
  importResult = null;
  selectedServer = null;
  libraryItems = [];
  selectedItems = new Set();
}

export function setActiveTab(tab: "servers" | "import") {
  activeTab = tab;
  error = null;
  successMessage = null;

  if (tab === "servers") {
    loadServers();
  } else if (tab === "import") {
    // Reset import state and reload servers to ensure fresh data
    selectedServer = null;
    libraryItems = [];
    selectedItems = new Set();
    importResult = null;
    loadServers();
  }
}

export function setEditingServer(server: ArrServer | null) {
  editingServer = server;
  testResult = null;
}

export function clearTestResult() {
  testResult = null;
}

export function setSelectedServer(server: ArrServer | null) {
  selectedServer = server;
  libraryItems = [];
  selectedItems = new Set();
  importResult = null;

  if (server) {
    loadLibrary(server);
  }
}

export function toggleItemSelection(arrId: number) {
  const newSet = new Set(selectedItems);
  if (newSet.has(arrId)) {
    newSet.delete(arrId);
  } else {
    newSet.add(arrId);
  }
  selectedItems = newSet;
}

export function selectAllItems() {
  const newSet = new Set<number>();
  for (const item of libraryItems) {
    if (!item.already_tracked) {
      newSet.add(item.arr_id);
    }
  }
  selectedItems = newSet;
}

export function deselectAllItems() {
  selectedItems = new Set();
}

// Data loading
export async function loadServers() {
  loading = true;
  error = null;

  try {
    const result = await invoke<ArrServer[]>("get_arr_servers");
    logger.debug("[Arr] Loaded servers:", result);
    servers = result;
  } catch (err) {
    logger.error("[Arr] Failed to load servers:", err);
    error = err instanceof Error ? err.message : String(err);
  } finally {
    loading = false;
  }
}

export async function testConnection(baseUrl: string, apiKey: string, serverType: string): Promise<void> {
  testingConnection = true;
  testResult = null;

  try {
    const status = await invoke<ArrSystemStatus>("test_arr_server", {
      baseUrl,
      apiKey,
      serverType,
    });
    testResult = {
      success: true,
      message: `Connected successfully! ${serverType.charAt(0).toUpperCase() + serverType.slice(1)} v${status.version}`,
    };
  } catch (err) {
    logger.error("Connection test failed:", err);
    testResult = {
      success: false,
      message: err instanceof Error ? err.message : String(err),
    };
  } finally {
    testingConnection = false;
  }
}

export async function addServer(request: ArrServerRequest): Promise<void> {
  loading = true;
  error = null;

  try {
    logger.debug("[Arr] Adding server:", request);
    const id = await invoke<number>("add_arr_server", { server: request });
    logger.debug("[Arr] Server added with id:", id);
    successMessage = `${request.name} added successfully`;
    await loadServers();
    editingServer = null;
    setTimeout(() => (successMessage = null), 3000);
  } catch (err) {
    logger.error("Failed to add server:", err);
    error = err instanceof Error ? err.message : String(err);
    throw err;
  } finally {
    loading = false;
  }
}

export async function updateServer(id: number, request: ArrServerRequest): Promise<void> {
  loading = true;
  error = null;

  try {
    await invoke("update_arr_server", { id, server: request });
    successMessage = `${request.name} updated successfully`;
    await loadServers();
    editingServer = null;
    setTimeout(() => (successMessage = null), 3000);
  } catch (err) {
    logger.error("Failed to update server:", err);
    error = err instanceof Error ? err.message : String(err);
    throw err;
  } finally {
    loading = false;
  }
}

export async function deleteServer(id: number): Promise<void> {
  loading = true;
  error = null;

  try {
    await invoke("delete_arr_server", { id });
    successMessage = "Server deleted";
    await loadServers();
    setTimeout(() => (successMessage = null), 3000);
  } catch (err) {
    logger.error("Failed to delete server:", err);
    error = err instanceof Error ? err.message : String(err);
    throw err;
  } finally {
    loading = false;
  }
}

export async function loadLibrary(server: ArrServer): Promise<void> {
  loading = true;
  error = null;
  libraryItems = [];

  try {
    if (server.type === "sonarr") {
      libraryItems = await invoke<LibraryItem[]>("get_sonarr_library", { serverId: server.id });
    } else {
      libraryItems = await invoke<LibraryItem[]>("get_radarr_library", { serverId: server.id });
    }
  } catch (err) {
    logger.error("Failed to load library:", err);
    error = err instanceof Error ? err.message : String(err);
  } finally {
    loading = false;
  }
}

export async function importSelected(): Promise<void> {
  if (!selectedServer || selectedItems.size === 0) return;

  importing = true;
  error = null;
  importResult = null;

  try {
    const items: ImportItem[] = [];
    for (const arrId of selectedItems) {
      const item = libraryItems.find((i) => i.arr_id === arrId);
      if (item) {
        items.push({
          arr_id: item.arr_id,
          tvdb_id: item.tvdb_id,
          tmdb_id: item.tmdb_id,
        });
      }
    }

    const request: ImportRequest = {
      server_id: selectedServer.id,
      items,
    };

    if (selectedServer.type === "sonarr") {
      importResult = await invoke<ImportResult>("import_from_sonarr", { request });
      // Refresh sidebar shows
      await loadTrackedShows();
    } else {
      importResult = await invoke<ImportResult>("import_from_radarr", { request });
      // Refresh sidebar movies
      await loadTrackedMovies();
    }

    // Clear selection and refresh library
    selectedItems = new Set();
    await loadLibrary(selectedServer);
  } catch (err) {
    logger.error("Import failed:", err);
    error = err instanceof Error ? err.message : String(err);
  } finally {
    importing = false;
  }
}

// Helpers
export function getSonarrServers(): ArrServer[] {
  return servers.filter((s) => s.type === "sonarr");
}

export function getRadarrServers(): ArrServer[] {
  return servers.filter((s) => s.type === "radarr");
}

export function getUnselectedItemsCount(): number {
  return libraryItems.filter((i) => !i.already_tracked && !selectedItems.has(i.arr_id)).length;
}

export function getSelectedItemsCount(): number {
  return selectedItems.size;
}
