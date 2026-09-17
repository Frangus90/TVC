import { formatDateKey, localDayRange } from "../utils/dateFormat";
import { getDatabase } from "../utils/database";
let rangeRequest = 0;
import { showError } from "./toast.svelte";
import { invoke } from "@tauri-apps/api/core";
import { logger } from "../utils/logger";

export interface RacingSeries {
  id: number;
  slug: string;
  name: string;
  category: string;
  ics_url: string;
  fallback_ics_url: string | null;
  custom_ics_url: string | null;
  enabled: boolean;
  notify_enabled: boolean;
  notify_minutes: number;
  color: string;
  custom_color: string | null;
}

export interface RacingEvent {
  id: number;
  series_slug: string;
  uid: string;
  event_title: string;
  session_name: string | null;
  circuit: string | null;
  start_time: string;
  end_time: string | null;
  description: string | null;
  notified: boolean;
}

export interface RacingConfig {
  notifications_enabled: boolean;
  default_notify_minutes: number;
  last_refreshed: string | null;
}

// State
let racingSeries = $state<RacingSeries[]>([]);
let racingEvents = $state<RacingEvent[]>([]);
let racingConfig = $state<RacingConfig | null>(null);
let isLoading = $state(false);
let isRefreshing = $state(false);
let currentRacingRange = $state<{ start: string; end: string } | null>(null);
let refreshSignal = $state(0);
export interface RacingRefreshReport { attempted_at: string; events: number; succeeded: number; failures: { slug: string; name: string; error: string }[] }
let refreshReport = $state<RacingRefreshReport | null>(null);
export function getRacingRefreshReport() { return refreshReport; }

// Getters
export function getRacingSeries(): RacingSeries[] {
  return racingSeries;
}

export function getEnabledSeries(): RacingSeries[] {
  return racingSeries.filter((s) => s.enabled);
}

export function getRacingEvents(): RacingEvent[] {
  return racingEvents;
}

export function getRacingConfig(): RacingConfig | null {
  return racingConfig;
}

export function isRacingLoading(): boolean {
  return isLoading;
}

export function isRacingRefreshing(): boolean {
  return isRefreshing;
}


// Signal that increments after data changes, so dependent effects re-fire
export function getRefreshSignal(): number {
  return refreshSignal;
}

// Get the effective color for a series (custom override or default)
export function getSeriesColor(series: RacingSeries): string {
  return series.custom_color || series.color;
}

// Get events for a specific date
export function getRacingEventsForDate(date: string): RacingEvent[] {
  return racingEvents.filter((ev) => {
    // Compare just the date portion of start_time
    return formatDateKey(ev.start_time) === date;
  });
}

// Get the series object by slug
export function getSeriesBySlug(slug: string): RacingSeries | undefined {
  return racingSeries.find((s) => s.slug === slug);
}

// Actions
export async function loadRacingSeries(): Promise<void> {
  try {
    const series = await invoke<RacingSeries[]>("get_racing_series");
    racingSeries = series;
  } catch (error) {
    logger.error("Failed to load racing series", error);
    showError("Failed to load racing series" + ": " + String(error));
  }
}

export async function loadRacingConfig(): Promise<void> {
  try {
    const config = await invoke<RacingConfig>("get_racing_config");
    racingConfig = config;
    const db = await getDatabase();
    const rows = await db.select<{ value: string }[]>("SELECT value FROM settings WHERE key = 'racing_refresh_report'");
    refreshReport = rows[0] ? JSON.parse(rows[0].value) : null;
  } catch (error) {
    logger.error("Failed to load racing config", error);
    showError("Failed to load racing config" + ": " + String(error));
  }
}

export async function loadRacingEventsForRange(
  start: string,
  end: string
): Promise<void> {
  const request = ++rangeRequest;
  currentRacingRange = { start, end };
  isLoading = true;
  try {
    const events = await invoke<RacingEvent[]>("get_racing_events_for_range", {
      ...localDayRange(start, end),
    });
    if (request === rangeRequest) racingEvents = events;
  } catch (error) {
    if (request !== rangeRequest) return;
    logger.error("Failed to load racing events", error);
    showError("Failed to load racing events" + ": " + String(error));
  } finally {
    if (request === rangeRequest) isLoading = false;
  }
}

export async function toggleSeries(
  slug: string,
  enabled: boolean
): Promise<void> {
  try {
    await invoke("toggle_racing_series", { slug, enabled });
    racingSeries = racingSeries.map((s) =>
      s.slug === slug ? { ...s, enabled } : s
    );
    refreshSignal++;
  } catch (error) {
    logger.error("Failed to toggle racing series", error);
    showError("Failed to toggle racing series" + ": " + String(error));
  }
}

