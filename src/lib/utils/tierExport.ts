import { mount, unmount } from "svelte";
import { domToBlob } from "modern-screenshot";
import { save } from "@tauri-apps/plugin-dialog";
import { writeFile } from "@tauri-apps/plugin-fs";
import { getVersion } from "@tauri-apps/api/app";
import TierListExport from "../components/calendar/TierListExport.svelte";
import {
  getTiers,
  getTierListShows,
  getTierListMovies,
  type TierListShow,
  type TierListMovie,
} from "../stores/tiers.svelte";
import { showSuccess, showError, showInfo } from "../stores/toast.svelte";
import { logger } from "./logger";

const IMAGE_WAIT_TIMEOUT_MS = 30000;
// Downgrade scale only for extremely tall DOMs that would otherwise exceed
// Chromium's 16384-px canvas side limit at scale=2.
const SCALE_DOWNGRADE_HEIGHT_PX = 8000;

type ExportItem = TierListShow | TierListMovie;

function basename(p: string): string {
  const idx = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
  return idx >= 0 ? p.slice(idx + 1) : p;
}

async function waitForImages(host: HTMLElement): Promise<void> {
  const imgs = Array.from(host.querySelectorAll("img"));
  const all = Promise.all(
    imgs.map(async (img) => {
      if (!img.complete) {
        await new Promise<void>((res) => {
          img.onload = () => res();
          img.onerror = () => res();
        });
      }
      try {
        await img.decode();
      } catch {
        // ignore decode errors; the renderer falls back to a placeholder
      }
    })
  );
  const timeout = new Promise<void>((res) =>
    setTimeout(res, IMAGE_WAIT_TIMEOUT_MS)
  );
  await Promise.race([all, timeout]);
}

function nextPaint(): Promise<void> {
  return new Promise((res) =>
    requestAnimationFrame(() => requestAnimationFrame(() => res()))
  );
}

// Pre-fetch every poster and rasterize it down to the exact display size as a
// small JPEG data URL. This collapses each poster from ~50 KB TMDB source to
// ~5 KB and, critically, gives the screenshot library an image whose intrinsic
// size matches its display size — eliminating the per-img scaled bitmap that
// otherwise pressures Chromium's decoded image cache when rendering 300+ imgs.
const RASTER_W = 192;
const RASTER_H = 288;

async function rasterizeBlobToDataUrl(blob: Blob): Promise<string> {
  const bitmap = await createImageBitmap(blob);
  try {
    const canvas = document.createElement("canvas");
    canvas.width = RASTER_W;
    canvas.height = RASTER_H;
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("2D context unavailable");
    ctx.drawImage(bitmap, 0, 0, RASTER_W, RASTER_H);
    return canvas.toDataURL("image/jpeg", 0.85);
  } finally {
    bitmap.close();
  }
}

async function preloadPostersAsDataUrls(
  items: ExportItem[]
): Promise<ExportItem[]> {
  const uniqueUrls = Array.from(
    new Set(
      items
        .map((i) => i.poster_url)
        .filter((u): u is string => typeof u === "string" && u.length > 0)
    )
  );

  const urlMap = new Map<string, string>();
  let okCount = 0;
  let failCount = 0;

  await Promise.all(
    uniqueUrls.map(async (url) => {
      try {
        const res = await fetch(url, { mode: "cors", cache: "reload" });
        if (!res.ok) {
          failCount++;
          return;
        }
        const blob = await res.blob();
        const dataUrl = await rasterizeBlobToDataUrl(blob);
        urlMap.set(url, dataUrl);
        okCount++;
      } catch (e) {
        failCount++;
        logger.error(`Failed to preload poster ${url}`, e);
      }
    })
  );

  logger.info(
    `[tier export] preloaded posters: ${okCount} ok, ${failCount} failed, ${uniqueUrls.length} unique (rasterized to ${RASTER_W}x${RASTER_H})`
  );

  return items.map((i) => ({
    ...i,
    poster_url: i.poster_url ? (urlMap.get(i.poster_url) ?? null) : null,
  })) as ExportItem[];
}

export async function exportTierListAsImage(
  mode: "shows" | "movies"
): Promise<void> {
  const tiers = getTiers();
  const items: ExportItem[] =
    mode === "shows" ? getTierListShows() : getTierListMovies();

  if (items.length === 0) {
    showInfo(`Nothing to export — your ${mode} tier list is empty.`);
    return;
  }

  let appVersion = "0.0.0";
  try {
    appVersion = await getVersion();
  } catch (e) {
    logger.error("Failed to read app version for export", e);
  }

  // Render the host fully visible (no opacity, no negative z-index) so
  // Webview2's compositor actually uploads GPU bitmaps for every poster.
  // Hide the in-progress render from the user behind a covering overlay.
  const overlay = document.createElement("div");
  overlay.style.cssText =
    "position:fixed; inset:0; background:#0f0f0f; z-index:1000000; " +
    "display:flex; align-items:center; justify-content:center; " +
    "color:#a3a3a3; font-size:16px; " +
    'font-family:-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;';
  overlay.textContent = "Generating tier list image…";
  document.body.appendChild(overlay);

  const host = document.createElement("div");
  host.style.cssText =
    "position:fixed; left:0; top:0; z-index:999999; pointer-events:none;";
  document.body.appendChild(host);

  let component: ReturnType<typeof mount> | null = null;

  try {
    const preloadedItems = await preloadPostersAsDataUrls(items);

    component = mount(TierListExport, {
      target: host,
      props: {
        mode,
        tiers,
        items: preloadedItems as TierListShow[] | TierListMovie[],
        appVersion,
      },
    });

    if (document.fonts && document.fonts.ready) {
      await document.fonts.ready;
    }
    await waitForImages(host);
    await nextPaint();

    const target = host.firstElementChild as HTMLElement | null;
    if (!target) {
      throw new Error("Export target was not rendered");
    }

    const scale = target.scrollHeight > SCALE_DOWNGRADE_HEIGHT_PX ? 1 : 2;
    logger.info(
      `[tier export] rendering at scale ${scale}, target height ${target.scrollHeight}px`
    );

    const blob = await domToBlob(target, {
      scale,
      type: "image/png",
      backgroundColor: "#0f0f0f",
      width: target.scrollWidth,
      height: target.scrollHeight,
    });

    logger.info(
      `[tier export] PNG blob size: ${(blob.size / 1024).toFixed(0)} KB`
    );

    if (!blob) {
      throw new Error("Failed to encode PNG");
    }

    const today = new Date().toISOString().slice(0, 10);
    const filePath = await save({
      defaultPath: `tvc-${mode}-tier-list-${today}.png`,
      filters: [{ name: "PNG image", extensions: ["png"] }],
    });

    if (!filePath) {
      return;
    }

    const bytes = new Uint8Array(await blob.arrayBuffer());
    await writeFile(filePath, bytes);

    showSuccess(`Saved ${basename(filePath)}`);
  } catch (err) {
    logger.error("Tier list export failed", err);
    showError(`Export failed: ${err instanceof Error ? err.message : err}`);
  } finally {
    if (component) {
      try {
        unmount(component);
      } catch (e) {
        logger.error("Failed to unmount export component", e);
      }
    }
    host.remove();
    overlay.remove();
  }
}
