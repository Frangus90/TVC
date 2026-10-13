import { invoke } from "@tauri-apps/api/core";
import type { NavId } from "../config/navItems";
import { logger } from "../utils/logger";
import { showError } from "./toast.svelte";

export type StartupPage = "calendar" | "movies" | "shows" | "last_used";
export type CalendarView = "month" | "week" | "agenda";
export type CalendarFilter = "all" | "shows" | "movies";

export interface GeneralPreferences {
  startup_page: StartupPage;
  remember_calendar: boolean;
  close_action: "tray" | "quit";
  start_minimized: boolean;
  release_country: string;
  last_page: NavId;
  calendar_view: CalendarView;
  calendar_filter: CalendarFilter;
}

interface WindowsStartup {
  enabled: boolean;
  supported: boolean;
}

let preferences = $state<GeneralPreferences>({
  startup_page: "calendar",
  remember_calendar: false,
  close_action: "tray",
  start_minimized: false,
  release_country: "US",
  last_page: "calendar",
  calendar_view: "month",
  calendar_filter: "all",
});
let loaded = $state(false);
let startup = $state<WindowsStartup>({ enabled: false, supported: false });
let loadRequest: Promise<void> | null = null;
let saveQueue: Promise<void> = Promise.resolve();
let requestedPage: NavId | null = null;
let requestedCalendar: string | null = null;

export function getGeneralPreferences() {
  return preferences;
}

export function waitForGeneralPreferenceSaves(): Promise<void> { return saveQueue; }

export function isGeneralLoaded() {
  return loaded;
}

export function getWindowsStartup() {
  return startup;
}

export async function loadGeneralPreferences(): Promise<void> {
  if (loaded) return;
  if (loadRequest) return loadRequest;
  loadRequest = invoke<GeneralPreferences>("get_general_preferences")
    .then((saved) => {
      preferences = saved;
      loaded = true;
    })
    .finally(() => { loadRequest = null; });
  return loadRequest;
}

export function updateGeneralPreferences(updates: Partial<GeneralPreferences>): Promise<void> {
  // Navigation, calendar and Settings all share this queue. Merge when a write
  // starts so a slow earlier save cannot undo a newer, unrelated preference.
  const save = saveQueue.then(async () => {
    await loadGeneralPreferences();
    preferences = await invoke<GeneralPreferences>("update_general_preferences", {
      preferences: { ...preferences, ...updates },
    });
  });
  // The caller receives the rejection; recover only the queue for its next save.
  saveQueue = save.catch(() => {});
  return save;
}

export async function loadWindowsStartup(): Promise<void> {
  startup = await invoke<WindowsStartup>("get_start_with_windows");
}

export async function setWindowsStartup(enabled: boolean): Promise<void> {
  startup = await invoke<WindowsStartup>("set_start_with_windows", { enabled });
}

export function getStartupState(): { page: NavId; viewMode: CalendarView; filter: CalendarFilter } {
  return {
    page: preferences.startup_page === "last_used" ? preferences.last_page : preferences.startup_page,
    viewMode: preferences.remember_calendar ? preferences.calendar_view : "month",
    filter: preferences.remember_calendar ? preferences.calendar_filter : "all",
  };
}

export async function rememberLastPage(page: NavId): Promise<void> {
  if (!loaded || requestedPage === page) return;
  requestedPage = page;
  try {
    await updateGeneralPreferences({ last_page: page });
  } catch (error) {
    if (requestedPage === page) requestedPage = null;
    logger.error("Could not remember the last page", error);
    showError("Could not remember the last page. Your saved startup setting is unchanged.");
  }
}

export async function rememberCalendarState(viewMode: CalendarView, filter: CalendarFilter): Promise<void> {
  if (!loaded || !preferences.remember_calendar) return;
  const requested = `${viewMode}/${filter}`;
  if (requestedCalendar === requested) return;
  requestedCalendar = requested;
  try {
    await updateGeneralPreferences({ calendar_view: viewMode, calendar_filter: filter });
  } catch (error) {
    if (requestedCalendar === requested) requestedCalendar = null;
    logger.error("Could not remember the calendar view", error);
    showError("Could not remember the calendar view and filters.");
  }
}
