let libraryRequest = 0;
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
  status: string | null;
  monitored: boolean;
  tvdb_id: number | null;
  tmdb_id: number | null;
  already_tracked: boolean;
}

export interface SonarrImportFilters {
  monitored: boolean;
  continuing: boolean;
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
let sonarrFilters = $state<SonarrImportFilters>({
  monitored: false,
  continuing: false,
});
let importing = $state(false);
let importResult = $state<ImportResult | null>(null);

// Getters
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

export function getFilteredLibraryItems() {
  if (!selectedServer || selectedServer.type !== "sonarr") {
    return libraryItems;
  }

  return libraryItems.filter((item) => {
    if (sonarrFilters.monitored && !item.monitored) {
      return false;
    }

    if (sonarrFilters.continuing && item.status?.toLowerCase() !== "continuing") {
      return false;
    }

    return true;
  });
}

export function getSonarrFilters() {
  return sonarrFilters;
}

export function getActiveSonarrFilterCount() {
  return Number(sonarrFilters.monitored) + Number(sonarrFilters.continuing);
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
export function initArrTab() {
  activeTab = "servers";
  error = null;
  successMessage = null;
  editingServer = null;
  testResult = null;
  importResult = null;
  selectedServer = null;
  libraryItems = [];
  selectedItems = new Set();
  loadServers();
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

function pruneSelectionToFilteredItems() {
  const visibleIds = new Set(getFilteredLibraryItems().map((item) => item.arr_id));
  selectedItems = new Set([...selectedItems].filter((arrId) => visibleIds.has(arrId)));
}

export function setSonarrFilter(
  filter: keyof SonarrImportFilters,
  enabled: boolean
) {
  sonarrFilters = {
    ...sonarrFilters,
    [filter]: enabled,
  };
  pruneSelectionToFilteredItems();
}

export function clearSonarrFilters() {
  sonarrFilters = {
    monitored: false,
    continuing: false,
  };
  pruneSelectionToFilteredItems();
}

export function setSelectedServer(server: ArrServer | null) {
  libraryRequest++;
  loading = false;
  selectedServer = server;
  libraryItems = [];
  selectedItems = new Set();
  sonarrFilters = {
    monitored: false,
    continuing: false,
  };
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
  for (const item of getFilteredLibraryItems()) {
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
    logger.debug("[Arr] Loaded server count:", result.length);
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
    logger.debug("[Arr] Adding server:", { name: request.name, type: request.type });
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
  const request = ++libraryRequest;
  loading = true; error = null; libraryItems = [];
  try {
    const items = await invoke<LibraryItem[]>(server.type === "sonarr" ? "get_sonarr_library" : "get_radarr_library", { serverId: server.id });
    if (request === libraryRequest) libraryItems = items;
  } catch (err) {
    if (request === libraryRequest) error = String(err);
  } finally {
    if (request === libraryRequest) loading = false;
  }
}

export async function importSelected(): Promise<void> {
  if (!selectedServer || selectedItems.size === 0) return;

  const server = selectedServer;
  const requestGeneration = libraryRequest;
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
      server_id: server.id,
      items,
    };

    if (server.type === "sonarr") {
      const result = await invoke<ImportResult>("import_from_sonarr", { request });
      if (requestGeneration === libraryRequest) importResult = result;
      // Refresh sidebar shows
      await loadTrackedShows();
    } else {
      const result = await invoke<ImportResult>("import_from_radarr", { request });
      if (requestGeneration === libraryRequest) importResult = result;
      // Refresh sidebar movies
      await loadTrackedMovies();
    }

    // Clear selection and refresh library
    if (requestGeneration === libraryRequest) {
      selectedItems = new Set();
      await loadLibrary(server);
    }
  } catch (err) {
    logger.error("Import failed:", err);
    if (requestGeneration !== libraryRequest) return;
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
  return getFilteredLibraryItems().filter((i) => !i.already_tracked && !selectedItems.has(i.arr_id)).length;
}

export function getSelectedItemsCount(): number {
  return selectedItems.size;
}
