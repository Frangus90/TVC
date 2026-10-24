<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { Bell, Play, Zap } from "lucide-svelte";
  import {
    getNotificationSettings, updateNotificationSettings, testNotification, loadNotificationSettings,
    isNotificationSettingsLoading, isNotificationSettingsSaving, getNotificationSettingsLoadError,
    getNotificationSettingsSaveError, retryNotificationSettingsSave,
  } from "../../stores/notifications.svelte";
  import { previewSound, getAvailableSounds } from "../../utils/notificationSound";
  import { getNow } from "../../stores/now.svelte";
  import { clockMinutes, isWithinQuietHours } from "../../utils/quietHours";

  const settings = $derived(getNotificationSettings());
  const sounds = getAvailableSounds();
  const isDev = import.meta.env.DEV;
  let volumeInput = $state(50);
  let volumeTimer: ReturnType<typeof setTimeout> | null = null;
  let quietStart = $state("22:00");
  let quietEnd = $state("08:00");
  let quietDraft = $state(false);
  let quietError = $state<string | null>(null);
  let testing = $state(false);
  let testingAll = $state(false);
  let testResult = $state<string | null>(null);
  $effect(() => { const volume = settings?.sound_volume; if (!volumeTimer && volume !== undefined) volumeInput = volume; });
  $effect(() => { const start = settings?.quiet_hours_start, end = settings?.quiet_hours_end; if (!quietDraft && start && end) { quietStart = start; quietEnd = end; } });
  onMount(() => { if (!settings && !isNotificationSettingsLoading()) void loadNotificationSettings(); });
  onDestroy(() => { if (volumeTimer) flushVolume(); });

  function changeVolume(event: Event) {
    volumeInput = parseInt((event.target as HTMLInputElement).value);
    if (volumeTimer) clearTimeout(volumeTimer);
    volumeTimer = setTimeout(flushVolume, 250);
  }
  function flushVolume() {
    if (!volumeTimer) return;
    clearTimeout(volumeTimer); volumeTimer = null;
    void updateNotificationSettings({ sound_volume: volumeInput });
  }
  function toggle(key: 'enabled' | 'sound_enabled' | 'racing_enabled' | 'plex_enabled' | 'system_enabled') {
    if (settings) void updateNotificationSettings({ [key]: !settings[key] });
  }
  function setWindowsDelivery(event: Event) {
    const value = (event.target as HTMLSelectElement).value;
    void updateNotificationSettings({ os_fallback: value === 'always', tray_notifications: value !== 'never' });
  }
  function validQuietRange(): boolean {
    if (clockMinutes(quietStart) === null || clockMinutes(quietEnd) === null) {
      quietError = "Enter both times in 24-hour HH:mm format, for example 22:00."; return false;
    }
    if (quietStart === quietEnd) { quietError = "Choose different start and end times."; return false; }
    quietError = null; return true;
  }
  function saveQuietRange() {
    if (!validQuietRange()) return;
    quietDraft = false;
    void updateNotificationSettings({ quiet_hours_start: quietStart, quiet_hours_end: quietEnd });
  }
  function toggleQuietHours() {
    if (!settings) return;
    if (settings.quiet_hours_enabled) { void updateNotificationSettings({ quiet_hours_enabled: false }); return; }
    if (!validQuietRange()) return;
    quietDraft = false;
    void updateNotificationSettings({ quiet_hours_enabled: true, quiet_hours_start: quietStart, quiet_hours_end: quietEnd });
  }
  async function sendTest() {
    testing = true; testResult = null;
    flushVolume();
    try {
      const sent = await testNotification('system');
      testResult = sent ? 'Test sent. Check the bell for its history; delivery follows your saved preferences and quiet hours.' : 'The test was not sent. Save your settings and try again.';
    } finally { testing = false; }
  }
  async function fireAll() {
    testingAll = true;
    try { for (const type of ['racing', 'plex', 'premiere', 'update', 'system']) { await testNotification(type); await new Promise(resolve => setTimeout(resolve, 600)); } }
    finally { testingAll = false; }
  }
  const categoryOptions: { key: 'racing_enabled' | 'plex_enabled' | 'system_enabled'; label: string; help: string }[] = [
    { key: 'racing_enabled', label: 'Racing', help: 'Race session reminders.' },
    { key: 'plex_enabled', label: 'Plex', help: 'Updates when Plex marks a title as watched.' },
    { key: 'system_enabled', label: 'System', help: 'App messages and test notifications.' },
  ];
  const windowsDelivery = $derived(settings?.os_fallback ? 'always' : settings?.tray_notifications ? 'hidden' : 'never');
