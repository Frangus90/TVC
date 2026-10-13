<script lang="ts">
  import { onMount } from "svelte";
  import {
    getGeneralPreferences,
    getWindowsStartup,
    loadGeneralPreferences,
    loadWindowsStartup,
    setWindowsStartup,
    updateGeneralPreferences,
    type GeneralPreferences,
  } from "../../stores/general.svelte";

  let loading = $state(true);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let saved = $state(false);
  const countries = [
    ["NO", "Norway"], ["US", "United States"], ["GB", "United Kingdom"],
    ["SE", "Sweden"], ["DK", "Denmark"], ["FI", "Finland"], ["DE", "Germany"],
    ["FR", "France"], ["NL", "Netherlands"], ["ES", "Spain"], ["IT", "Italy"],
    ["CA", "Canada"], ["AU", "Australia"],
  ];

  async function load() {
    loading = true;
    error = null;
    try {
      await Promise.all([loadGeneralPreferences(), loadWindowsStartup()]);
    } catch (cause) {
      error = `Could not load general settings: ${String(cause)}`;
    } finally {
      loading = false;
    }
  }

  async function save(updates: Partial<GeneralPreferences>) {
    saving = true;
    saved = false;
    error = null;
    try {
      await updateGeneralPreferences(updates);
      saved = true;
    } catch (cause) {
      error = `Could not save general settings: ${String(cause)}`;
    } finally {
      saving = false;
    }
  }

  async function toggleWindowsStartup() {
    saving = true;
    saved = false;
    error = null;
    try {
      await setWindowsStartup(!getWindowsStartup().enabled);
      saved = true;
    } catch (cause) {
      error = `Could not change Windows startup: ${String(cause)}`;
    } finally {
      saving = false;
    }
  }

  async function saveSelection(event: Event, key: "startup_page" | "close_action" | "release_country") {
    const select = event.currentTarget as HTMLSelectElement;
    await save({ [key]: select.value } as Partial<GeneralPreferences>);
    // Restore the confirmed choice when saving fails, including the native select.
    select.value = getGeneralPreferences()[key];
  }

  onMount(() => { void load(); });
</script>

