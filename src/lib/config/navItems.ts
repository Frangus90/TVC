import { Tv, Film, Archive, Flag, Layers, Award } from "lucide-svelte";
import type { Component } from "svelte";

/**
 * Single source of truth for the app's top-level navigation. Adding or removing a
 * feature is a one-line edit here — the sidebar tab row, the main-area dispatch in
 * App.svelte, and the Header all derive from this list.
 *
 * - `kind: "calendar"` — main area is the calendar (Month/Week/Agenda) and the tab
 *   has a contextual list panel in the sidebar (Shows/Movies/Archive).
 * - `kind: "standalone"` — main area is `load()`'s component, full-width, no list
 *   panel (Racing/Tiers/Awards).
 */
export type NavId = "shows" | "movies" | "archive" | "racing" | "tiers" | "awards";

export interface NavItem {
  id: NavId;
  label: string;
  icon: Component;
  kind: "calendar" | "standalone";
  /** Lazy loader for a standalone tab's main-area component. */
  load?: () => Promise<{ default: Component }>;
}

export const navItems: NavItem[] = [
  { id: "shows", label: "Shows", icon: Tv, kind: "calendar" },
  { id: "movies", label: "Movies", icon: Film, kind: "calendar" },
  { id: "archive", label: "Archive", icon: Archive, kind: "calendar" },
  {
    id: "racing",
    label: "Racing",
    icon: Flag,
    kind: "standalone",
    load: () => import("../components/racing/RaceCalendar.svelte"),
  },
  {
    id: "tiers",
    label: "Tiers",
    icon: Layers,
    kind: "standalone",
    load: () => import("../components/calendar/TierView.svelte"),
  },
  {
    id: "awards",
    label: "Awards",
    icon: Award,
    kind: "standalone",
    load: () => import("../components/awards/AwardsView.svelte"),
  },
];

export function navItemById(id: NavId): NavItem {
  return navItems.find((n) => n.id === id) ?? navItems[0];
}
