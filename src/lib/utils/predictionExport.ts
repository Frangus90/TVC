import { mount, unmount } from "svelte";
import { domToBlob } from "modern-screenshot";
import { save } from "@tauri-apps/plugin-dialog";
import { writeFile } from "@tauri-apps/plugin-fs";
import { getVersion } from "@tauri-apps/api/app";
import PredictionExport from "../components/awards/PredictionExport.svelte";
import type { CeremonyDetail } from "../stores/awards.svelte";
import { showInfo, showSuccess, showError } from "../stores/toast.svelte";
import { logger } from "./logger";

interface Pick {
  category: string;
  pick: string;
  result: "win" | "miss" | null;
}

function basename(p: string): string {
  const idx = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
  return idx >= 0 ? p.slice(idx + 1) : p;
}

/**
 * Render the user's picks for a ceremony to a shareable PNG. Only categories with
 * a prediction are included; skipped ones are omitted. If the winners are known,
 * each pick is marked correct/incorrect.
 */
export async function exportPredictionsAsImage(
  ceremony: CeremonyDetail,
  predictions: Record<number, number>,
): Promise<void> {
  const isPast = ceremony.status === "past";
  const picks: Pick[] = [];
  for (const cat of ceremony.categories) {
    const nomineeId = predictions[cat.id];
    if (nomineeId == null) continue;
    const nom = cat.nominees.find((n) => n.id === nomineeId);
    if (!nom) continue;
    picks.push({
      category: cat.name,
      pick: nom.title,
      result: isPast ? (nom.is_winner ? "win" : "miss") : null,
    });
  }

  if (picks.length === 0) {
    showInfo("No picks to export — pick a winner in at least one category first.");
    return;
  }

  let appVersion = "0.0.0";
  try {
    appVersion = await getVersion();
  } catch (e) {
    logger.error("Failed to read app version for export", e);
  }

  // Render off-screen (kept laid out so fonts/metrics resolve) then capture.
  const host = document.createElement("div");
  host.style.cssText = "position:fixed; left:-10000px; top:0; pointer-events:none;";
  document.body.appendChild(host);
  let component: ReturnType<typeof mount> | null = null;

  try {
    component = mount(PredictionExport, {
      target: host,
      props: {
        ceremonyName: ceremony.name,
        picks,
        appVersion,
        date: new Date().toISOString().slice(0, 10),
      },
    });

    if (document.fonts?.ready) await document.fonts.ready;
    await new Promise<void>((res) =>
      requestAnimationFrame(() => requestAnimationFrame(() => res())),
    );

    const target = host.firstElementChild as HTMLElement | null;
    if (!target) throw new Error("Export target was not rendered");

    const blob = await domToBlob(target, {
      scale: 2,
      type: "image/png",
      backgroundColor: "#0f0f0f",
      width: target.scrollWidth,
      height: target.scrollHeight,
    });
    if (!blob) throw new Error("Failed to encode PNG");

    const today = new Date().toISOString().slice(0, 10);
    const slug = ceremony.name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-|-$/g, "");
    const filePath = await save({
      defaultPath: `tvc-picks-${slug}-${today}.png`,
      filters: [{ name: "PNG image", extensions: ["png"] }],
    });
    if (!filePath) return;

    await writeFile(filePath, new Uint8Array(await blob.arrayBuffer()));
    showSuccess(`Saved ${basename(filePath)}`);
  } catch (err) {
    logger.error("Prediction export failed", err);
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
  }
}