<div class="general-settings">
  {#if error}
    <div class="settings-message text-error" role="alert">
      <span>{error}</span>
      <button type="button" onclick={load} disabled={saving} class="focus-button">Retry</button>
    </div>
  {/if}
  {#if loading}
    <p class="text-sm text-text-muted" role="status">Loading settings…</p>
  {:else}
    <section aria-labelledby="general-opening-title">
      <h3 id="general-opening-title">Opening and closing TVC</h3>
      <div class="setting-row">
        <div>
          <label for="startup-page">Startup page</label>
          <p>The page you see when you open TVC.</p>
        </div>
        <select id="startup-page" value={getGeneralPreferences().startup_page} disabled={saving}
          onchange={(event) => saveSelection(event, "startup_page")}>
          <option value="calendar">Calendar</option>
          <option value="movies">Movies</option>
          <option value="shows">TV shows</option>
          <option value="last_used">Last used page</option>
        </select>
      </div>
      <div class="setting-row">
        <div>
          <span class="setting-label" id="remember-calendar-label">Remember calendar view and filters</span>
          <p>Keep Month, Week or Agenda and your choice of TV shows or movies.</p>
        </div>
        <button type="button" class="setting-toggle" class:enabled={getGeneralPreferences().remember_calendar}
          role="switch" aria-checked={getGeneralPreferences().remember_calendar} aria-labelledby="remember-calendar-label"
          disabled={saving} onclick={() => save({ remember_calendar: !getGeneralPreferences().remember_calendar })}>
          <span></span>
        </button>
      </div>
      <div class="setting-row">
        <div>
          <label for="close-action">When I close the window</label>
          <p>Keeping TVC in the tray lets sync and reminders continue.</p>
        </div>
        <select id="close-action" value={getGeneralPreferences().close_action} disabled={saving}
          onchange={(event) => saveSelection(event, "close_action")}>
          <option value="tray">Keep running in the tray</option>
          <option value="quit">Quit TVC</option>
        </select>
      </div>
      <div class="setting-row">
        <div>
          <span class="setting-label" id="windows-startup-label">Start with Windows</span>
          <p>{getWindowsStartup().supported ? "Open TVC automatically when you sign in." : "Available in installed releases."}</p>
        </div>
        <button type="button" class="setting-toggle" class:enabled={getWindowsStartup().enabled}
          role="switch" aria-checked={getWindowsStartup().enabled} aria-labelledby="windows-startup-label"
          disabled={saving || !getWindowsStartup().supported} onclick={toggleWindowsStartup}>
          <span></span>
        </button>
      </div>
      <div class="setting-row">
        <div>
          <span class="setting-label" id="start-tray-label">Start in tray with Windows</span>
          <p>A normal launch still opens the window.</p>
        </div>
        <button type="button" class="setting-toggle" class:enabled={getGeneralPreferences().start_minimized}
          role="switch" aria-checked={getGeneralPreferences().start_minimized} aria-labelledby="start-tray-label"
          disabled={saving} onclick={() => save({ start_minimized: !getGeneralPreferences().start_minimized })}>
          <span></span>
        </button>
      </div>
    </section>
    <section aria-labelledby="general-releases-title">
      <h3 id="general-releases-title">Movie releases</h3>
      <div class="setting-row">
        <div>
          <label for="release-country">Preferred release country</label>
          <p>Digital and physical dates use this country.</p>
        </div>
        <select id="release-country" value={getGeneralPreferences().release_country} disabled={saving}
          onchange={(event) => saveSelection(event, "release_country")}>
          {#each countries as [code, name]}
            <option value={code}>{name}</option>
          {/each}
        </select>
      </div>
      <p class="settings-note">Original theatrical dates and years are kept. Saved dates update on the next TMDB sync.</p>
    </section>
    <p class="save-status text-sm text-text-muted" role="status">{saving ? "Saving…" : saved ? "Saved" : "Changes save automatically."}</p>
  {/if}
</div>

<style>
  .general-settings { max-width: 940px; }
  section + section { margin-top: 1.75rem; }
  h3 { margin: 0 0 .4rem; font-size: 1rem; font-weight: 600; }
  .setting-row { display: flex; align-items: center; justify-content: space-between; gap: 1.5rem; padding: 1rem 0; border-bottom: 1px solid var(--color-border); }
  .setting-row > div { min-width: 0; }
  label, .setting-label { font-size: .875rem; font-weight: 500; color: var(--color-text); }
  .setting-row p, .settings-note { margin: .3rem 0 0; font-size: .8125rem; line-height: 1.5; color: var(--color-text-muted); }
  select { flex-shrink: 0; max-width: 100%; width: 13rem; padding: .45rem .65rem; border: 1px solid var(--color-border); border-radius: .5rem; background: var(--color-background); color: var(--color-text); font-size: .875rem; }
  .setting-toggle { position: relative; flex-shrink: 0; width: 2.5rem; height: 1.25rem; border-radius: 999px; background: var(--color-border); transition: background .15s; }
  .setting-toggle.enabled { background: var(--accent-color); }
  .setting-toggle span { position: absolute; top: .125rem; left: .125rem; width: 1rem; height: 1rem; border-radius: 50%; background: white; transition: transform .15s; }
  .setting-toggle.enabled span { transform: translateX(1.25rem); }
  button:disabled, select:disabled { opacity: .5; cursor: default; }
  .settings-message { display: flex; align-items: center; justify-content: space-between; gap: 1rem; margin-bottom: 1rem; font-size: .875rem; }
  .save-status { min-height: 1.5rem; margin: 1rem 0 0; }
  @media (max-width: 600px) { .setting-row { gap: 1rem; } .setting-row:has(select) { flex-wrap: wrap; } select { width: 100%; } }
  @media (prefers-reduced-motion: reduce) { .setting-toggle, .setting-toggle span { transition: none; } }
</style>
