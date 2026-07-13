import { getVersion } from "@tauri-apps/api/app";
import { logger } from "../utils/logger";
import changelog from "../../../CHANGELOG.md?raw";

const LAST_SEEN_VERSION_KEY = "tvc_last_seen_version";

// State
let modalOpen = $state(false);
let appVersion = $state<string | null>(null);
let hasUnseen = $state(false);

// In dev builds, display the newest changelog version (with a -dev suffix)
// instead of the real Tauri app version, so the sidebar reflects unreleased
// work. This only affects what's displayed — appVersion (used for the
// hasUnseenChanges/last-seen comparison) always stays the real app version.
function getDevDisplayVersion(): string | null {
  const match = changelog.match(/^##\s+\[(\d+\.\d+\.\d+)\]/m);
  return match ? `${match[1]}-dev` : null;
}

// Getters
export function isWhatsNewOpen() {
  return modalOpen;
}

export function getAppVersion() {
  if (import.meta.env.DEV) {
    return getDevDisplayVersion() ?? appVersion;
  }
  return appVersion;
}

export function hasUnseenChanges() {
  return hasUnseen;
}

// Actions
export function openWhatsNew() {
  modalOpen = true;
  markAsSeen();
}

export function closeWhatsNew() {
  modalOpen = false;
}

function markAsSeen() {
  if (appVersion) {
    localStorage.setItem(LAST_SEEN_VERSION_KEY, appVersion);
    hasUnseen = false;
  }
}

export async function initWhatsNew(): Promise<void> {
  try {
    appVersion = await getVersion();
    const lastSeen = localStorage.getItem(LAST_SEEN_VERSION_KEY);

    if (!lastSeen || lastSeen !== appVersion) {
      hasUnseen = true;
      // Auto-show on first launch after update (but not on very first install)
      if (lastSeen) {
        modalOpen = true;
        markAsSeen();
      } else {
        // First install — just mark current version as seen, don't show modal
        markAsSeen();
      }
    }
  } catch (error) {
    logger.error("Failed to initialize What's New", error);
    appVersion = "0.0.0";
  }
}
