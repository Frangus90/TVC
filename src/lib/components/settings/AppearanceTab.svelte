<script lang="ts">
  import { onDestroy } from "svelte";
  import { RotateCcw } from "lucide-svelte";
  import {
    getThemeSettings,
    updateThemeSettings,
    type ThemeSettings,
    defaultTheme,
    normalizeAccentColor,
    isThemeSaving,
    getThemeSaveError,
    getThemeLoadError,
    loadThemeSettings,
    retryThemeSave,
  } from "../../stores/theme.svelte";

  let colourError = $state<string | null>(null);
  let fontInput = $state(1);
  let fontTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => { if (!fontTimer) fontInput = getThemeSettings().fontSize; });
  onDestroy(() => { if (fontTimer) { clearTimeout(fontTimer); fontTimer = null; void updateThemeSettings({ fontSize: fontInput }); } });

  const accentPresets = [
    { color: "#bba4ff", name: "Focus" },
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
    const normalized = normalizeAccentColor(color);
    if (!normalized) { colourError = "Use #RGB or #RRGGBB, for example #bba4ff."; return; }
    colourError = null;
    await updateThemeSettings({ accentColor: normalized });
  }

  function handleFontSize(event: Event) {
    fontInput = parseFloat((event.target as HTMLInputElement).value);
    if (fontTimer) clearTimeout(fontTimer);
    fontTimer = setTimeout(flushFontSize, 250);
  }
  function flushFontSize() {
    if (!fontTimer) return;
    clearTimeout(fontTimer);
    fontTimer = null;
    void updateThemeSettings({ fontSize: fontInput });
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
  {#if getThemeLoadError()}
    <div role="alert" class="p-3 rounded-lg border border-red-500/30 text-red-400 text-sm">
      <p>{getThemeLoadError()}</p><button type="button" onclick={() => loadThemeSettings()} class="underline mt-1">Retry loading</button>
    </div>
  {/if}
  {#if getThemeSaveError()}
    <div role="alert" class="p-3 rounded-lg border border-red-500/30 text-red-400 text-sm">
      <p>{getThemeSaveError()}</p><button type="button" disabled={isThemeSaving()} onclick={() => retryThemeSave()} class="underline mt-1">Retry saving</button>
    </div>
  {:else if isThemeSaving()}<p role="status" class="text-xs text-text-muted">Saving appearance…</p>{/if}
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
            aria-label={`${preset.name} accent colour`}
            aria-pressed={settings().accentColor === preset.color}
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
          aria-label="Choose a custom accent colour"
          title="Custom color"
        />
        <input
          aria-label="Custom accent colour hexadecimal value"
          aria-invalid={!!colourError}
          aria-describedby={colourError ? "accent-colour-error" : undefined}
          type="text"
          value={settings().accentColor}
          onchange={(e) => handleAccentColor((e.target as HTMLInputElement).value)}
          class="w-28 px-3 py-1.5 rounded border border-border bg-surface text-text text-sm font-mono"
          placeholder="#3b82f6"
        />
      </div>
      {#if colourError}<p id="accent-colour-error" role="alert" class="text-sm text-red-400">{colourError}</p>{/if}
    </div>
  </div>

  <!-- Layout -->
  <div class="space-y-3">
    <h3 class="text-sm font-semibold text-text-muted uppercase tracking-wider">Layout</h3>

    <!-- Font Size -->
    <div class="p-3 rounded-lg bg-background border border-border space-y-2">
      <div class="flex items-center justify-between">
        <p class="text-sm font-medium text-text">Font Size</p>
        <span class="text-sm text-text-muted">{Math.round(fontInput * 100)}%</span>
      </div>
      <div class="flex items-center gap-3">
        <span class="text-xs text-text-muted">75%</span>
        <input
          type="range"
          min="0.75"
          max="1.5"
          step="0.05"
          value={fontInput}
          aria-label="Font size"
          oninput={handleFontSize}
          onchange={flushFontSize}
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
        aria-pressed={settings().compactSpacing}
      >
        <span class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform {settings().compactSpacing ? 'translate-x-5' : ''}"></span>
      </button>
    </div>

    <!-- Hide Posters -->
    <div class="flex items-center justify-between p-3 rounded-lg bg-background border border-border">
      <div>
        <p class="text-sm font-medium text-text">Hide Posters</p>
        <p class="text-xs text-text-muted">Hide posters in library and calendar lists and title pickers</p>
      </div>
      <button
        onclick={() => handleToggle('hidePosters')}
        class="relative w-10 h-5 rounded-full transition-colors {settings().hidePosters ? 'bg-accent' : 'bg-border'}"
        aria-label="Toggle hide posters"
        aria-pressed={settings().hidePosters}
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
        role="switch"
        aria-checked={settings().colorblindFriendly}
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
