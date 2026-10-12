<script lang="ts">
  import { modalFocus } from "../../utils/modalFocus";
  import { fade, scale } from "svelte/transition";
  import { Palette, Bell, Flag, Play, Server, X, ListOrdered } from "lucide-svelte";
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
  import TierListTab from "./TierListTab.svelte";

  let { embedded = false }: { embedded?: boolean } = $props();

  function focusSettings(node: HTMLElement) {
    if (!embedded) return modalFocus(node, closeSettings);
  }

  const tabs: { id: SettingsTab; label: string; icon: typeof Palette }[] = [
    { id: 'appearance', label: 'Appearance', icon: Palette },
    { id: 'notifications', label: 'Notifications', icon: Bell },
    { id: 'tierlist', label: 'Tier List', icon: ListOrdered },
    { id: 'racing', label: 'Racing', icon: Flag },
    { id: 'plex', label: 'Plex', icon: Play },
    { id: 'arr', label: 'Sonarr / Radarr', icon: Server },
  ];
</script>

{#if embedded || isSettingsOpen()}
  {#if !embedded}
    <!-- Backdrop -->
    <div
      in:fade={{ duration: 150 }}
      class="fixed inset-0 bg-black/60 z-50"
      onclick={closeSettings}
      role="button"
      tabindex="0"
      onkeydown={(e) => { if (e.key === 'Escape' || e.key === 'Enter' || e.key === ' ') { closeSettings(); } }}
      aria-label="Close settings"
    ></div>
  {/if}

  <!-- Modal or full page -->
  <div
    in:scale={{ duration: embedded ? 0 : 200, start: 0.95, opacity: 0 }}
    class={embedded ? "focus-page focus-settings-page" : "fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-[60] bg-surface rounded-xl border border-border shadow-2xl w-[1000px] max-w-[95vw] h-[85vh] max-h-[85vh] flex flex-col"}
    use:focusSettings
    role={embedded ? "region" : "dialog"}
    aria-modal={embedded ? undefined : true}
    aria-labelledby="settings-modal-title"
  >
    {#if embedded}
      <header class="focus-page-heading">
        <div>
          <p class="focus-eyebrow">Make it yours</p>
          <h1 id="settings-modal-title">Settings</h1>
          <p class="text-text-muted">Adjust TVC's appearance, reminders, and connected services.</p>
        </div>
      </header>
    {:else}
      <div class="flex items-center justify-between px-6 py-4 border-b border-border">
        <h2 id="settings-modal-title" class="text-xl font-semibold">Settings</h2>
        <button onclick={closeSettings} class="p-2 rounded-lg hover:bg-surface-hover transition-colors" aria-label="Close">
          <X class="w-5 h-5 text-text-muted" />
        </button>
      </div>
    {/if}

    <!-- Body: Tab Nav + Content -->
    <div class={embedded ? "focus-settings-body" : "flex flex-1 overflow-hidden"}>
      <!-- Left Tab Nav -->
      <nav class={embedded ? "focus-tabs" : "w-[180px] border-r border-border py-2 flex-shrink-0"} aria-label="Settings sections">
        {#each tabs as tab}
          {@const isActive = getActiveSettingsTab() === tab.id}
          <button
            onclick={() => setActiveSettingsTab(tab.id)}
            class="flex items-center gap-2.5 px-4 py-2.5 text-sm transition-colors text-left {embedded ? '' : 'w-full'}
              {isActive
                ? embedded ? 'text-accent border-b-2 border-accent' : 'bg-accent/10 text-accent border-r-2 border-accent'
                : 'text-text-muted hover:text-text hover:bg-surface-hover'}"
            aria-pressed={isActive}
          >
            <tab.icon class="w-4 h-4 flex-shrink-0" />
            <span class="truncate">{tab.label}</span>
          </button>
        {/each}
      </nav>

      <!-- Right Content Panel -->
      <div class={embedded ? "focus-panel focus-settings-content bg-surface border border-border rounded-xl p-6" : "flex-1 overflow-y-auto p-6"}>
        {#if getActiveSettingsTab() === 'appearance'}
          <AppearanceTab />
        {:else if getActiveSettingsTab() === 'notifications'}
          <NotificationsTab />
        {:else if getActiveSettingsTab() === 'tierlist'}
          <TierListTab />
        {:else if getActiveSettingsTab() === 'racing'}
          <RacingTab />
        {:else if getActiveSettingsTab() === 'plex'}
          <PlexTab />
        {:else if getActiveSettingsTab() === 'arr'}
          <ArrTab />
        {/if}
      </div>
    </div>

    {#if !embedded}
      <!-- Footer -->
      <div class="px-6 py-4 border-t border-border flex justify-end">
        <button
          onclick={closeSettings}
          class="px-4 py-2 text-text-muted hover:bg-surface-hover rounded-lg transition-colors"
        >
          Close
        </button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .focus-settings-page { max-width: 1180px; }
  .focus-settings-body { min-width: 0; }
  .focus-settings-content { min-width: 0; }
  .focus-settings-content :global(h3) { font-size: 1rem; letter-spacing: 0; text-transform: none; color: var(--color-text); }
  .focus-settings-content :global(.bg-background) { border-radius: 10px; }
  .focus-settings-content :global(input:not([type="color"]):not([type="range"])),
  .focus-settings-content :global(select),
  .focus-settings-content :global(textarea) { min-height: 36px; }
  @media (max-width: 760px) {
    .focus-settings-content { padding: 16px; }
  }
</style>
