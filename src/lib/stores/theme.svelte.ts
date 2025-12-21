import Database from "@tauri-apps/plugin-sql";

// Use separate database in dev mode to avoid breaking production data
const DB_NAME = import.meta.env.DEV ? "sqlite:tvc_dev.db" : "sqlite:tvc.db";

export interface ThemeSettings {
  colorScheme: string;
  accentColor: string;
  fontSize: number;
  compactSpacing: boolean;
  colorblindFriendly: boolean;
}

let db: Database | null = null;

async function getDb(): Promise<Database> {
  if (!db) {
    db = await Database.load(DB_NAME);
  }
  return db;
}

// Default theme settings
const defaultTheme: ThemeSettings = {
  colorScheme: "default",
  accentColor: "#3b82f6",
  fontSize: 1,
  compactSpacing: false,
  colorblindFriendly: false,
};

let themeSettings = $state<ThemeSettings>(defaultTheme);
let initialized = $state(false);

export function getThemeSettings() {
  return themeSettings;
}

export async function loadThemeSettings(): Promise<void> {
  if (initialized) return;

  try {
    const database = await getDb();
    const settings = await database.select<{ key: string; value: string }[]>(
      "SELECT key, value FROM settings WHERE key LIKE 'theme_%'"
    );

    const loaded: Partial<ThemeSettings> = {};
    for (const setting of settings) {
      const key = setting.key.replace("theme_", "");
      if (key === "colorScheme" || key === "accentColor") {
        (loaded as any)[key] = setting.value;
      } else if (key === "fontSize") {
        loaded.fontSize = parseFloat(setting.value) || 1;
      } else if (key === "compactSpacing") {
        loaded.compactSpacing = setting.value === "true";
      } else if (key === "colorblindFriendly") {
        loaded.colorblindFriendly = setting.value === "true";
      }
    }

    themeSettings = { ...defaultTheme, ...loaded };
    applyTheme();
    initialized = true;
  } catch (error) {
    console.error("Failed to load theme settings:", error);
    applyTheme();
    initialized = true;
  }
}

export async function updateThemeSettings(updates: Partial<ThemeSettings>): Promise<void> {
  themeSettings = { ...themeSettings, ...updates };
  applyTheme();

  try {
    const database = await getDb();
    for (const [key, value] of Object.entries(updates)) {
      await database.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)",
        [`theme_${key}`, String(value)]
      );
    }
  } catch (error) {
    console.error("Failed to save theme settings:", error);
  }
}

function applyTheme() {
  const root = document.documentElement;

  // Apply accent color
  root.style.setProperty("--accent-color", themeSettings.accentColor);
  root.style.setProperty("--accent-hover", adjustBrightness(themeSettings.accentColor, -10));

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
  const num = parseInt(color.replace("#", ""), 16);
  const amt = Math.round(2.55 * percent);
  const R = Math.max(0, Math.min(255, (num >> 16) + amt));
  const G = Math.max(0, Math.min(255, ((num >> 8) & 0x00ff) + amt));
  const B = Math.max(0, Math.min(255, (num & 0x0000ff) + amt));
  return `#${((1 << 24) + (R << 16) + (G << 8) + B).toString(16).slice(1)}`;
}

// Initialize theme on load
if (typeof window !== "undefined") {
  loadThemeSettings();
}






