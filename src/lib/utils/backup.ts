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
}

export function parseBackup(content: string): BackupData {
  const data = JSON.parse(content);
  if (!data || typeof data !== "object" || Array.isArray(data)) {
    throw new Error("Invalid backup file format");
  }
  const version = data.version === undefined ? "1.0" : data.version;
  if (!["1.0", "2.0", "3.0"].includes(version)) {
    throw new Error(`Unsupported backup version: ${String(version)}`);
  }
  if (typeof data.exported_at !== "string"
    || ![data.shows, data.episodes, data.movies].every(Array.isArray)
    || (data.tiers !== undefined && !Array.isArray(data.tiers))) {
    throw new Error("Invalid backup file format");
  }
  if (version === "3.0") {
    const personal = data.personal_data;
    if (!Array.isArray(data.tiers) || !personal || ![
      personal.ceremonies, personal.categories, personal.nominees,
      personal.predictions, personal.title_mappings,
      personal.scrobble_history, personal.change_history,
    ].every(Array.isArray)) {
      throw new Error("Backup v3.0 is missing required tiers, predictions, or history data");
    }
  } else if (data.personal_data != null) {
    throw new Error("Personal data requires backup version 3.0");
  }
  return { ...data, version, tiers: data.tiers ?? [] };
}

export function describeBackup(data: BackupData): string {
  const library = `Replace your library with ${data.shows.length} shows, ${data.episodes.length} episodes, ${data.movies.length} movies, and ${data.tiers.length} tiers.`;
  const personal = data.personal_data;
  const history = personal
    ? `Also replace awards with ${personal.ceremonies.length} ceremonies and ${personal.predictions.length} saved predictions, plus ${personal.title_mappings.length} Plex title corrections, ${personal.scrobble_history.length} scrobbles, and ${personal.change_history.length} change-history entries.`
    : "This older backup contains no predictions or history. Existing awards and predictions will be kept. Plex title corrections, scrobble logs, and change history will be cleared because the file cannot restore them. Export a new backup first if you need to keep them.";
  return `${library}\n\n${history}\n\nSettings, credentials, and racing data stay unchanged. Cached cast/crew and Sonarr/Radarr import links are cleared. Restart TVC after importing.`;
}
