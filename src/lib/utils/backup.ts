// The backend validates individual records and relationships before any writes.
// These checks let the confirmation describe the file's actual backup scope.
export interface BackupData {
  version: string;
  exported_at: string;
  shows: unknown[];
  episodes: unknown[];
  movies: unknown[];
  tiers: unknown[];
  personal_data?: {
    ceremonies: unknown[];
    categories: unknown[];
    nominees: unknown[];
    predictions: unknown[];
    title_mappings: unknown[];
    scrobble_history: unknown[];
    change_history: unknown[];
  } | null;
  portable_settings?: PortableSettings | null;
}

export interface PortableSettings {
  appearance: Record<string, unknown>;
  general: Record<string, unknown>;
  notifications: Record<string, unknown>;
  racing: { notifications_enabled: boolean; default_notify_minutes: number; series: unknown[] };
  tier_preset: string;
}

function exactKeys(value: unknown, keys: string[]): value is Record<string, unknown> {
  return !!value && typeof value === "object" && !Array.isArray(value)
    && Object.keys(value).length === keys.length && keys.every(key => Object.prototype.hasOwnProperty.call(value, key));
}

function validatePortable(value: unknown): asserts value is PortableSettings {
  const generalKeys = ["startup_page", "remember_calendar", "close_action", "start_minimized", "release_country", "last_page", "calendar_view", "calendar_filter"];
  if (!exactKeys(value, ["appearance", "general", "notifications", "racing", "tier_preset"])
    || !exactKeys(value.appearance, ["accentColor", "fontSize", "compactSpacing", "colorblindFriendly", "hidePosters", "posterSize"])
    || !(exactKeys(value.general, generalKeys) || exactKeys(value.general, [...generalKeys, "default_tier_list"]))
    || (Object.prototype.hasOwnProperty.call(value.general, "default_tier_list") && !["shows", "movies"].includes(value.general.default_tier_list as string))
    || !exactKeys(value.notifications, ["enabled", "sound_enabled", "sound_volume", "sound_choice", "popup_position", "popup_duration", "max_visible", "os_fallback", "tray_notifications", "racing_enabled", "plex_enabled", "premiere_enabled", "update_enabled", "system_enabled", "quiet_hours_enabled", "quiet_hours_start", "quiet_hours_end"])
    || !exactKeys(value.racing, ["notifications_enabled", "default_notify_minutes", "series"])
    || !Array.isArray(value.racing.series)
    || value.racing.series.some(series => !exactKeys(series, ["slug", "enabled", "notify_enabled", "notify_minutes", "custom_color"]))) {
    throw new Error("Backup v4.0 contains missing or unsupported portable preferences");
  }
  // Native validation checks each preference's value before the restore transaction.
}

export function parseBackup(content: string): BackupData {
  const data = JSON.parse(content);
  if (!data || typeof data !== "object" || Array.isArray(data)) {
    throw new Error("Invalid backup file format");
  }
  const version = data.version === undefined ? "1.0" : data.version;
  if (!["1.0", "2.0", "3.0", "4.0"].includes(version)) {
    throw new Error(`Unsupported backup version: ${String(version)}`);
  }
  if (typeof data.exported_at !== "string"
    || ![data.shows, data.episodes, data.movies].every(Array.isArray)
    || (data.tiers != null && !Array.isArray(data.tiers))) {
    throw new Error("Invalid backup file format");
  }
  if (version === "3.0" || version === "4.0") {
    const personal = data.personal_data;
    if (!Array.isArray(data.tiers) || !personal || ![
      personal.ceremonies, personal.categories, personal.nominees,
      personal.predictions, personal.title_mappings,
      personal.scrobble_history, personal.change_history,
    ].every(Array.isArray)) {
      throw new Error(`Backup v${version} is missing required tiers, predictions, or history data`);
    }
  } else if (data.personal_data != null) {
    throw new Error("Personal data requires backup version 3.0");
  }
  if (version === "4.0") validatePortable(data.portable_settings);
  else if (data.portable_settings != null) throw new Error("Portable settings require backup version 4.0");
  return { ...data, version, tiers: data.tiers ?? [] };
}

export function describeBackup(data: BackupData): string {
  const library = `Replace your library with ${data.shows.length} shows, ${data.episodes.length} episodes, ${data.movies.length} movies, and ${data.tiers.length} tiers.`;
  const personal = data.personal_data;
  const history = personal
    ? `Also replace awards with ${personal.ceremonies.length} ceremonies and ${personal.predictions.length} saved predictions, plus ${personal.title_mappings.length} Plex title corrections, ${personal.scrobble_history.length} scrobbles, and ${personal.change_history.length} change-history entries.`
    : "This older backup contains no predictions or history. Existing awards and predictions will be kept. Plex title corrections, scrobble logs, and change history will be cleared because the file cannot restore them. Export a new backup first if you need to keep them.";
  const settings = data.portable_settings
    ? `Also replace Appearance, General and calendar preferences, Notifications (including quiet hours), the tier preset, and Racing series/reminder preferences (${data.portable_settings.racing.series.length} supplied series; series absent from the backup are disabled). Credentials, Windows startup registration, backup folders and schedules, sync schedules, Plex/Sonarr/Radarr connections, custom Racing feed URLs, and downloaded Racing events stay unchanged.`
    : "Settings, credentials, and racing data stay unchanged.";
  return `${library}\n\n${history}\n\n${settings} Cached cast/crew and Sonarr/Radarr import links are cleared. Restart TVC after importing.`;
}
