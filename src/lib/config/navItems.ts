import { CalendarDays, Tv, Clapperboard, Flag, Layers, Trophy, ChartNoAxesCombined, Database, Settings } from "lucide-svelte";
import type { Component } from "svelte";

export type NavId = "calendar" | "shows" | "movies" | "tiers" | "racing" | "awards" | "statistics" | "data" | "settings";

export interface NavItem {
  id: NavId;
  label: string;
  icon: any;
  kind: "calendar" | "standalone";
  load?: () => Promise<{ default: Component<any> }>;
}

// The sidebar and page dispatcher share the same navigation definition.
export const navItems: NavItem[] = [
  { id: "calendar", label: "Calendar", icon: CalendarDays, kind: "calendar" },
  { id: "shows", label: "TV shows", icon: Tv, kind: "standalone", load: () => import("../components/LibraryView.svelte") },
  { id: "movies", label: "Movies", icon: Clapperboard, kind: "standalone", load: () => import("../components/LibraryView.svelte") },
  { id: "tiers", label: "Tier lists", icon: Layers, kind: "standalone", load: () => import("../components/calendar/TierView.svelte") },
  { id: "racing", label: "Racing", icon: Flag, kind: "standalone", load: () => import("../components/racing/RaceCalendar.svelte") },
  { id: "awards", label: "Awards", icon: Trophy, kind: "standalone", load: () => import("../components/awards/AwardsView.svelte") },
  { id: "statistics", label: "Statistics", icon: ChartNoAxesCombined, kind: "standalone", load: () => import("../components/StatisticsDashboard.svelte") },
  { id: "data", label: "Data", icon: Database, kind: "standalone", load: () => import("../components/DataManagement.svelte") },
  { id: "settings", label: "Settings", icon: Settings, kind: "standalone", load: () => import("../components/settings/UnifiedSettings.svelte") },
];

export function navItemById(id: NavId): NavItem {
  return navItems.find((item) => item.id === id) ?? navItems[0];
}
