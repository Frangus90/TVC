<script lang="ts">
  import { X, Bell, BellOff, Volume2, VolumeX, Monitor, MonitorOff, Play, Zap, Flag, Tv, Download, Info } from "lucide-svelte";
  import {
    getNotificationSettings,
    isNotificationSettingsOpen,
    closeNotificationSettings,
    updateNotificationSettings,
    testNotification,
  } from "../../stores/notifications.svelte";
  import { previewSound, getAvailableSounds } from "../../utils/notificationSound";

  const isDev = import.meta.env.DEV;
  const settings = $derived(getNotificationSettings());
  const sounds = getAvailableSounds();
  let testingAll = $state(false);

  const testTypes = [
    { type: "racing", label: "Racing", icon: Flag, color: "text-red-400" },
    { type: "plex", label: "Plex", icon: Play, color: "text-orange-400" },
    { type: "premiere", label: "Premiere", icon: Tv, color: "text-accent" },
    { type: "update", label: "Update", icon: Download, color: "text-green-400" },
    { type: "system", label: "System", icon: Info, color: "text-blue-400" },
  ];

  async function fireAll() {
    testingAll = true;
    for (const t of testTypes) {
      await testNotification(t.type);
      await new Promise((r) => setTimeout(r, 600));
    }
    testingAll = false;
  }

  const positionOptions = [
    { value: "top-right", label: "Top Right" },
    { value: "top-left", label: "Top Left" },
    { value: "bottom-right", label: "Bottom Right" },
    { value: "bottom-left", label: "Bottom Left" },
  ];

  const durationOptions = [
    { value: 3000, label: "3 seconds" },
    { value: 5000, label: "5 seconds" },
    { value: 8000, label: "8 seconds" },
    { value: 12000, label: "12 seconds" },
    { value: 0, label: "Manual dismiss" },
  ];

  const categoryToggles = [
    { key: "racing_enabled", label: "Racing", description: "Race session reminders" },
    { key: "plex_enabled", label: "Plex", description: "Plex scrobble events" },
    { key: "premiere_enabled", label: "Premieres", description: "Show & movie premieres" },
    { key: "update_enabled", label: "Updates", description: "App update notifications" },
    { key: "system_enabled", label: "System", description: "System messages" },
  ] as const;

  function toggle(key: string) {
    if (!settings) return;
    updateNotificationSettings({ [key]: !(settings as any)[key] });
  }

  function handlePreviewSound() {
    if (!settings) return;
    previewSound(settings.sound_choice, settings.sound_volume);
  }
</script>

