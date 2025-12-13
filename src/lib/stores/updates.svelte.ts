import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { ask } from "@tauri-apps/plugin-dialog";

let updateAvailable = $state(false);
let updateVersion = $state<string | null>(null);
let isChecking = $state(false);
let isDownloading = $state(false);
let downloadProgress = $state(0);

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

export async function checkForUpdates(silent = true): Promise<void> {
  if (isChecking) return;

  isChecking = true;
  try {
    const update = await check();

    if (update?.available) {
      updateAvailable = true;
      updateVersion = update.version;

      if (!silent) {
        const confirmed = await ask(
          `A new version (${update.version}) is available!\n\n${update.body || "No release notes available."}\n\nWould you like to download and install it now?`,
          {
            title: "Update Available",
            kind: "info",
            okLabel: "Update Now",
            cancelLabel: "Later",
          }
        );

        if (confirmed) {
          await downloadAndInstall(update);
        }
      }
    }
  } catch (error) {
    console.error("Failed to check for updates:", error);
  } finally {
    isChecking = false;
  }
}

export async function downloadAndInstall(update?: Update): Promise<void> {
  if (isDownloading) return;

  try {
    // If no update passed, check for one
    if (!update) {
      const checked = await check();
      if (!checked?.available) return;
      update = checked;
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
    console.error("Failed to download/install update:", error);
    isDownloading = false;
    downloadProgress = 0;
  }
}

// Manual update trigger (for settings or menu)
export async function triggerUpdateCheck(): Promise<void> {
  await checkForUpdates(false);

  if (!updateAvailable) {
    await ask("You're running the latest version!", {
      title: "No Updates",
      kind: "info",
      okLabel: "OK",
    });
  }
}
