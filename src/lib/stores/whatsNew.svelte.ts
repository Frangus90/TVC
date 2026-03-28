import { getVersion } from "@tauri-apps/api/app";
import { logger } from "../utils/logger";

const LAST_SEEN_VERSION_KEY = "tvc_last_seen_version";

// State
let modalOpen = $state(false);
let appVersion = $state<string | null>(null);
let hasUnseen = $state(false);

// Getters
export function isWhatsNewOpen() {
  return modalOpen;
}

export function getAppVersion() {
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