</script>

{#if getNotificationSettingsLoadError()}
  <div role="alert" class="p-3 rounded-lg border border-error/30 text-error text-sm mb-4">
    <p>{getNotificationSettingsLoadError()}</p><button type="button" onclick={() => loadNotificationSettings()} class="underline mt-1">Retry loading</button>
  </div>
{:else if !settings || isNotificationSettingsLoading()}
  <p role="status" class="text-sm text-text-muted py-4">Loading notification settings…</p>
{/if}
{#if settings}
  <div class="space-y-4">
    {#if getNotificationSettingsSaveError()}
      <div role="alert" class="p-3 rounded-lg border border-error/30 text-error text-sm">
        <p>{getNotificationSettingsSaveError()}</p><button type="button" disabled={isNotificationSettingsSaving()} onclick={() => retryNotificationSettingsSave()} class="underline mt-1">Retry saving</button>
      </div>
    {/if}

    <div class="settings-group">
      <div class="settings-row">
        <div><p class="font-medium text-sm">Notifications</p><p class="text-xs text-text-muted">Allow app alerts, sounds, and Windows notifications.</p></div>
        <button type="button" role="switch" aria-checked={settings.enabled} aria-label="Notifications" onclick={() => toggle('enabled')}
          class="relative w-10 h-5 shrink-0 rounded-full {settings.enabled ? 'bg-accent' : 'bg-border'}"><span class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform {settings.enabled ? 'translate-x-5' : ''}"></span></button>
      </div>
      <div class="settings-row">
        <div><label for="windows-notification-delivery" class="font-medium text-sm">Windows notifications</label><p class="text-xs text-text-muted">Choose when alerts also appear outside TVC.</p></div>
        <select id="windows-notification-delivery" value={windowsDelivery} onchange={setWindowsDelivery} class="bg-surface border border-border rounded-lg px-3 py-1.5 text-sm">
          <option value="never">Never</option><option value="hidden">When TVC is hidden</option><option value="always">Always</option>
        </select>
      </div>
      <div class="settings-row">
        <div><p class="font-medium text-sm">Notification sound</p></div>
        <div class="settings-controls flex flex-wrap items-center gap-3">
          {#if settings.sound_enabled}
            <select aria-label="Notification sound" value={settings.sound_choice} onchange={(event) => updateNotificationSettings({ sound_choice: event.currentTarget.value })} class="bg-surface border border-border rounded-lg px-2 py-1.5 text-sm">
              {#each sounds as sound}<option value={sound.value}>{sound.label}</option>{/each}
            </select>
            <button type="button" onclick={() => previewSound(settings!.sound_choice, volumeInput)} aria-label="Preview notification sound" title="Preview sound" class="p-2 rounded-lg hover:bg-surface-hover text-text-muted"><Play size={16} /></button>
            <input type="range" min="0" max="100" step="5" value={volumeInput} oninput={changeVolume} onchange={flushVolume} aria-label="Notification volume" class="w-28 accent-accent" />
            <span class="text-xs text-text-muted w-9 text-right">{volumeInput}%</span>
          {/if}
          <button type="button" role="switch" aria-checked={settings.sound_enabled} aria-label="Notification sound" onclick={() => toggle('sound_enabled')}
            class="relative w-10 h-5 shrink-0 rounded-full {settings.sound_enabled ? 'bg-accent' : 'bg-border'}"><span class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform {settings.sound_enabled ? 'translate-x-5' : ''}"></span></button>
        </div>
      </div>
      <div class="settings-row">
        <div><p class="font-medium text-sm">Quiet hours</p><p class="text-xs text-text-muted">Silence popups, sounds, and Windows alerts; keep notification history.</p></div>
        <div class="settings-controls flex flex-wrap items-center gap-2">
          <label for="quiet-hours-start" class="text-xs text-text-muted">From</label>
          <input id="quiet-hours-start" type="text" inputmode="numeric" maxlength="5" pattern="(?:[01][0-9]|2[0-3]):[0-5][0-9]" value={quietStart}
            oninput={(event) => { quietStart = event.currentTarget.value; quietDraft = true; quietError = null; }} onchange={saveQuietRange} onkeydown={(event) => { if (event.key === "Enter") { event.preventDefault(); saveQuietRange(); } }}
            aria-label="Quiet hours start, 24-hour HH:mm" aria-invalid={!!quietError} aria-describedby="quiet-hours-help" class="w-20 bg-surface border border-border rounded-lg px-2 py-1.5 text-sm font-mono" />
          <label for="quiet-hours-end" class="text-xs text-text-muted">To</label>
          <input id="quiet-hours-end" type="text" inputmode="numeric" maxlength="5" pattern="(?:[01][0-9]|2[0-3]):[0-5][0-9]" value={quietEnd}
            oninput={(event) => { quietEnd = event.currentTarget.value; quietDraft = true; quietError = null; }} onchange={saveQuietRange} onkeydown={(event) => { if (event.key === "Enter") { event.preventDefault(); saveQuietRange(); } }}
            aria-label="Quiet hours end, 24-hour HH:mm" aria-invalid={!!quietError} aria-describedby="quiet-hours-help" class="w-20 bg-surface border border-border rounded-lg px-2 py-1.5 text-sm font-mono" />
          <button type="button" role="switch" aria-checked={settings.quiet_hours_enabled} aria-label="Quiet hours" onclick={toggleQuietHours}
            class="relative w-10 h-5 shrink-0 rounded-full ml-1 {settings.quiet_hours_enabled ? 'bg-accent' : 'bg-border'}"><span class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform {settings.quiet_hours_enabled ? 'translate-x-5' : ''}"></span></button>
        </div>
      </div>
    </div>
    <p id="quiet-hours-help" class="text-xs text-text-muted">Quiet hours use your computer's local 24-hour clock and can run overnight.{#if quietError}<span role="alert" class="text-error block mt-1">{quietError}</span>{/if}</p>
    {#if settings.quiet_hours_enabled && isWithinQuietHours(settings, getNow())}<p class="text-xs text-text-muted">Quiet hours are active. New notifications appear in the bell's history without alerts.</p>{/if}

    <div class="settings-group">
      <div class="settings-row">
        <div>
          <label for="notification-auto-clear" class="font-medium text-sm">Automatically clear history</label>
          <p id="notification-auto-clear-help" class="text-xs text-text-muted">Clear notifications older than this, including unread entries. 1 month means 30 days.</p>
          <p class="text-xs text-text-muted">Checked when TVC opens and every minute while it is running. Older entries clear when you save a period.</p>
        </div>
        <select id="notification-auto-clear" value={settings.auto_clear_after_days} aria-describedby="notification-auto-clear-help"
          onchange={(event) => updateNotificationSettings({ auto_clear_after_days: Number(event.currentTarget.value) })}
          class="bg-surface border border-border rounded-lg px-3 py-1.5 text-sm">
          <option value={0}>Off</option>
          <option value={1}>1 day</option>
          <option value={7}>1 week</option>
          <option value={30}>1 month</option>
        </select>
      </div>
    </div>

    <div class="flex flex-wrap items-center justify-between gap-3">
      <span role="status" class="text-xs text-text-muted">{isNotificationSettingsSaving() ? 'Saving notification settings…' : getNotificationSettingsSaveError() ? 'Changes are not saved.' : 'Changes save automatically.'}</span>
      <button type="button" onclick={sendTest} disabled={testing || !settings.enabled || !settings.system_enabled || !!getNotificationSettingsSaveError()}
        class="flex items-center gap-2 px-3 py-1.5 border border-border rounded-lg text-sm hover:bg-surface-hover disabled:opacity-50"><Bell size={15} />{testing ? 'Sending test…' : 'Test notification'}</button>
    </div>
    {#if !settings.enabled}<p class="text-xs text-text-muted">Notifications are off. Your other preferences are kept.</p>
    {:else if !settings.system_enabled}<p class="text-xs text-text-muted">Enable System notifications under Advanced to send a test.</p>{/if}
    {#if testResult}<p role="status" class="text-xs text-text-muted">{testResult}</p>{/if}

    <details class="settings-group">
      <summary class="cursor-pointer font-medium text-sm py-2">Advanced · popup display and categories</summary>
      <div class="settings-row"><label for="notification-popup-position" class="text-sm">Popup position</label><select id="notification-popup-position" value={settings.popup_position} onchange={(event) => updateNotificationSettings({ popup_position: event.currentTarget.value })} class="bg-surface border border-border rounded-lg px-2 py-1.5 text-sm">{#each [['top-right','Top right'],['top-left','Top left'],['bottom-right','Bottom right'],['bottom-left','Bottom left']] as [value,label]}<option {value}>{label}</option>{/each}</select></div>
      <div class="settings-row"><label for="notification-popup-duration" class="text-sm">Popup duration</label><select id="notification-popup-duration" value={settings.popup_duration} onchange={(event) => updateNotificationSettings({ popup_duration: parseInt(event.currentTarget.value) })} class="bg-surface border border-border rounded-lg px-2 py-1.5 text-sm">{#each [[3000,'3 seconds'],[5000,'5 seconds'],[8000,'8 seconds'],[12000,'12 seconds'],[0,'Until dismissed']] as [value,label]}<option {value}>{label}</option>{/each}</select></div>
      <div class="settings-row"><label for="notification-popup-maximum" class="text-sm">Maximum visible popups</label><select id="notification-popup-maximum" value={settings.max_visible} onchange={(event) => updateNotificationSettings({ max_visible: parseInt(event.currentTarget.value) })} class="bg-surface border border-border rounded-lg px-2 py-1.5 text-sm">{#each [1,2,3,4,5] as value}<option {value}>{value}</option>{/each}</select></div>
      {#each categoryOptions as option}
        <div class="settings-row"><div><p class="text-sm font-medium">{option.label}</p><p class="text-xs text-text-muted">{option.help}</p></div>
          <button type="button" role="switch" aria-checked={settings[option.key]} aria-label={`${option.label} notifications`} onclick={() => toggle(option.key)} class="relative w-10 h-5 shrink-0 rounded-full {settings[option.key] ? 'bg-accent' : 'bg-border'}"><span class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform {settings[option.key] ? 'translate-x-5' : ''}"></span></button>
        </div>
      {/each}
    </details>
    {#if isDev}
      <details class="settings-group">
        <summary class="cursor-pointer text-sm text-warning py-2">Development · notification tests</summary>
        <div class="flex flex-wrap gap-2 mt-2">{#each ['racing', 'plex', 'premiere', 'update', 'system'] as type}<button type="button" onclick={() => testNotification(type)} class="px-3 py-1.5 text-sm border border-border rounded-lg hover:bg-surface-hover">{type[0].toUpperCase() + type.slice(1)}</button>{/each}</div>
        <button type="button" onclick={fireAll} disabled={testingAll} class="flex items-center gap-2 px-3 py-1.5 mt-3 text-sm rounded-lg bg-warning/10 text-warning disabled:opacity-50"><Zap size={15} />{testingAll ? 'Testing…' : 'Test all types'}</button>
      </details>
    {/if}
  </div>
{/if}
