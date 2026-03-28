// Sidebar state management

let collapsed = $state(false);

export function isSidebarCollapsed() {
  return collapsed;
}

export function toggleSidebar() {
  collapsed = !collapsed;
}

export function setSidebarCollapsed(value: boolean) {
  collapsed = value;
}

// Tab state management
export type SidebarTab = "shows" | "movies" | "archive" | "racing";
let activeTab = $state<SidebarTab>("shows");

export function getSidebarTab() {
  return activeTab;
}

export function setSidebarTab(tab: SidebarTab) {
  activeTab = tab;
}
