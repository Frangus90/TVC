import { getDatabase } from "../utils/database";
import { logger } from "../utils/logger";

export interface ThemeSettings {
  colorScheme: string;
  accentColor: string;
  fontSize: number;
  compactSpacing: boolean;
  colorblindFriendly: boolean;
  hidePosters: boolean;
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
  root.style.setProperty("--accent-color", themeSettings.accentColor);
  root.style.setProperty("--accent-hover", adjustBrightness(themeSettings.accentColor, -10));
  root.style.setProperty("--accent-foreground", accentForeground(themeSettings.accentColor));

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

function adjustBrightness(color: string, percent: number): string {
  // Simple brightness adjustment for hover colors
  // This is a simplified version - in production, use a proper color manipulation library
  const num = parseInt((normalizeAccentColor(color) ?? defaultTheme.accentColor).slice(1), 16);
  const amt = Math.round(2.55 * percent);
  const R = Math.max(0, Math.min(255, (num >> 16) + amt));
  const G = Math.max(0, Math.min(255, ((num >> 8) & 0x00ff) + amt));
  const B = Math.max(0, Math.min(255, (num & 0x0000ff) + amt));
  return `#${((1 << 24) + (R << 16) + (G << 8) + B).toString(16).slice(1)}`;
}

// Keep text readable on both the Focus default and saved custom accent colours.
function accentForeground(color: string): string {
  const value = color.replace("#", "");
  const hex = value.length === 3 ? value.split("").map(c => c + c).join("") : value;
  const components = [0, 2, 4].map(index => {
    const channel = parseInt(hex.slice(index, index + 2), 16) / 255;
    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  });
  const luminance = components[0] * 0.2126 + components[1] * 0.7152 + components[2] * 0.0722;
  return luminance > 0.179 ? "#121318" : "#ffffff";
}

// Initialize theme on load
if (typeof window !== "undefined") {
  loadThemeSettings();
}