{#if isNotificationSettingsOpen() && settings}
  <button
    class="fixed inset-0 bg-black/50 z-40"
    onclick={closeNotificationSettings}
    aria-label="Close notification settings"
  ></button>

  <div class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 bg-surface rounded-xl border border-border shadow-2xl w-[520px] max-w-[95vw] max-h-[85vh] flex flex-col">
    <!-- Header -->
    <div class="p-5 border-b border-border flex items-center justify-between">
      <h2 class="text-lg font-semibold text-text">Notification Settings</h2>
      <button
        onclick={closeNotificationSettings}
        class="p-2 rounded-lg hover:bg-surface-hover transition-colors text-text-muted"
      >
        <X class="w-5 h-5" />
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-auto p-5 space-y-6">
      <!-- General -->
      <div class="space-y-3">
        <h3 class="text-sm font-semibold text-text-muted uppercase tracking-wider">General</h3>

        <!-- Master toggle -->
        <div class="flex items-center justify-between p-3 rounded-lg bg-background border border-border">
          <div class="flex items-center gap-3">
            {#if settings.enabled}
              <Bell class="w-4 h-4 text-accent" />
            {:else}
              <BellOff class="w-4 h-4 text-text-muted" />
            {/if}
            <div>
              <p class="text-sm font-medium text-text">Notifications</p>
              <p class="text-xs text-text-muted">Enable in-app notifications</p>
            </div>
          </div>
          <button
            onclick={() => toggle("enabled")}
            class="relative w-10 h-5 rounded-full transition-colors {settings.enabled ? 'bg-accent' : 'bg-border'}"
            aria-label="Toggle notifications"
          >
            <span class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform {settings.enabled ? 'translate-x-5' : ''}"></span>
          </button>
        </div>

        <!-- Tray notifications -->
        <div class="flex items-center justify-between p-3 rounded-lg bg-background border border-border">
          <div class="flex items-center gap-3">
            {#if settings.tray_notifications}
              <Monitor class="w-4 h-4 text-accent" />
            {:else}
              <MonitorOff class="w-4 h-4 text-text-muted" />
            {/if}
            <div>
              <p class="text-sm font-medium text-text">Notify When Minimized</p>
              <p class="text-xs text-text-muted">Send OS notifications when app is in the tray</p>
            </div>
          </div>
          <button
            onclick={() => toggle("tray_notifications")}
            class="relative w-10 h-5 rounded-full transition-colors {settings.tray_notifications ? 'bg-accent' : 'bg-border'}"
            aria-label="Toggle tray notifications"
          >
            <span class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform {settings.tray_notifications ? 'translate-x-5' : ''}"></span>
          </button>
        </div>

        <!-- OS fallback -->
        <div class="flex items-center justify-between p-3 rounded-lg bg-background border border-border">
          <div class="flex items-center gap-3">
            <Monitor class="w-4 h-4 text-text-muted" />
            <div>
              <p class="text-sm font-medium text-text">Always Send OS Notifications</p>
              <p class="text-xs text-text-muted">Also send Windows notifications when app is visible</p>
            </div>
          </div>
          <button
            onclick={() => toggle("os_fallback")}
            class="relative w-10 h-5 rounded-full transition-colors {settings.os_fallback ? 'bg-accent' : 'bg-border'}"
            aria-label="Toggle OS notifications"
          >
            <span class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform {settings.os_fallback ? 'translate-x-5' : ''}"></span>
          </button>
        </div>
      </div>

      <!-- Sound -->
      <div class="space-y-3">
        <h3 class="text-sm font-semibold text-text-muted uppercase tracking-wider">Sound</h3>

        <!-- Sound toggle -->
        <div class="flex items-center justify-between p-3 rounded-lg bg-background border border-border">
          <div class="flex items-center gap-3">
            {#if settings.sound_enabled}
              <Volume2 class="w-4 h-4 text-accent" />
            {:else}
              <VolumeX class="w-4 h-4 text-text-muted" />
            {/if}
            <div>
              <p class="text-sm font-medium text-text">Sound</p>
              <p class="text-xs text-text-muted">Play a sound with notifications</p>
            </div>
          </div>
          <button
            onclick={() => toggle("sound_enabled")}
            class="relative w-10 h-5 rounded-full transition-colors {settings.sound_enabled ? 'bg-accent' : 'bg-border'}"
            aria-label="Toggle notification sound"
          >
            <span class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform {settings.sound_enabled ? 'translate-x-5' : ''}"></span>
          </button>
        </div>

        {#if settings.sound_enabled}
          <!-- Sound choice -->
          <div class="flex items-center justify-between p-3 rounded-lg bg-background border border-border">
            <p class="text-sm font-medium text-text">Sound</p>
            <div class="flex items-center gap-2">
              <select
                value={settings.sound_choice}
                onchange={(e) => updateNotificationSettings({ sound_choice: (e.target as HTMLSelectElement).value })}
                class="bg-surface border border-border rounded-md px-2 py-1 text-sm text-text"
              >
                {#each sounds as sound}
                  <option value={sound.value}>{sound.label}</option>
                {/each}
              </select>
              <button
                onclick={handlePreviewSound}
                class="p-1.5 rounded-md hover:bg-surface-hover transition-colors text-text-muted hover:text-text"
                title="Preview sound"
              >
                <Play class="w-4 h-4" />
              </button>
            </div>
          </div>

          <!-- Volume slider -->
          <div class="p-3 rounded-lg bg-background border border-border">
            <div class="flex items-center justify-between mb-2">
              <p class="text-sm font-medium text-text">Volume</p>
              <span class="text-xs text-text-muted">{settings.sound_volume}%</span>
            </div>
            <input
              type="range"
              min="0"
              max="100"
              step="5"
              value={settings.sound_volume}
              oninput={(e) => updateNotificationSettings({ sound_volume: parseInt((e.target as HTMLInputElement).value) })}
              class="w-full accent-accent"
            />
          </div>
        {/if}
      </div>

      <!-- Display -->
      <div class="space-y-3">
        <h3 class="text-sm font-semibold text-text-muted uppercase tracking-wider">Display</h3>

        <!-- Position -->
        <div class="flex items-center justify-between p-3 rounded-lg bg-background border border-border">
          <p class="text-sm font-medium text-text">Popup Position</p>
          <select
            value={settings.popup_position}
            onchange={(e) => updateNotificationSettings({ popup_position: (e.target as HTMLSelectElement).value })}
            class="bg-surface border border-border rounded-md px-2 py-1 text-sm text-text"
          >
            {#each positionOptions as opt}
              <option value={opt.value}>{opt.label}</option>
            {/each}
          </select>
        </div>

        <!-- Duration -->
        <div class="flex items-center justify-between p-3 rounded-lg bg-background border border-border">
          <p class="text-sm font-medium text-text">Auto-dismiss</p>
          <select
            value={settings.popup_duration}
            onchange={(e) => updateNotificationSettings({ popup_duration: parseInt((e.target as HTMLSelectElement).value) })}
            class="bg-surface border border-border rounded-md px-2 py-1 text-sm text-text"
          >
            {#each durationOptions as opt}
              <option value={opt.value}>{opt.label}</option>
            {/each}
          </select>
        </div>

        <!-- Max visible -->
        <div class="flex items-center justify-between p-3 rounded-lg bg-background border border-border">
          <p class="text-sm font-medium text-text">Max Popups</p>
          <select
            value={settings.max_visible}
            onchange={(e) => updateNotificationSettings({ max_visible: parseInt((e.target as HTMLSelectElement).value) })}
            class="bg-surface border border-border rounded-md px-2 py-1 text-sm text-text"
          >
            {#each [1, 2, 3, 4, 5] as n}
              <option value={n}>{n}</option>
            {/each}
          </select>
        </div>
      </div>

      <!-- Categories -->
      <div class="space-y-3">
        <h3 class="text-sm font-semibold text-text-muted uppercase tracking-wider">Categories</h3>

        {#each categoryToggles as cat}
          <div class="flex items-center justify-between p-3 rounded-lg bg-background border border-border">
            <div>
              <p class="text-sm font-medium text-text">{cat.label}</p>
              <p class="text-xs text-text-muted">{cat.description}</p>
            </div>
            <button
              onclick={() => toggle(cat.key)}
              class="relative w-10 h-5 rounded-full transition-colors {(settings as any)[cat.key] ? 'bg-accent' : 'bg-border'}"
              aria-label="Toggle {cat.label} notifications"
            >
              <span class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform {(settings as any)[cat.key] ? 'translate-x-5' : ''}"></span>
            </button>
          </div>
        {/each}
      </div>

      <!-- Dev Testing -->
      {#if isDev}
        <div class="space-y-3">
          <h3 class="text-sm font-semibold text-yellow-500 uppercase tracking-wider flex items-center gap-2">
            <Zap class="w-4 h-4" />
            Dev Testing
          </h3>

          <div class="grid grid-cols-2 gap-2">
            {#each testTypes as t}
              {@const Icon = t.icon}
              <button
                onclick={() => testNotification(t.type)}
                class="flex items-center gap-2 px-3 py-2 rounded-lg bg-background border border-border hover:bg-surface-hover transition-colors text-sm font-medium"
              >
                <Icon class="w-4 h-4 {t.color}" />
                <span class="text-text">{t.label}</span>
              </button>
            {/each}
          </div>

          <button
            onclick={fireAll}
            disabled={testingAll}
            class="w-full flex items-center justify-center gap-2 px-4 py-2 rounded-lg bg-yellow-500/10 text-yellow-500 hover:bg-yellow-500/20 transition-colors text-sm font-medium disabled:opacity-50"
          >
            <Zap class="w-4 h-4" />
            {testingAll ? "Firing..." : "Fire All Types"}
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}
