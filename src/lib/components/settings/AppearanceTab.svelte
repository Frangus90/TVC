<script lang="ts">
  import { onDestroy } from "svelte";
  import { RotateCcw, Check, Clock, CircleX, TriangleAlert } from "lucide-svelte";
  import WatchedBadge from "../common/WatchedBadge.svelte";
  import {
    getThemeSettings, updateThemeSettings, defaultTheme, normalizeAccentColor,
    isThemeSaving, getThemeSaveError, getThemeLoadError, loadThemeSettings, retryThemeSave,
    type PosterSize,
  } from "../../stores/theme.svelte";

  let colourError = $state<string | null>(null);
  let fontInput = $state(1);
  let fontTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => { const font = getThemeSettings().fontSize; if (!fontTimer) fontInput = font; });
  onDestroy(() => { if (fontTimer) flushFontSize(); });

  const accentPresets = [
    { color: "#bba4ff", name: "Focus" }, { color: "#3b82f6", name: "Blue" },
    { color: "#8b5cf6", name: "Purple" }, { color: "#ec4899", name: "Pink" },
    { color: "#ef4444", name: "Red" }, { color: "#f97316", name: "Orange" },
    { color: "#eab308", name: "Yellow" }, { color: "#22c55e", name: "Green" },
    { color: "#06b6d4", name: "Cyan" },
  ];
  const settings = $derived(getThemeSettings());

  async function setAccent(color: string) {
    const normalized = normalizeAccentColor(color);
    if (!normalized) { colourError = "Use #RGB or #RRGGBB, for example #bba4ff."; return; }
    colourError = null;
    await updateThemeSettings({ accentColor: normalized });
  }
  function changeFont(event: Event) {
    fontInput = parseFloat((event.target as HTMLInputElement).value);
    if (fontTimer) clearTimeout(fontTimer);
    fontTimer = setTimeout(flushFontSize, 250);
  }
  function flushFontSize() {
    if (!fontTimer) return;
    clearTimeout(fontTimer); fontTimer = null;
    void updateThemeSettings({ fontSize: fontInput });
  }
  function resetDefaults() {
    if (fontTimer) { clearTimeout(fontTimer); fontTimer = null; }
    fontInput = defaultTheme.fontSize; colourError = null;
    void updateThemeSettings({ ...defaultTheme });
  }
  const isDefault = $derived(
    settings.accentColor === defaultTheme.accentColor && settings.fontSize === defaultTheme.fontSize &&
    settings.compactSpacing === defaultTheme.compactSpacing && settings.hidePosters === defaultTheme.hidePosters &&
    settings.colorblindFriendly === defaultTheme.colorblindFriendly && settings.posterSize === defaultTheme.posterSize,
  );
</script>

