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
