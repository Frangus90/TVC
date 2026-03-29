<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import { Palette, Bell, Flag, Play, Server, X } from "lucide-svelte";
  import {
    isSettingsOpen,
    closeSettings,
    getActiveSettingsTab,
    setActiveSettingsTab,
    type SettingsTab,
  } from "../../stores/settings.svelte";
  import AppearanceTab from "./AppearanceTab.svelte";
  import NotificationsTab from "./NotificationsTab.svelte";
  import RacingTab from "./RacingTab.svelte";
  import PlexTab from "./PlexTab.svelte";
  import ArrTab from "./ArrTab.svelte";

  const tabs: { id: SettingsTab; label: string; icon: typeof Palette }[] = [
    { id: 'appearance', label: 'Appearance', icon: Palette },
    { id: 'notifications', label: 'Notifications', icon: Bell },
    { id: 'racing', label: 'Racing', icon: Flag },
    { id: 'plex', label: 'Plex', icon: Play },
    { id: 'arr', label: 'Sonarr / Radarr', icon: Server },
  ];
</script>

{#if isSettingsOpen()}
  <!-- Backdrop -->
  <div
    transition:fade={{ duration: 150 }}
    class="fixed inset-0 bg-black/60 z-50"
    onclick={closeSettings}
    role="button"
    tabindex="0"
    onkeydown={(e) => { if (e.key === 'Escape' || e.key === 'Enter' || e.key === ' ') { closeSettings(); } }}
    aria-label="Close settings"
  ></div>

  <!-- Modal -->
  <div
    transition:scale={{ duration: 200, start: 0.95, opacity: 0 }}
    class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-[60] bg-surface rounded-xl border border-border shadow-2xl w-[1000px] max-w-[95vw] h-[85vh] max-h-[85vh] flex flex-col"
    role="dialog"
    aria-modal="true"
    aria-labelledby="settings-modal-title"
  >
    <!-- Header -->
    <div class="flex items-center justify-between px-6 py-4 border-b border-border">
      <h2 id="settings-modal-title" class="text-xl font-semibold">Settings</h2>
      <button
        onclick={closeSettings}
        class="p-2 rounded-lg hover:bg-surface-hover transition-colors"
        aria-label="Close"
      >
        <X class="w-5 h-5 text-text-muted" />
      </button>
    </div>

    <!-- Body: Tab Nav + Content -->
    <div class="flex flex-1 overflow-hidden">
      <!-- Left Tab Nav -->
      <nav class="w-[180px] border-r border-border py-2 flex-shrink-0">
        {#each tabs as tab}
          {@const isActive = getActiveSettingsTab() === tab.id}
          <button
            onclick={() => setActiveSettingsTab(tab.id)}
            class="w-full flex items-center gap-3 px-4 py-2.5 text-sm transition-colors text-left
              {isActive
                ? 'bg-accent/10 text-accent border-r-2 border-accent'
                : 'text-text-muted hover:text-text hover:bg-surface-hover'}"
            aria-pressed={isActive}
          >
            <tab.icon class="w-4 h-4 flex-shrink-0" />
            <span class="truncate">{tab.label}</span>
          </button>
        {/each}
      </nav>

      <!-- Right Content Panel -->
      <div class="flex-1 overflow-y-auto p-6">
        {#if getActiveSettingsTab() === 'appearance'}
          <AppearanceTab />
        {:else if getActiveSettingsTab() === 'notifications'}
          <NotificationsTab />
        {:else if getActiveSettingsTab() === 'racing'}
          <RacingTab />
        {:else if getActiveSettingsTab() === 'plex'}
          <PlexTab />
        {:else if getActiveSettingsTab() === 'arr'}
          <ArrTab />
        {/if}
      </div>
    </div>

    <!-- Footer -->
    <div class="px-6 py-4 border-t border-border flex justify-end">
      <button
        onclick={closeSettings}
        class="px-4 py-2 text-text-muted hover:bg-surface-hover rounded-lg transition-colors"
      >
        Close
      </button>
    </div>
  </div>
{/if}