export async function updateSeriesColor(
  slug: string,
  color: string | null
): Promise<void> {
  try {
    await invoke("update_racing_series_color", { slug, color });
    racingSeries = racingSeries.map((s) =>
      s.slug === slug ? { ...s, custom_color: color } : s
    );
  } catch (error) {
    logger.error("Failed to update series color", error);
    showError("Failed to update series color" + ": " + String(error));
  }
}

export async function updateSeriesNotification(
  slug: string,
  notifyEnabled: boolean,
  notifyMinutes: number
): Promise<void> {
  try {
    await invoke("update_racing_series_notification", {
      slug,
      notifyEnabled,
      notifyMinutes,
    });
    racingSeries = racingSeries.map((s) =>
      s.slug === slug
        ? { ...s, notify_enabled: notifyEnabled, notify_minutes: notifyMinutes }
        : s
    );
  } catch (error) {
    logger.error("Failed to update series notification", error);
    showError("Failed to update series notification" + ": " + String(error));
  }
}

export async function updateSeriesIcsUrl(
  slug: string,
  customUrl: string | null
): Promise<void> {
  try {
    await invoke("update_racing_series_ics_url", { slug, customUrl });
    racingSeries = racingSeries.map((s) =>
      s.slug === slug ? { ...s, custom_ics_url: customUrl } : s
    );
  } catch (error) {
    logger.error("Failed to update series ICS URL", error);
    showError("Failed to update series ICS URL" + ": " + String(error));
  }
}

export async function refreshRacingData(): Promise<void> {
  isRefreshing = true;
  try {
    refreshReport = await invoke<RacingRefreshReport>("refresh_racing_data");
    // Reload events for current range
    if (currentRacingRange) {
      await loadRacingEventsForRange(
        currentRacingRange.start,
        currentRacingRange.end
      );
    }
    await loadRacingConfig();
    refreshSignal++;
  } catch (error) {
    logger.error("Failed to refresh racing data", error);
    showError("Failed to refresh racing data" + ": " + String(error));
  } finally {
    isRefreshing = false;
  }
}

export async function refreshSingleSeries(slug: string): Promise<void> {
  try {
    await invoke("refresh_single_racing_series", { slug });
    if (refreshReport) {
      refreshReport = { ...refreshReport, failures: refreshReport.failures.filter(f => f.slug !== slug) };
      const db = await getDatabase();
      await db.execute("INSERT OR REPLACE INTO settings (key, value) VALUES ('racing_refresh_report', $1)", [JSON.stringify(refreshReport)]);
    }
    if (currentRacingRange) {
      await loadRacingEventsForRange(
        currentRacingRange.start,
        currentRacingRange.end
      );
    }
    refreshSignal++;
  } catch (error) {
    logger.error("Failed to refresh series", error);
    showError("Failed to refresh series" + ": " + String(error));
  }
}

export async function updateRacingConfig(
  notificationsEnabled: boolean
): Promise<void> {
  try {
    await invoke("update_racing_config", {
      notificationsEnabled,
      defaultNotifyMinutes: racingConfig?.default_notify_minutes ?? 30,
    });
    racingConfig = {
      notifications_enabled: notificationsEnabled,
      default_notify_minutes: racingConfig?.default_notify_minutes ?? 30,
      last_refreshed: racingConfig?.last_refreshed ?? null,
    };
  } catch (error) {
    await loadRacingConfig();
    logger.error("Failed to update racing config", error);
    showError("Failed to update racing config" + ": " + String(error));
  }
}

export async function updateAllSeriesLeadTime(
  minutes: number
): Promise<void> {
  const enabledSeries = racingSeries.filter(
    (s) => s.enabled && s.notify_enabled
  );
  for (const series of enabledSeries) {
    await updateSeriesNotification(series.slug, true, minutes);
  }
}

// Dev: fire a test notification to verify the system works
export async function testNotification(): Promise<void> {
  try {
    await invoke("test_racing_notification");
  } catch (error) {
    logger.error("Failed to send test notification", error);
    showError("Failed to send test notification" + ": " + String(error));
  }
}

// Refresh racing calendar (called when range changes)
export async function refreshRacingCalendar(): Promise<void> {
  if (currentRacingRange) {
    await loadRacingEventsForRange(
      currentRacingRange.start,
      currentRacingRange.end
    );
  }
}
