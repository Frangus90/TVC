<script lang="ts">
  import { RotateCcw } from "lucide-svelte";
  import {
    getThemeSettings,
    updateThemeSettings,
    type ThemeSettings,
  } from "../../stores/theme.svelte";

  const defaultTheme: ThemeSettings = {
    colorScheme: "default",
    accentColor: "#3b82f6",
    fontSize: 1,
    compactSpacing: false,
    colorblindFriendly: false,
    hidePosters: false,
  };

  const accentPresets = [
    { color: "#3b82f6", name: "Blue" },
    { color: "#8b5cf6", name: "Purple" },
    { color: "#ec4899", name: "Pink" },
    { color: "#ef4444", name: "Red" },
    { color: "#f97316", name: "Orange" },
    { color: "#eab308", name: "Yellow" },
    { color: "#22c55e", name: "Green" },
    { color: "#06b6d4", name: "Cyan" },
  ];

  function settings() {
    return getThemeSettings();
  }

  async function handleAccentColor(color: string) {
    await updateThemeSettings({ accentColor: color });
  }

  async function handleFontSize(event: Event) {
    const input = event.target as HTMLInputElement;
    await updateThemeSettings({ fontSize: parseFloat(input.value) });
  }

  async function handleToggle(key: keyof ThemeSettings) {
    await updateThemeSettings({ [key]: !settings()[key] });
  }

  async function resetToDefaults() {
    await updateThemeSettings({ ...defaultTheme });
  }

  function isDefault(): boolean {
    const s = settings();
    return (
      s.accentColor === defaultTheme.accentColor &&
      s.fontSize === defaultTheme.fontSize &&
      s.compactSpacing === defaultTheme.compactSpacing &&
      s.colorblindFriendly === defaultTheme.colorblindFriendly &&
      s.hidePosters === defaultTheme.hidePosters
    );
  }
</script>

<div class="space-y-6">
  <!-- Theme -->
  <div class="space-y-3">
    <h3 class="text-sm font-semibold text-text-muted uppercase tracking-wider">Theme</h3>

    <!-- Accent Color -->
    <div class="p-3 rounded-lg bg-background border border-border space-y-3">
      <p class="text-sm font-medium text-text">Accent Color</p>
      <div class="flex items-center gap-2 flex-wrap">
        {#each accentPresets as preset}
          <button
            onclick={() => handleAccentColor(preset.color)}
            class="w-7 h-7 rounded-full border-2 transition-all {settings().accentColor === preset.color ? 'border-text scale-110' : 'border-transparent hover:scale-105'}"
            style="background-color: {preset.color}"
            title={preset.name}
          ></button>
        {/each}
      </div>
      <div class="flex items-center gap-3">
        <input
          type="color"
          value={settings().accentColor}
          onchange={(e) => handleAccentColor((e.target as HTMLInputElement).value)}
          class="w-10 h-8 rounded border border-border cursor-pointer bg-transparent"
          style="padding: 0;"
          title="Custom color"
        />
        <input
          type="text"
          value={settings().accentColor}
          onchange={(e) => handleAccentColor((e.target as HTMLInputElement).value)}
          class="w-28 px-3 py-1.5 rounded border border-border bg-surface text-text text-sm font-mono"
          placeholder="#3b82f6"
        />
      </div>
    </div>
  </div>

  <!-- Layout -->
  <div class="space-y-3">
    <h3 class="text-sm font-semibold text-text-muted uppercase tracking-wider">Layout</h3>

    <!-- Font Size -->
    <div class="p-3 rounded-lg bg-background border border-border space-y-2">
      <div class="flex items-center justify-between">
        <p class="text-sm font-medium text-text">Font Size</p>
        <span class="text-sm text-text-muted">{Math.round(settings().fontSize * 100)}%</span>
      </div>
      <div class="flex items-center gap-3">
        <span class="text-xs text-text-muted">75%</span>
        <input
          type="range"
          min="0.75"
          max="1.5"
          step="0.05"
          value={settings().fontSize}
          oninput={handleFontSize}
          class="flex-1"
        />
        <span class="text-xs text-text-muted">150%</span>
      </div>
    </div>

    <!-- Compact Spacing -->
    <div class="flex items-center justify-between p-3 rounded-lg bg-background border border-border">
      <div>
        <p class="text-sm font-medium text-text">Compact Spacing</p>
        <p class="text-xs text-text-muted">Tighter UI spacing for more content</p>
      </div>
      <button
        onclick={() => handleToggle('compactSpacing')}
        class="relative w-10 h-5 rounded-full transition-colors {settings().compactSpacing ? 'bg-accent' : 'bg-border'}"
        aria-label="Toggle compact spacing"
      >
        <span class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform {settings().compactSpacing ? 'translate-x-5' : ''}"></span>
      </button>
    </div>

    <!-- Hide Posters -->
    <div class="flex items-center justify-between p-3 rounded-lg bg-background border border-border">
      <div>
        <p class="text-sm font-medium text-text">Hide Posters</p>
        <p class="text-xs text-text-muted">Hide TV and movie posters in the sidebar</p>
      </div>
      <button
        onclick={() => handleToggle('hidePosters')}
        class="relative w-10 h-5 rounded-full transition-colors {settings().hidePosters ? 'bg-accent' : 'bg-border'}"
        aria-label="Toggle hide posters"
      >
        <span class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform {settings().hidePosters ? 'translate-x-5' : ''}"></span>
      </button>
    </div>
  </div>

  <!-- Accessibility -->
  <div class="space-y-3">
    <h3 class="text-sm font-semibold text-text-muted uppercase tracking-wider">Accessibility</h3>

    <div class="flex items-center justify-between p-3 rounded-lg bg-background border border-border">
      <div class="pr-4">
        <p class="text-sm font-medium text-text">Colorblind-Friendly Palette</p>
        <p class="text-xs text-text-muted">High-contrast colors for episode statuses that are easier to distinguish with color vision deficiencies</p>
      </div>
      <button
        onclick={() => handleToggle('colorblindFriendly')}
        class="relative w-10 h-5 rounded-full transition-colors flex-shrink-0 {settings().colorblindFriendly ? 'bg-accent' : 'bg-border'}"
        aria-label="Toggle colorblind-friendly palette"
      >
        <span class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform {settings().colorblindFriendly ? 'translate-x-5' : ''}"></span>
      </button>
    </div>
  </div>

  <!-- Reset -->
  {#if !isDefault()}
    <button
      onclick={resetToDefaults}
      class="flex items-center gap-2 px-4 py-2 rounded-lg bg-surface-hover/50 text-text-muted hover:text-text hover:bg-surface-hover transition-colors text-sm"
    >
      <RotateCcw class="w-4 h-4" />
      Reset to Defaults
    </button>
  {/if}
</div>
