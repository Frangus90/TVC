export type SettingsTab = 'appearance' | 'notifications' | 'tierlist' | 'racing' | 'plex' | 'arr';

let settingsOpen = $state(false);
let activeTab = $state<SettingsTab>('appearance');

export function openSettings(tab?: SettingsTab) {
  if (tab) activeTab = tab;
  settingsOpen = true;
}

export function closeSettings() {
  settingsOpen = false;
}

export function isSettingsOpen() {
  return settingsOpen;
}

export function getActiveSettingsTab() {
  return activeTab;
}

export function setActiveSettingsTab(tab: SettingsTab) {
  activeTab = tab;
}
