// Sidebar state management

import { getDatabase } from "../utils/database";
import { logger } from "../utils/logger";

export const SIDEBAR_DEFAULT_WIDTH = 256;
export const SIDEBAR_MIN_WIDTH = 200;
export const SIDEBAR_MAX_WIDTH = 500;
export const SIDEBAR_COLLAPSED_WIDTH = 64;

let collapsed = $state(false);
let width = $state(SIDEBAR_DEFAULT_WIDTH);
let isResizing = $state(false);

export function isSidebarCollapsed() {
  return collapsed;
}

export function toggleSidebar() {
  collapsed = !collapsed;
}

export function setSidebarCollapsed(value: boolean) {
  collapsed = value;
}

export function getSidebarWidth() {
  return width;
}

export function getIsResizing() {
  return isResizing;
}

export function setIsResizing(value: boolean) {
  isResizing = value;
}

function clampWidth(w: number): number {
  const windowCap = Math.floor(window.innerWidth * 0.5);
  const max = Math.min(SIDEBAR_MAX_WIDTH, windowCap);
  return Math.max(SIDEBAR_MIN_WIDTH, Math.min(max, w));
}

export function setSidebarWidth(w: number) {
  width = clampWidth(w);
}

export async function loadSidebarWidth(): Promise<void> {
  try {
    const db = await getDatabase();
    const rows = await db.select<{ value: string }[]>(
      "SELECT value FROM settings WHERE key = 'sidebar_width'"
    );
    if (rows.length > 0) {
      const parsed = parseInt(rows[0].value, 10);
      if (!isNaN(parsed)) width = clampWidth(parsed);
    }
  } catch (error) {
    logger.error("Failed to load sidebar width", error);
  }
}

export async function saveSidebarWidth(): Promise<void> {
  try {
    const db = await getDatabase();
    await db.execute(
      "INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)",
      ["sidebar_width", String(width)]
    );
  } catch (error) {
    logger.error("Failed to save sidebar width", error);
  }
}

// Tab state management — the tab union is derived from the nav config so they
// can't drift apart.
import type { NavId } from "../config/navItems";
export type SidebarTab = NavId;
let activeTab = $state<SidebarTab>("shows");

export function getSidebarTab() {
  return activeTab;
}

export function setSidebarTab(tab: SidebarTab) {
  activeTab = tab;
}
