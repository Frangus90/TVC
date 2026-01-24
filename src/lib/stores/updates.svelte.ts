import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { message } from "@tauri-apps/plugin-dialog";
import { logger } from "../utils/logger";

// State
let updateAvailable = $state(false);
let updateVersion = $state<string | null>(null);
let updateBody = $state<string | null>(null);
let isChecking = $state(false);
let isDownloading = $state(false);
let downloadProgress = $state(0);
let modalOpen = $state(false);
let currentUpdate = $state<Update | null>(null);
let isDummyUpdate = $state(false);

// Getters
export function isUpdateAvailable() {
  return updateAvailable;
}

export function getUpdateVersion() {
  return updateVersion;
}

export function isCheckingForUpdates() {
  return isChecking;
}

export function isDownloadingUpdate() {
  return isDownloading;
}

export function getDownloadProgress() {
  return downloadProgress;
}

export function isUpdateModalOpen() {
  return modalOpen;
}

export function getUpdateInfo() {
  return {
    version: updateVersion,
    body: updateBody,
  };
}

// Actions
export function openUpdateModal() {
  modalOpen = true;
}

export function closeUpdateModal() {
  if (!isDownloading) {
    modalOpen = false;
  }
}

export async function checkForUpdates(silent = true): Promise<void> {
  logger.debug("[TVC Update] checkForUpdates called, silent:", silent);
  if (isChecking) {
    logger.debug("[TVC Update] Already checking, skipping");
    return;
  }

  isChecking = true;
  try {
    logger.debug("[TVC Update] Calling check()...");
    const update = await check();
    logger.debug("[TVC Update] check() result:", update);

    if (update?.available) {
      logger.debug("[TVC Update] Update available! Version:", update.version);
      updateAvailable = true;
      updateVersion = update.version;
      updateBody = update.body || null;
      currentUpdate = update;
      isDummyUpdate = false; // Real update, not dummy

      if (!silent) {
        logger.debug("[TVC Update] Opening update modal...");
        openUpdateModal();
      }
    } else {
      logger.debug("[TVC Update] No update available or update is null");
      // Reset dummy update flag if no real update found
      isDummyUpdate = false;
    }
  } catch (error) {
    logger.error("[TVC Update] Failed to check for updates:", error);
    isDummyUpdate = false; // Reset on error
  } finally {
    isChecking = false;
    logger.debug("[TVC Update] Check complete");
  }
}

export async function downloadAndInstall(update?: Update): Promise<void> {
  if (isDownloading) return;

  try {
    // If no update passed, use the stored one or check for one
    if (!update) {
      if (currentUpdate) {
        update = currentUpdate;
      } else {
        const checked = await check();
        if (!checked?.available) return;
        update = checked;
      }
    }

    isDownloading = true;
    downloadProgress = 0;

    let downloaded = 0;
    let contentLength = 0;

    await update.downloadAndInstall((event) => {
      switch (event.event) {
        case "Started":
          contentLength = event.data.contentLength ?? 0;
          break;
        case "Progress":
          downloaded += event.data.chunkLength;
          if (contentLength > 0) {
            downloadProgress = Math.round((downloaded / contentLength) * 100);
          }
          break;
        case "Finished":
          downloadProgress = 100;
          break;
      }
    });

    // Relaunch the app after installation
    await relaunch();
  } catch (error) {
    logger.error("Failed to download/install update:", error);
    isDownloading = false;
    downloadProgress = 0;
  }
}

// Called from the update modal
export async function downloadAndInstallUpdate(): Promise<void> {
  // Check if this is a dummy update (for development testing)
  if (isDummyUpdate || !currentUpdate) {
    // Import toast system dynamically to avoid circular dependencies
    const { showError } = await import("./toast.svelte");
    showError("This is a test update. Download is disabled in development mode.");
    return;
  }
  await downloadAndInstall(currentUpdate || undefined);
}

// Development-only function to simulate an update for testing
export function simulateDummyUpdate(version: string, releaseNotes: string): void {
  updateAvailable = true;
  updateVersion = version;
  updateBody = releaseNotes;
  currentUpdate = null; // No real update object
  isDummyUpdate = true; // Mark as dummy update
  openUpdateModal();
}

// Manual update trigger (for settings or menu)
export async function triggerUpdateCheck(): Promise<void> {
  await checkForUpdates(false);

  if (!updateAvailable) {
    await message("You're running the latest version!", {
      title: "No Updates",
      kind: "info",
    });
  }
}
