import { getDatabase } from "../utils/database";
import { logger } from "../utils/logger";
import { accentAppearance } from "../utils/appearanceColors";

export type PosterSize = "small" | "medium" | "large";

export interface ThemeSettings {
  colorScheme: string;
  accentColor: string;
  fontSize: number;
  compactSpacing: boolean;
  colorblindFriendly: boolean;
  hidePosters: boolean;
  posterSize: PosterSize;
}

// Use shared database utility
const getDb = getDatabase;

// Default theme settings
export const defaultTheme: ThemeSettings = {
  colorScheme: "default",
  accentColor: "#bba4ff",
  fontSize: 1,
  compactSpacing: false,
  colorblindFriendly: false,
  hidePosters: false,
  posterSize: "medium",
};

let themeSettings = $state<ThemeSettings>(defaultTheme);
let initialized = $state(false);
let loading = $state(false);
let saving = $state(false);
let loadError = $state<string | null>(null);
let saveError = $state<string | null>(null);
let loadPromise: Promise<void> | null = null;
let saveQueue: Promise<void> = Promise.resolve();
let pendingSaves = 0;
let saveRevision = 0;

export function isThemeLoading() { return loading; }
export async function waitForThemeSaves(): Promise<void> { await loadPromise; await saveQueue; }
export function isThemeSaving() { return saving; }
export function getThemeLoadError() { return loadError; }
export function getThemeSaveError() { return saveError; }

export function normalizeAccentColor(value: string): string | null {
  const color = value.trim();
  if (!/^#(?:[0-9a-f]{3}|[0-9a-f]{6})$/i.test(color)) return null;
  const hex = color.slice(1).toLowerCase();
  return `#${hex.length === 3 ? hex.split("").map(char => char + char).join("") : hex}`;
}

export function getThemeSettings() {
  return themeSettings;
}

export function loadThemeSettings(): Promise<void> {
  if (initialized) return Promise.resolve();
  if (loadPromise) return loadPromise;
  loadPromise = readThemeSettings().finally(() => { loadPromise = null; });
  return loadPromise;
}

async function readThemeSettings(): Promise<void> {
  loading = true;
  loadError = null;
  try {
    const database = await getDb();
    const rows = await database.select<{ key: string; value: string }[]>(
      "SELECT key, value FROM settings WHERE key LIKE 'theme_%'"
    );
    const loaded: Partial<ThemeSettings> = {};
    for (const row of rows) {
      const key = row.key.replace("theme_", "");
      if (key === "accentColor") loaded.accentColor = normalizeAccentColor(row.value) ?? defaultTheme.accentColor;
      else if (key === "posterSize" && ["small", "medium", "large"].includes(row.value)) loaded.posterSize = row.value as PosterSize;
      else if (key === "colorScheme") loaded.colorScheme = row.value;
      else if (key === "fontSize") loaded.fontSize = Math.max(0.75, Math.min(1.5, parseFloat(row.value) || 1));
      else if (key === "compactSpacing" || key === "colorblindFriendly" || key === "hidePosters") loaded[key] = row.value === "true";
    }
    themeSettings = { ...defaultTheme, ...loaded };
    initialized = true;
  } catch (error) {
    logger.error("Failed to load theme settings", error);
    loadError = `Could not load appearance settings: ${error}`;
  } finally {
    applyTheme();
    loading = false;
  }
}

export async function updateThemeSettings(updates: Partial<ThemeSettings>): Promise<boolean> {
  if (!initialized) await loadThemeSettings();
  if (loadError) return false;
  if (updates.accentColor !== undefined) {
    const normalized = normalizeAccentColor(updates.accentColor);
    if (!normalized) { saveError = "Enter a colour as #RGB or #RRGGBB."; return false; }
    updates = { ...updates, accentColor: normalized };
  }
  themeSettings = { ...themeSettings, ...updates };
  applyTheme();
  const snapshot = { ...themeSettings };
  const revision = ++saveRevision;
  pendingSaves++;
  saving = true;
  saveError = null;
  // Persist complete snapshots in order. A later save also recovers earlier failed fields.
  const operation = saveQueue.then(async () => {
    const database = await getDb();
    for (const [key, value] of Object.entries(snapshot)) {
      await database.execute("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)", [`theme_${key}`, String(value)]);
    }
  });
  saveQueue = operation.catch(() => {}); // The caller below reports failures; keep the next save usable.
  try {
    await operation;
    if (revision === saveRevision) saveError = null;
    return true;
  } catch (error) {
    logger.error("Failed to save theme settings", error);
    if (revision === saveRevision) saveError = `Appearance changes were not saved: ${error}`;
    return false;
  } finally {
    saving = --pendingSaves > 0;
  }
}

export function retryThemeSave() { return updateThemeSettings({}); }

function applyTheme() {
  const root = document.documentElement;

  // Apply accent color
  const accent = accentAppearance(themeSettings.accentColor);
  root.style.setProperty("--accent-color", accent.color);
  root.style.setProperty("--accent-hover", accent.hover);
  root.style.setProperty("--accent-foreground", accent.foreground);
  root.style.setProperty("--accent-hover-foreground", accent.hoverForeground);
  root.style.setProperty("--accent-text", accent.text);
  root.style.setProperty("--accent-focus", accent.focus);

  root.style.setProperty("--poster-size-scale", String({ small: 0.8, medium: 1, large: 1.2 }[themeSettings.posterSize]));

  // Apply font size
  root.style.setProperty("--font-size-multiplier", String(themeSettings.fontSize));

  // Apply compact spacing
  if (themeSettings.compactSpacing) {
    root.classList.add("compact-spacing");
  } else {
    root.classList.remove("compact-spacing");
  }

  // Apply colorblind-friendly palette if enabled
  if (themeSettings.colorblindFriendly) {
    root.classList.add("colorblind-friendly");
  } else {
    root.classList.remove("colorblind-friendly");
  }
}

// Initialize theme on load
if (typeof window !== "undefined") {
  loadThemeSettings();
}










