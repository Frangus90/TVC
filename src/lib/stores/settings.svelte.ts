export type SettingsTab = 'general' | 'appearance' | 'notifications' | 'tierlist' | 'racing' | 'integrations';
type SettingsTarget = SettingsTab | 'plex' | 'arr';

let settingsOpen = $state(false);
let activeTab = $state<SettingsTab>('general');
let integrationTab = $state<'plex' | 'arr'>('plex');

export function openSettings(tab?: SettingsTarget) {
  if (tab) setActiveSettingsTab(tab);
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

export function setActiveSettingsTab(tab: SettingsTarget) {
  if (tab === 'plex' || tab === 'arr') {
    integrationTab = tab;
    activeTab = 'integrations';
  } else activeTab = tab;
}

export function getActiveIntegrationTab() { return integrationTab; }
export function setActiveIntegrationTab(tab: 'plex' | 'arr') { integrationTab = tab; }