<div class="space-y-4">
  {#if getThemeLoadError()}
    <div role="alert" class="p-3 rounded-lg border border-error/30 text-error text-sm">
      <p>{getThemeLoadError()}</p><button type="button" onclick={() => loadThemeSettings()} class="underline mt-1">Retry loading</button>
    </div>
  {/if}
  {#if getThemeSaveError()}
    <div role="alert" class="p-3 rounded-lg border border-error/30 text-error text-sm">
      <p>{getThemeSaveError()}</p><button type="button" disabled={isThemeSaving()} onclick={() => retryThemeSave()} class="underline mt-1">Retry saving</button>
    </div>
  {/if}

  <div class="settings-group">
    <div class="settings-row">
      <div><p class="font-medium text-sm">Accent colour</p></div>
      <div class="settings-controls flex flex-wrap items-center gap-2">
        {#each accentPresets as preset}
          <button type="button" onclick={() => setAccent(preset.color)}
            class="w-7 h-7 rounded-full border-2 transition-all {settings.accentColor === preset.color ? 'border-text scale-110' : 'border-transparent hover:scale-105'}"
            style:background-color={preset.color} title={preset.name} aria-label={`${preset.name} accent colour`} aria-pressed={settings.accentColor === preset.color}></button>
        {/each}
        <details class="custom-colour">
          <summary class="cursor-pointer text-sm text-text-muted hover:text-text">Custom colour</summary>
          <div class="flex flex-wrap items-center gap-2 mt-2">
            <input type="color" value={settings.accentColor} onchange={(event) => setAccent(event.currentTarget.value)} aria-label="Choose a custom accent colour" class="w-9 h-8 rounded border border-border bg-transparent" />
            <input type="text" value={settings.accentColor} onchange={(event) => setAccent(event.currentTarget.value)} aria-label="Custom accent colour hexadecimal value"
              aria-invalid={!!colourError} aria-describedby={colourError ? 'accent-colour-error' : undefined}
              class="w-28 px-2 py-1 rounded border border-border bg-surface text-text text-sm font-mono" placeholder="#bba4ff" />
          </div>
          {#if colourError}<p id="accent-colour-error" role="alert" class="text-sm text-error mt-1">{colourError}</p>{/if}
        </details>
      </div>
    </div>

    <div class="settings-row">
      <label for="appearance-font-size" class="font-medium text-sm">Font size</label>
      <div class="settings-controls flex items-center gap-3">
        <input id="appearance-font-size" type="range" min="0.75" max="1.5" step="0.05" value={fontInput} oninput={changeFont} onchange={flushFontSize} class="w-44 accent-accent" />
        <span class="text-sm text-text-muted w-12 text-right">{Math.round(fontInput * 100)}%</span>
      </div>
    </div>

    <div class="settings-row">
      <div><p class="font-medium text-sm">Poster size</p><p class="text-xs text-text-muted">Affects libraries and tier lists independently of font size.</p></div>
      <div class="settings-controls flex flex-wrap items-center gap-1" role="group" aria-label="Poster size">
        {#each ['small', 'medium', 'large'] as size}
          <button type="button" onclick={() => updateThemeSettings({ posterSize: size as PosterSize })} aria-pressed={settings.posterSize === size}
            class="px-3 py-1.5 rounded-lg border text-sm {settings.posterSize === size ? 'border-accent bg-accent/10 text-accent' : 'border-border text-text-muted hover:bg-surface-hover'}">{size[0].toUpperCase() + size.slice(1)}</button>
        {/each}
      </div>
    </div>

    {#each [
      { key: 'compactSpacing' as const, label: 'Compact spacing', help: 'Fit more content into the same space.' },
      { key: 'hidePosters' as const, label: 'Hide posters', help: 'Use text lists in libraries, calendars, and title pickers.' },
      { key: 'colorblindFriendly' as const, label: 'Colourblind-friendly colours', help: 'Use blue and orange for results across TVC. Labels and symbols help with red–green colour blindness.' },
    ] as option}
      <div class="settings-row">
        <div>
          <p class="font-medium text-sm">{option.label}</p><p class="text-xs text-text-muted">{option.help}</p>
          {#if option.key === 'colorblindFriendly'}
            <div class="appearance-status-preview" role="group" aria-label="Status colour preview">
              <WatchedBadge />
              <span class="text-upcoming bg-upcoming/15"><Clock size={13} aria-hidden="true" />Upcoming</span>
              <span class="text-success bg-success/15"><Check size={13} aria-hidden="true" />Correct</span>
              <span class="text-error bg-error/15"><CircleX size={13} aria-hidden="true" />Incorrect</span>
              <span class="text-warning bg-warning/15"><TriangleAlert size={13} aria-hidden="true" />Needs attention</span>
            </div>
          {/if}
        </div>
        <button type="button" role="switch" aria-checked={settings[option.key]} aria-label={option.label}
          onclick={() => updateThemeSettings({ [option.key]: !settings[option.key] })}
          class="appearance-switch relative w-10 h-5 shrink-0 rounded-full transition-colors {settings[option.key] ? 'bg-accent' : 'bg-border'}">
          <span style:background-color={settings[option.key] ? 'var(--accent-foreground)' : 'var(--color-text)'} class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full transition-transform {settings[option.key] ? 'translate-x-5' : ''}"></span>
        </button>
      </div>
    {/each}
  </div>

  <div class="flex flex-wrap items-center justify-between gap-3">
    <span role="status" class="text-xs text-text-muted">{isThemeSaving() ? 'Saving appearance…' : getThemeSaveError() ? 'Changes are not saved.' : 'Changes save automatically.'}</span>
    {#if !isDefault}<button type="button" onclick={resetDefaults} class="flex items-center gap-2 text-sm text-text-muted hover:text-text"><RotateCcw size={15} />Reset appearance</button>{/if}
  </div>
</div>

<style>
  .appearance-status-preview { display:flex; flex-wrap:wrap; align-items:center; gap:.35rem; margin-top:.55rem; }
  .appearance-status-preview > span { display:inline-flex; align-items:center; gap:.25rem; padding:.2rem .4rem; border-radius:.35rem; font-size:.7rem; white-space:nowrap; }
  .appearance-switch[aria-checked="true"] { outline:1px solid var(--accent-focus); }
</style>
