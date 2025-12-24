<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import { Settings, X } from "lucide-svelte";
  import {
    getThemeSettings,
    updateThemeSettings,
  } from "../stores/theme.svelte";

  let open = $state(false);
  
  function getSettings() {
    return getThemeSettings();
  }

  async function handleAccentColorChange(event: Event) {
    const input = event.target as HTMLInputElement;
    await updateThemeSettings({ accentColor: input.value });
  }

  async function handleFontSizeChange(event: Event) {
    const input = event.target as HTMLInputElement;
    await updateThemeSettings({ fontSize: parseFloat(input.value) });
  }

  async function handleCompactSpacingChange(event: Event) {
    const input = event.target as HTMLInputElement;
    await updateThemeSettings({ compactSpacing: input.checked });
  }

  async function handleColorblindFriendlyChange(event: Event) {
    const input = event.target as HTMLInputElement;
    await updateThemeSettings({ colorblindFriendly: input.checked });
  }

  async function handleHidePostersChange(event: Event) {
    const input = event.target as HTMLInputElement;
    await updateThemeSettings({ hidePosters: input.checked });
  }
</script>

<button
  onclick={() => open = !open}
  class="p-2 rounded-lg hover:bg-surface-hover transition-colors"
  aria-label="Theme settings"
  title="Theme settings"
>
  <Settings class="w-5 h-5 text-text-muted" />
</button>

{#if open}
  <!-- Backdrop -->
  <div
    transition:fade={{ duration: 150 }}
    class="fixed inset-0 bg-black/60 z-50"
    onclick={() => open = false}
    role="button"
    tabindex="0"
    onkeydown={(e) => { if (e.key === 'Escape' || e.key === 'Enter' || e.key === ' ') { open = false; } }}
    aria-label="Close modal"
  ></div>

  <!-- Modal -->
  <div
    transition:scale={{ duration: 200, start: 0.95, opacity: 0 }}
    class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-[60] bg-surface rounded-xl border border-border shadow-2xl w-[500px] max-w-[95vw] max-h-[85vh] flex flex-col"
    role="dialog"
    aria-modal="true"
    aria-labelledby="theme-modal-title"
  >
    <!-- Header -->
    <div class="flex items-center justify-between p-5 border-b border-border">
      <h2 id="theme-modal-title" class="text-xl font-semibold">Theme Settings</h2>
      <button
        onclick={() => open = false}
        class="p-2 rounded-lg hover:bg-surface-hover transition-colors"
        aria-label="Close"
      >
        <X class="w-5 h-5 text-text-muted" />
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-5 space-y-6">
      <!-- Accent Color -->
      <div>
        <label for="accent-color-label" class="block text-sm font-medium text-text-muted mb-2">
          Accent Color
        </label>
        <div class="flex items-center gap-3">
          <input
            id="accent-color-picker"
            type="color"
            value={getSettings().accentColor}
            onchange={handleAccentColorChange}
            class="w-16 h-10 rounded border border-border cursor-pointer"
            aria-labelledby="accent-color-label"
          />
          <input
            id="accent-color-text"
            type="text"
            value={getSettings().accentColor}
            onchange={handleAccentColorChange}
            class="flex-1 px-3 py-2 rounded border border-border bg-surface text-text"
            placeholder="#3b82f6"
            aria-labelledby="accent-color-label"
          />
        </div>
      </div>

      <!-- Font Size -->
      <div>
        <label for="font-size-range" class="block text-sm font-medium text-text-muted mb-2">
          Font Size: {Math.round(getSettings().fontSize * 100)}%
        </label>
        <input
          id="font-size-range"
          type="range"
          min="0.75"
          max="1.5"
          step="0.05"
          value={getSettings().fontSize}
          oninput={handleFontSizeChange}
          class="w-full"
        />
      </div>

      <!-- Compact Spacing -->
      <div class="flex items-center justify-between">
        <div>
          <span class="block text-sm font-medium text-text-muted mb-1">
            Compact Spacing
          </span>
          <p class="text-xs text-text-muted">Tighter UI spacing for more content</p>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input
            id="compact-spacing"
            type="checkbox"
            checked={getSettings().compactSpacing}
            onchange={handleCompactSpacingChange}
            class="sr-only peer"
          />
          <div class="w-11 h-6 bg-border peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-border after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-accent"></div>
        </label>
      </div>

      <!-- Hide Posters -->
      <div class="flex items-center justify-between">
        <div>
          <span class="block text-sm font-medium text-text-muted mb-1">
            Hide Posters
          </span>
          <p class="text-xs text-text-muted">Hide TV and movie posters in the sidebar</p>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input
            id="hide-posters"
            type="checkbox"
            checked={getSettings().hidePosters}
            onchange={handleHidePostersChange}
            class="sr-only peer"
          />
          <div class="w-11 h-6 bg-border peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-border after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-accent"></div>
        </label>
      </div>

      <!-- Colorblind Friendly -->
      <div class="flex items-center justify-between">
        <div>
          <span class="block text-sm font-medium text-text-muted mb-1">
            Colorblind-Friendly Palette
          </span>
          <p class="text-xs text-text-muted">Uses high-contrast, distinguishable colors for episode statuses (upcoming, available, watched, premiere) that are easier to distinguish for users with color vision deficiencies</p>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input
            id="colorblind-friendly"
            type="checkbox"
            checked={getSettings().colorblindFriendly}
            onchange={handleColorblindFriendlyChange}
            class="sr-only peer"
          />
          <div class="w-11 h-6 bg-border peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-border after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-accent"></div>
        </label>
      </div>
    </div>

    <!-- Footer -->
    <div class="p-4 border-t border-border">
      <button
        onclick={() => open = false}
        class="w-full py-2.5 text-text-muted hover:bg-surface-hover rounded-lg transition-colors"
      >
        Close
      </button>
    </div>
  </div>
{/if}

